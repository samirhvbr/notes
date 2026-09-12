#!/usr/bin/env python3
"""Sign updater payloads, record them durably, and publish each feed last."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import shlex
import subprocess
import tempfile
from urllib.parse import quote
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / 'apps/notes-app/node_modules/.bin/tauri'
CONFIG = ROOT / 'apps/notes-app/src-tauri/tauri.conf.json'


def sha(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def signing_env():
    env = os.environ.copy()
    if not env.get('TAURI_SIGNING_PRIVATE_KEY') and not env.get('TAURI_SIGNING_PRIVATE_KEY_PATH'):
        key = Path.home() / '.config/tura-notes/updater.key'
        if not key.is_file():
            raise ValueError('Updater signing key missing. Set TAURI_SIGNING_PRIVATE_KEY_PATH or use --no-sign for a local test build.')
        env['TAURI_SIGNING_PRIVATE_KEY_PATH'] = str(key)
    env.setdefault('TAURI_SIGNING_PRIVATE_KEY_PASSWORD', '')
    return env


def sign(path):
    # Never echo signer stdout: some CLI versions include key-related diagnostics.
    result = subprocess.run([str(CLI), 'signer', 'sign', str(path)], env=signing_env(),
                            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    if result.returncode:
        raise ValueError('Updater signing failed. Check the configured key and password.')
    subprocess.run(['cargo', 'run', '--locked', '--quiet', '--manifest-path', str(ROOT / 'tools/updater-verify/Cargo.toml'),
                    '--', str(path), str(path) + '.sig', str(CONFIG)], check=True)


def record_path(artifact):
    return Path(str(artifact) + '.updater.json')


def verify(artifact, version):
    data = json.loads(record_path(artifact).read_text())
    if data['version'] != version or data['sha256'] != sha(artifact):
        raise ValueError('Updater payload version or hash mismatch')
    signature = Path(str(artifact) + '.sig').read_text().strip()
    if signature != data['signature']:
        raise ValueError('Updater signature changed after build')
    pubkey = json.loads(CONFIG.read_text())['plugins']['updater']['pubkey']
    if data['pubkey'] != pubkey:
        raise ValueError('Updater key changed; rebuild the payload')
    return data


def prepare(artifact, version, platform):
    try:
        data = verify(artifact, version)
        if data['platform'] == platform:
            return
    except (OSError, ValueError, KeyError):
        pass
    sign(artifact)
    data = dict(version=version, platform=platform, sha256=sha(artifact),
                signature=Path(str(artifact) + '.sig').read_text().strip(),
                pubkey=json.loads(CONFIG.read_text())['plugins']['updater']['pubkey'])
    temp = Path(str(record_path(artifact)) + '.tmp')
    temp.write_text(json.dumps(data, indent=2) + '\n')
    temp.replace(record_path(artifact))


def publish(artifact, version, host, app, stage, base):
    data = verify(artifact, version)
    if not re.fullmatch(r'[a-zA-Z0-9_.@-]+', host) or host.startswith('-'):
        raise ValueError('Invalid upload host')
    if not stage.startswith('/'):
        raise ValueError('Staging directory must be absolute')
    if not base.startswith('https://'):
        raise ValueError('Updater downloads require an HTTPS base URL')
    destination = app.rstrip('/') + '/public/updates/tura-notes'
    # Keep versions and platform payloads separate; updating Linux must not erase macOS.
    filename = f"{version}-{data['platform']}-{data['sha256'][:16]}-{artifact.name}"
    feed = data['platform'] + '.json'
    content = dict(version=version, notes=f'Tura Notes {version}',
                   url=base.rstrip('/') + '/updates/tura-notes/' + quote(filename, safe=''),
                   signature=data['signature'])
    q = shlex.quote
    with tempfile.TemporaryDirectory(prefix='tura-update-') as local:
        manifest = Path(local) / feed
        manifest.write_text(json.dumps(content, indent=2) + '\n')
        # A unique staging directory keeps simultaneous platform publishes separate.
        remote = subprocess.check_output(['ssh', host, f'mktemp -d {q(stage.rstrip("/") + "/tura-update.XXXXXX")}'], text=True).strip()
        if not remote.startswith(stage.rstrip('/') + '/tura-update.') or '\n' in remote:
            raise ValueError('Unexpected remote staging directory')
        try:
            subprocess.run(['scp', str(artifact), str(manifest), f'{host}:{remote}/'], check=True)
            staged_payload = remote + '/' + artifact.name
            actual = subprocess.check_output(['ssh', host, 'sha256sum -- ' + q(staged_payload)], text=True).split()[0]
            if actual != data['sha256']:
                raise ValueError('Updater upload checksum mismatch; feed unchanged')
            feed_temp = destination + '/' + feed + '.' + remote.rsplit('/', 1)[-1] + '.tmp'
            command = (
                f'sudo -u www-data mkdir -p {q(destination)} && '
                f'sudo -u www-data install -m 644 {q(staged_payload)} {q(destination + "/" + filename)} && '
                f'sudo -u www-data install -m 644 {q(remote + "/" + feed)} {q(feed_temp)} && '
                f'sudo -u www-data mv -f {q(feed_temp)} {q(destination + "/" + feed)}'
            )
            # mktemp creates mode 700; permit the service user to read staged files.
            subprocess.run(['ssh', host, f'chmod 755 {q(remote)} && chmod 644 {q(staged_payload)} {q(remote + "/" + feed)} && ' + command], check=True)
        finally:
            subprocess.run(['ssh', host, 'rm -rf -- ' + q(remote)], check=False)
    feed_url = base.rstrip('/') + '/updates/tura-notes/' + feed
    with urlopen(feed_url, timeout=30) as response:
        if json.load(response) != content:
            raise ValueError('Published updater feed did not match the generated manifest')
    value = hashlib.sha256()
    with urlopen(content['url'], timeout=120) as response:
        for chunk in iter(lambda: response.read(1024 * 1024), b''):
            value.update(chunk)
    if value.hexdigest() != data['sha256']:
        raise ValueError('Public updater download checksum mismatch')
    print(f'Updater feed published and verified: {feed_url}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('command', choices=['preflight', 'prepare', 'verify', 'publish'])
    parser.add_argument('--artifact', type=Path)
    parser.add_argument('--version')
    parser.add_argument('--platform')
    parser.add_argument('--host')
    parser.add_argument('--app')
    parser.add_argument('--stage', default='/tmp')
    parser.add_argument('--base')
    args = parser.parse_args()
    if args.command == 'preflight':
        with tempfile.TemporaryDirectory() as directory:
            probe = Path(directory) / 'probe'
            probe.write_bytes(b'Tura Notes updater signing preflight\n')
            sign(probe)
    elif args.command == 'prepare':
        prepare(args.artifact, args.version, args.platform)
    elif args.command == 'verify':
        verify(args.artifact, args.version)
    else:
        publish(args.artifact, args.version, args.host, args.app, args.stage, args.base)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'updater: {error}')
