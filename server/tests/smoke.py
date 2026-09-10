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

compose = sys.argv[1:] == ["--compose"]
root = pathlib.Path(__file__).resolve().parents[2]
cmd = ["docker", "compose", "-f", "server/compose.yml", "-f", "server/tests/compose.yml"]
env = os.environ.copy()
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
        credentials = json.loads(cli("token", "list"))
        cli("token", "revoke", credentials[0]["id"])
        assert request("GET", note)[0] == 401
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
        # Remove only this disposable volume fixture using its owning UID so
        # TemporaryDirectory can clean up the host-side test directory too.
        run(["docker", "run", "--rm", "--network", "none", "-v", str(backups) + ":/backup",
             "--entrypoint", "sh", "notes-server:local", "-c", "rm -rf /backup/restored /backup/notes.tar.gz"])
    if not compose:
        cli("backup", str(temp / "backup.tar.gz"))
        cli("restore", str(temp / "backup.tar.gz"), str(temp / "restored"))
        assert (temp / "restored/workspaces/smoke/smoke.md").read_bytes() == b"changed\r\nonce\r\n"
    print("HTTPS container and offline restore smoke passed" if compose else "native TCP and offline restore smoke passed")
