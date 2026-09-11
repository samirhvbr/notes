#!/usr/bin/env bash
# Build a local macOS DMG. This does not publish, sign, or notarize an artifact.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

skip_npm_ci=0
skip_git_pull=0
for arg in "$@"; do
  case "$arg" in
    --skip-npm-ci) skip_npm_ci=1 ;;
    --skip-git-pull) skip_git_pull=1 ;;
    --help)
      cat <<'EOF'
Usage: ./build-local.sh [--skip-npm-ci] [--skip-git-pull]

Builds a local macOS DMG at:
  target/release/bundle/dmg/

The DMG is for local verification only. It is not signed, notarized, or
published. The build version is stamped from version.md temporarily and the
committed 0.0.0 placeholder is restored when the script exits.
EOF
      exit 0
      ;;
    *) echo "build-local.sh: unknown option: $arg" >&2; exit 2 ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  echo "build-local.sh: local DMG builds require macOS." >&2
  exit 1
fi

command -v node >/dev/null || { echo "build-local.sh: Node.js is required." >&2; exit 1; }
command -v npm >/dev/null || { echo "build-local.sh: npm is required." >&2; exit 1; }
command -v cargo >/dev/null || { echo "build-local.sh: Rust cargo is required." >&2; exit 1; }
xcode-select -p >/dev/null || { echo "build-local.sh: install Xcode Command Line Tools first." >&2; exit 1; }

if [ "$skip_git_pull" -eq 0 ]; then
  git pull --ff-only
fi

config="apps/notes-app/src-tauri/tauri.conf.json"
original="$(mktemp)"
cp "$config" "$original"
cleanup() {
  cp "$original" "$config"
  rm -f "$original"
}
trap cleanup EXIT

version="$(tools/stamp-version.sh)"
if [ "$skip_npm_ci" -eq 0 ]; then
  (cd apps/notes-app && npm ci)
fi

(cd apps/notes-app && npm run tauri build -- --bundles dmg)

artifact_dir="target/release/bundle/dmg"
artifact="$(find "$artifact_dir" -maxdepth 1 -type f -name "*_${version}_*.dmg" -print -quit)"
if [ -z "$artifact" ]; then
  echo "build-local.sh: Tauri completed without a DMG in $artifact_dir." >&2
  exit 1
fi

echo "Built Tura Notes $version: $artifact"
echo "Local verification only: do not distribute this unsigned, unnotarized DMG."
