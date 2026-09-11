#!/usr/bin/env python3
"""Real TCP acceptance, native or through the Compose HTTPS proxy.

Creates only disposable test workspaces. Credentials stay in private temporary
files or captured subprocess output and are never printed.
"""
import json
import os
import pathlib
import socket
import ssl
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid

compose = sys.argv[1:] == ["--compose"]
root = pathlib.Path(__file__).resolve().parents[2]
cmd = ["docker", "compose", "-f", "server/compose.yml", "-f", "server/tests/compose.yml"]
env = os.environ.copy()
env.pop("NOTES_SYNC_CA_FILE", None)
env.update(NOTES_DOMAIN="localhost", NOTES_HTTP_PORT="8080", NOTES_HTTPS_PORT="8443")


def run(args):
    return subprocess.check_output(args, cwd=root, env=env, stderr=subprocess.PIPE).decode().strip()


with tempfile.TemporaryDirectory() as temp:
    temp = pathlib.Path(temp)
    proc = None
    if compose:
        def cli(*args):
            return run(cmd + ["exec", "-T", "notes-server", "notes-server", *args])
        cli("workspace", "create", "smoke")
        cli("token", "create", "smoke", "smoke", ".", "read,create,update,move,delete,search", "/tmp/smoke.secret")
        token = run(cmd + ["exec", "-T", "notes-server", "cat", "/tmp/smoke.secret"])
        ca = run(cmd + ["exec", "-T", "caddy", "cat", "/data/caddy/pki/authorities/local/root.crt"])
        (temp / "ca.crt").write_text(ca)
        context = ssl.create_default_context(cafile=str(temp / "ca.crt"))
        base = "https://localhost:8443"
    else:
        binary = str(root / "target/debug/notes-server")
        env["NOTES_SERVER_DATA"] = str(temp / "data")
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        env["NOTES_SERVER_BIND"] = f"127.0.0.1:{port}"
        def cli(*args):
            return run([binary, *args])
        cli("workspace", "create", "smoke")
        cli("token", "create", "smoke", "smoke", ".", "read,create,update,move,delete,search", str(temp / "smoke.secret"))
        token = (temp / "smoke.secret").read_text()
        proc = subprocess.Popen([binary, "serve"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        context = None
        base = f"http://127.0.0.1:{port}"

    def request(method, path, value=None, headers=None, auth=True):
        h = {"Content-Type": "application/json"}
        if auth:
            h["Authorization"] = "Bearer " + token
        h.update(headers or {})
        req = urllib.request.Request(base + path, data=None if value is None else json.dumps(value).encode(), headers=h, method=method)
        try:
            response = urllib.request.urlopen(req, context=context, timeout=10)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            return response.status, response.headers, json.load(response)

    try:
        for attempt in range(100):
            try:
                if request("GET", "/healthz", auth=False)[0] == 200:
                    break
            except (urllib.error.URLError, ConnectionError):
                pass
            time.sleep(0.1)
        else:
            raise RuntimeError("server did not become healthy")
        if not compose:
            # Eight incomplete bodies occupy admission slots; the ninth request
            # is refused rather than queued or allocated another large buffer.
            held = []
            try:
                for _ in range(8):
                    stream = socket.create_connection(("127.0.0.1", port), timeout=5)
                    stream.sendall(("POST /v1/workspaces/smoke/notes HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1000\r\n\r\n").encode())
                    held.append(stream)
                time.sleep(0.2)
                assert request("GET", "/v1/workspaces")[0] == 503
            finally:
                for stream in held:
                    stream.close()
            time.sleep(0.1)
            denied = subprocess.run([binary, "backup", str(temp / "live.tar.gz")], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            assert denied.returncode != 0 and not (temp / "live.tar.gz").exists()
        assert request("GET", "/v1/workspaces", auth=False)[0] == 401
        collection = "/v1/workspaces/smoke/notes"
        note = collection + "/smoke.md"
        status, headers, _ = request("POST", collection, {"path":"smoke.md", "text":"smoke\r\n"}, {"If-None-Match":"*"})
        assert status == 201
        first = headers["ETag"]
        assert request("PUT", note, {"text":"changed\n"}, {"If-Match":first})[0] == 200
        assert request("PUT", note, {"text":"stale"}, {"If-Match":first})[0] == 412
        read = request("GET", note)
        assert read[2]["text"] == "changed\n"
        assert request("PATCH", note, {"text":"once\n"}, {"If-Match":read[1]["ETag"]})[0] == 200
        assert request("PATCH", note, {"text":"once\n"}, {"If-Match":read[1]["ETag"]})[0] == 200
        assert request("GET", note)[2]["text"] == "changed\nonce\n"
        assert request("GET", "/unknown")[0] == 404
        assert request("GET", "/v1/openapi.json")[2]["openapi"] == "3.1.0"
        sync = "/v1/workspaces/smoke/sync/revisions"
        status, _, page = request("GET", sync)
        assert status == 200 and page["revisions"] == []
        revision = str(uuid.uuid4())
        publication = {"workspace": page["workspace"], "expected": None,
                       "revision": {"id": revision, "note": str(uuid.uuid4()), "parents": [],
                                    "device": str(uuid.uuid4()), "path": "empty.md",
                                    "content": "b3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"},
                       "content_base64": ""}
        for _ in range(2):
            status, _, receipt = request("POST", sync, publication)
            assert status == 200 and receipt == {"revision": revision, "stored": True, "applied": False}
        assert request("GET", sync + "/" + revision)[2] == publication
        status, _, page = request("GET", sync)
        assert status == 200 and len(page["revisions"]) == 1 and page["next_cursor"] == 1
        assert request("GET", collection + "/empty.md")[0] == 404
        # Two real client processes transfer byte-identical content through the
        # same authenticated transport. Neither process applies workspace writes.
        client = str(root / "target/debug/notes-sync-client")
        cli("workspace", "create", "client")
        remote_token_path = "/tmp/client.secret" if compose else str(temp / "client.secret")
        cli("token", "create", "client", "client", ".", "read,create,update,move,delete", remote_token_path)
        client_token = run(cmd + ["exec", "-T", "notes-server", "cat", remote_token_path]) if compose else pathlib.Path(remote_token_path).read_text()
        client_secret = temp / "client-transport.secret"
        client_secret.write_text(client_token)
        client_secret.chmod(0o600)
        source = temp / "client-source"
        source.mkdir()
        original = b"\xef\xbb\xbfsource\r\n\xff"
        (source / "original.md").write_bytes(original)
        target = temp / "client-target"
        target.mkdir()
        sender, receiver = temp / "sender-state", temp / "receiver-state"
        if compose:
            # A test CA is explicitly trusted; TLS verification stays enabled.
            untrusted = subprocess.run([client, "init-upload", str(sender), str(source), base, "client", str(client_secret), "--allow-private"], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            assert untrusted.returncode != 0 and not sender.exists()
            env["NOTES_SYNC_CA_FILE"] = str(temp / "ca.crt")
        run([client, "init-upload", str(sender), str(source), base, "client", str(client_secret), "--allow-private"])
        run([client, "stage", str(sender)])
        if not compose:
            proc.terminate()
            proc.wait(timeout=15)
            offline = subprocess.run([client, "transfer", str(sender), str(client_secret)], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            assert offline.returncode != 0
            assert json.loads(run([client, "status", str(sender)]))["pending"] == 1
            proc = subprocess.Popen([binary, "serve"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            for _ in range(100):
                try:
                    if request("GET", "/healthz", auth=False)[0] == 200:
                        break
                except (urllib.error.URLError, ConnectionError):
                    pass
                time.sleep(0.1)
            else:
                raise RuntimeError("server did not restart")
        run([client, "transfer", str(sender), str(client_secret)])
        run([client, "init-receive", str(receiver), str(target), base, "client", str(client_secret), "--allow-private"])
        run([client, "transfer", str(receiver), str(client_secret)])
        received = json.loads(run([client, "received", str(receiver)]))
        assert len(received) == 1
        run([client, "export", str(receiver), received[0]["id"]])
        assert (receiver / ("received-" + received[0]["id"] + ".md")).read_bytes() == original
        assert list(target.iterdir()) == []
        assert (source / "original.md").read_bytes() == original
        run([client, "acknowledge", str(receiver), str(client_secret)])
        assert json.loads(run([client, "status", str(receiver)]))["acknowledged_revisions"] == 0
        app_data = temp / "receiver-app-data"
        run([client, "apply", str(receiver), str(app_data)])
        assert (target / "original.md").read_bytes() == original
        assert json.loads(run([client, "status", str(receiver)]))["applied_revisions"] == 1
        run([client, "acknowledge", str(receiver), str(client_secret)])
        run([client, "acknowledge", str(receiver), str(client_secret)])
        assert json.loads(run([client, "status", str(receiver)]))["acknowledged_revisions"] == 1
        (source / "original.md").write_bytes(b"remote update\r\n")
        run([client, "stage", str(sender)])
        run([client, "transfer", str(sender), str(client_secret)])
        run([client, "transfer", str(receiver), str(client_secret)])
        (target / "original.md").write_bytes(b"local work")
        blocked = subprocess.run([client, "apply", str(receiver), str(app_data)], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        assert blocked.returncode != 0
        assert (target / "original.md").read_bytes() == b"local work"
        assert json.loads(run([client, "status", str(receiver)]))["applied_revisions"] == 1
        # A second device publishes an independent successor while the uploader
        # is offline. Its bytes can equal its parent; causal divergence still
        # requires an explicit two-parent resolution.
        parent = json.loads((sender / "client.json").read_text())["received"][-1]
        other = json.loads(json.dumps(parent))
        other["expected"] = parent["revision"]["id"]
        other["revision"]["parents"] = [other["expected"]]
        other["revision"]["id"] = str(uuid.uuid4())
        other["revision"]["device"] = str(uuid.uuid4())
        (source / "original.md").write_bytes(b"offline divergent edit")
        run([client, "stage", str(sender)])
        client_sync = "/v1/workspaces/client/sync/revisions"
        assert request("POST", client_sync, other, {"Authorization": "Bearer " + client_token})[0] == 200
        refused = subprocess.run([client, "transfer", str(sender), str(client_secret)], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        assert refused.returncode != 0
        run([client, "fetch", str(sender), str(client_secret)])
        conflict = json.loads(run([client, "conflicts", str(sender)]))[0]
        assert conflict["action"] == "conflict"
        chosen = temp / "resolution.md"
        chosen.write_bytes(b"chosen result\r\n")
        run([client, "resolve", str(sender), conflict["local"], conflict["remote"], str(chosen)])
        assert (source / "original.md").read_bytes() == b"offline divergent edit"
        run([client, "transfer", str(sender), str(client_secret)])
        assert json.loads(run([client, "conflicts", str(sender)])) == []
        merged = json.loads((sender / "client.json").read_text())["received"][-1]
        assert set(merged["revision"]["parents"]) == {conflict["local"], conflict["remote"]}
        assert merged["branches"][0]["revision"]["id"] == conflict["local"]
        fresh_root = temp / "resolved-source"
        fresh_root.mkdir()
        fresh_state = temp / "resolved-state"
        run([client, "init-receive", str(fresh_state), str(fresh_root), base, "client", str(client_secret), "--allow-private"])
        run([client, "fetch", str(fresh_state), str(client_secret)])
        run([client, "apply", str(fresh_state), str(temp / "resolved-app-data")])
        assert (fresh_root / "original.md").read_bytes() == chosen.read_bytes()
        # The expanded suite exceeds the real 60-request credential budget.
        # Wait for its window rather than weakening production rate limits.
        time.sleep(61)
        # Exercise explicit path and tombstone choices using actual CLI processes.
        for remote_deleted in (False, True):
            parent = json.loads((sender / "client.json").read_text())["received"][-1]
            other = json.loads(json.dumps(parent))
            other.pop("branches", None)
            other["expected"] = parent["revision"]["id"]
            other["revision"]["parents"] = [other["expected"]]
            other["revision"]["id"] = str(uuid.uuid4())
            other["revision"]["device"] = str(uuid.uuid4())
            if remote_deleted:
                other["revision"]["content"] = None
                other["content_base64"] = None
            else:
                other["revision"]["path"] = "remote-renamed.md"
            local_bytes = b"local versus deletion" if remote_deleted else b"local versus rename"
            (source / "original.md").write_bytes(local_bytes)
            run([client, "stage", str(sender)])
            assert request("POST", client_sync, other, {"Authorization": "Bearer " + client_token})[0] == 200
            run([client, "fetch", str(sender), str(client_secret)])
            conflict = json.loads(run([client, "conflicts", str(sender)]))[0]
            command = "resolve-delete" if remote_deleted else "resolve-to"
            choice = [client, command, str(sender), conflict["local"], conflict["remote"], "original.md"]
            if not remote_deleted:
                choice.append(str(chosen))
            run(choice)
            run([client, "transfer", str(sender), str(client_secret)])
            run([client, "transfer", str(sender), str(client_secret)])
            merged = json.loads((sender / "client.json").read_text())["received"][-1]
            assert set(merged["revision"]["parents"]) == {conflict["local"], conflict["remote"]}
            assert (merged["revision"]["content"] is None) == remote_deleted
            assert (source / "original.md").read_bytes() == local_bytes
            assert not (source / "remote-renamed.md").exists()
        run([client, "fetch", str(fresh_state), str(client_secret)])
        refused = subprocess.run([client, "apply", str(fresh_state), str(temp / "resolved-app-data")], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        assert refused.returncode != 0
        assert (fresh_root / "original.md").read_bytes() == chosen.read_bytes()
        assert not (fresh_root / "remote-renamed.md").exists()
        assert client_token not in (sender / "client.json").read_text()
        credentials = json.loads(cli("token", "list"))
        cli("token", "revoke", credentials[0]["id"])
        assert request("GET", note)[0] == 401
        assert request("GET", sync)[0] == 401
    finally:
        if proc:
            proc.terminate()
            proc.wait(timeout=15)
    if compose:
        container = run(cmd + ["ps", "-q", "notes-server"])
        run(cmd + ["stop", "notes-server"])
        backups = temp / "backups"
        backups.mkdir(mode=0o777)
        backups.chmod(0o777)  # Disposable CI directory writable by container UID 10001.
        run(["docker", "run", "--rm", "--network", "none", "--volumes-from", container,
             "-v", str(backups) + ":/backup", "notes-server:local", "backup", "/backup/notes.tar.gz"])
        run(["docker", "run", "--rm", "--network", "none", "-v", str(backups) + ":/backup",
             "notes-server:local", "restore", "/backup/notes.tar.gz", "/backup/restored", "/data"])
        restored_mount = str(backups / "restored") + ":/data"
        rows = json.loads(run(["docker", "run", "--rm", "--network", "none", "-v", restored_mount,
                               "notes-server:local", "token", "list"]))
        assert rows[0]["revoked"] is True
        data = subprocess.check_output(["docker", "run", "--rm", "--network", "none", "-v", restored_mount,
                                        "--entrypoint", "cat", "notes-server:local", "/data/workspaces/smoke/smoke.md"])
        assert data == b"changed\r\nonce\r\n"
        vault = json.loads(subprocess.check_output(["docker", "run", "--rm", "--network", "none", "-v", restored_mount,
                                                   "--entrypoint", "cat", "notes-server:local", "/data/sync/smoke/vault.json"]))
        assert vault["publications"] == [publication]
        # Remove only this disposable volume fixture using its owning UID so
        # TemporaryDirectory can clean up the host-side test directory too.
        run(["docker", "run", "--rm", "--network", "none", "-v", str(backups) + ":/backup",
             "--entrypoint", "sh", "notes-server:local", "-c", "rm -rf /backup/restored /backup/notes.tar.gz"])
    if not compose:
        cli("backup", str(temp / "backup.tar.gz"))
        cli("restore", str(temp / "backup.tar.gz"), str(temp / "restored"))
        assert (temp / "restored/workspaces/smoke/smoke.md").read_bytes() == b"changed\r\nonce\r\n"
        assert json.loads((temp / "restored/sync/smoke/vault.json").read_text())["publications"] == [publication]
    print("HTTPS container and offline restore smoke passed" if compose else "native TCP and offline restore smoke passed")
