#!/usr/bin/env python3
"""Record completed Linux bundles and verify them before a publish retry."""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
INPUTS = ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo', 'crates',
          'packages', 'apps/notes-app/src', 'apps/notes-app/public',
          'apps/notes-app/index.html', 'apps/notes-app/package.json',
          'apps/notes-app/package-lock.json', 'apps/notes-app/src-tauri',
          'tools/tauri.mjs', 'tools/stamp-version.sh']
SKIP = {'target', 'node_modules', 'dist', '.git', 'gen'}


def digest(path):
    with path.open('rb') as stream:
        value = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            value.update(chunk)
        return value.hexdigest()


def fingerprint():
    files = set()
    for name in INPUTS:
        path = ROOT / name
        if path.is_file():
            files.add(path)
        elif path.is_dir():
            # Prune build directories instead of walking a potentially huge cache.
            import os
            for base, dirs, names in os.walk(path):
                dirs[:] = [d for d in dirs if d not in SKIP]
                files.update(Path(base) / n for n in names)
    files.update((ROOT / 'apps/notes-app').glob('*config*.json'))
    files.update((ROOT / 'apps/notes-app').glob('*config*.ts'))
    value = hashlib.sha256()
    for path in sorted(p for p in files if p.is_file()):
        value.update(str(path.relative_to(ROOT)).encode() + b'\0')
        value.update(digest(path).encode() + b'\0')
    return value.hexdigest()


def main():
    command = sys.argv[1]
    if command == 'fingerprint':
        print(fingerprint())
        return
    directory, version, host, source, mode = sys.argv[2:7]
    directory = Path(directory)
    manifest = directory / '.build.json'
    identity = dict(version=version, host=host, source=source, mode=mode)
    if command == 'record':
        artifacts = [Path(p) for p in sys.argv[7:]]
        data = dict(identity, artifacts=[dict(name=p.name, sha256=digest(p)) for p in artifacts])
        temporary = manifest.with_suffix('.tmp')
        temporary.write_text(json.dumps(data, indent=2) + '\n')
        temporary.replace(manifest)
        return
    try:
        data = json.loads(manifest.read_text())
        if any(data.get(key) != value for key, value in identity.items()):
            raise ValueError('version, sources, architecture or build mode changed')
        entries = data['artifacts']
        if not entries:
            raise ValueError('empty artifact list')
        paths = []
        for entry in entries:
            name = entry['name']
            if Path(name).name != name:
                raise ValueError('invalid artifact path')
            path = directory / name
            if digest(path) != entry['sha256']:
                raise ValueError(f'checksum mismatch: {name}')
            sidecar = Path(str(path) + '.sha256')
            if sidecar.read_text().split()[0] != entry['sha256']:
                raise ValueError(f'missing or invalid checksum sidecar: {name}')
            paths.append(path)
        for path in paths:
            sys.stdout.buffer.write(str(path).encode() + b'\0')
    except (OSError, ValueError, KeyError, TypeError, IndexError) as error:
        print(f'Build required: {error}', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
