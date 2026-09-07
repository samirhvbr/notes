#!/usr/bin/env bash
# Everything that must be green before a commit, in the order that fails fastest.
#
# The cross-target step is the one worth explaining: `cargo clippy` on Linux
# cannot see code behind `#[cfg(windows)]`, and cannot see that a helper used
# only under `#[cfg(unix)]` becomes dead on Windows — where `-D warnings` turns
# it into a build failure. Two CI rounds were spent on exactly that. Checking the
# Windows target locally needs only `rustup target add x86_64-pc-windows-gnu`;
# it type-checks without linking, so no MSVC toolchain is involved.
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail=0
step() {
  printf '\n== %s\n' "$1"; shift
  if "$@"; then echo "   ok"; else echo "   FAILED"; fail=1; fi
}

step "cargo fmt"            cargo fmt --all --check
step "clippy (native)"      cargo clippy --all-targets -- -D warnings
if rustup target list --installed | grep -q x86_64-pc-windows-gnu; then
  step "clippy (windows)"   cargo clippy --target x86_64-pc-windows-gnu \
                              -p notes-model -p notes-fs -p notes-core \
                              --all-targets -- -D warnings
else
  printf '\n== clippy (windows)\n   SKIPPED — rustup target add x86_64-pc-windows-gnu\n'
fi
step "cargo test"           cargo test --workspace
step "byte preservation"    tools/byte-preservation.sh
step "generated types"      bash -c '
  rm -rf apps/notes-app/src/ipc/generated
  cargo test -p notes-model -p notes-core --quiet >/dev/null 2>&1
  git diff --quiet --exit-code -- apps/notes-app/src/ipc/generated'
step "no fs capability"     bash -c '
  ! grep -rqE "\"fs:[a-z-]+\"" apps/notes-app/src-tauri/capabilities/'
step "i18n keys match"      python3 -c '
import json,sys
en=json.load(open("apps/notes-app/src/i18n/en.json"))
pt=json.load(open("apps/notes-app/src/i18n/pt-BR.json"))
sys.exit(0 if set(en)==set(pt) else 1)'
step "frontend"             bash -c 'cd apps/notes-app && npm run build >/dev/null'

echo
if [ "$fail" -ne 0 ]; then echo "FAILED"; exit 1; fi
echo "all green"
