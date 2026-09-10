#!/usr/bin/env bash
# Everything that must be green before a commit, in the order that fails fastest.
#
# The cross-target step is the one worth explaining: `cargo clippy` on Linux
# cannot see code behind `#[cfg(windows)]`, and cannot see that a helper used
# only under `#[cfg(unix)]` becomes dead on Windows — where `-D warnings` turns
# it into a build failure. Two CI rounds were spent on exactly that.
#
# **It runs by default, and installs the target if it is missing.** Skipping it
# when `rustup target add x86_64-pc-windows-gnu` had never been run made the one
# step that would have caught both compile failures the one step nobody had —
# a check that silently opts out is not a check. The target type-checks without
# linking. Bundled SQLite additionally needs a MinGW C compiler
# (mingw-w64 on Homebrew / gcc-mingw-w64-x86-64 on Debian). The target install is a one-off of a
# few seconds. `NOTES_NO_WINDOWS_CHECK=1` opts out deliberately; a machine with
# no rustup and no target degrades to a warning rather than a failure, because
# refusing to run the rest of the gate over a cross-check helps nobody.
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail=0
step() {
  printf '\n== %s\n' "$1"; shift
  if "$@"; then echo "   ok"; else echo "   FAILED"; fail=1; fi
}

step "cargo fmt"            cargo fmt --all --check
step "clippy (native)"      cargo clippy --all-targets -- -D warnings
windows_target_ready() {
  [ -z "${NOTES_NO_WINDOWS_CHECK:-}" ] || { echo "opted out by NOTES_NO_WINDOWS_CHECK"; return 1; }
  command -v rustup >/dev/null 2>&1 || { echo "no rustup on this machine"; return 1; }
  rustup target list --installed 2>/dev/null | grep -q '^x86_64-pc-windows-gnu$' && return 0
  echo "   installing the x86_64-pc-windows-gnu target (one-off)…" >&2
  rustup target add x86_64-pc-windows-gnu >/dev/null 2>&1 || { echo "could not install the target"; return 1; }
}

if why=$(windows_target_ready); then
  step "clippy (windows)"   cargo clippy --target x86_64-pc-windows-gnu \
                              -p notes-model -p notes-fs -p notes-core -p notes-markdown -p notes-index -p notes-mcp -p notes-server \
                              --all-targets -- -D warnings
else
  printf '\n== clippy (windows)\n   WARNING, not run — %s\n' "${why:-unknown}"
fi
step "cargo test"           cargo test --workspace
step "server TCP smoke"    python3 server/tests/smoke.py
step "byte preservation"    tools/byte-preservation.sh
step "full disk (ENOSPC)"   tools/enospc.sh
step "generated types"      bash -c '
  rm -rf apps/notes-app/src/ipc/generated
  cargo test -p notes-model -p notes-core -p notes-markdown --lib --quiet >/dev/null 2>&1
  # Two questions, because one command answers only half of it: `git diff` sees
  # a changed file, and a type added by a new crate arrives *untracked*, which a
  # diff does not see at all.
  git diff --quiet --exit-code -- apps/notes-app/src/ipc/generated &&
  [ -z "$(git ls-files --others --exclude-standard -- apps/notes-app/src/ipc/generated)" ]'
step "no fs capability"     bash -c '
  ! grep -rqE "\"fs:[a-z-]+\"" apps/notes-app/src-tauri/capabilities/'
# The bundle version is stamped from version.md at build time (ADR-035). What
# is committed is the placeholder; a real number here is a second copy of the
# version, and it is the copy that goes stale.
step "version placeholder"  bash -c '
  grep -q '"'"'"version": "0.0.0"'"'"' apps/notes-app/src-tauri/tauri.conf.json'
# Every text colour against every surface it can land on, and the three dark
# levels far enough apart to survive a bad panel (`ACCEPTANCE-0.1d.md`).
step "contrast"            tools/contrast.sh
step "no blocking dialogs" tools/no-blocking-dialogs.sh
step "i18n keys match"      python3 -c '
import json,sys
en=json.load(open("apps/notes-app/src/i18n/en.json"))
pt=json.load(open("apps/notes-app/src/i18n/pt-BR.json"))
sys.exit(0 if set(en)==set(pt) else 1)'
step "development version" node --test tools/tauri.test.mjs
step "frontend tests"       bash -c 'cd apps/notes-app && npm test -- --run >/dev/null'
step "frontend"             bash -c 'cd apps/notes-app && npm run build >/dev/null'

echo
if [ "$fail" -ne 0 ]; then echo "FAILED"; exit 1; fi
echo "all green"
