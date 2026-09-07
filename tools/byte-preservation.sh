#!/usr/bin/env bash
# 0.1a acceptance, in its literal form: "Abrir cada arquivo dos fixtures e
# salvar sem editar → `git status` limpo."
#
# Runs against the committed corpus in the working tree, because git is the only
# thing that can answer "git sees no change". The hermetic version of the same
# assertion is `cargo test -p notes-fs --test fixtures`, which works on a copy.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [ -n "$(git status --porcelain -- fixtures/basic fixtures/edge-cases)" ]; then
  echo "the corpus is already dirty — commit or restore it first" >&2
  git status --short -- fixtures/basic fixtures/edge-cases >&2
  exit 1
fi

cargo build --quiet -p notes-fs --bin save-unchanged
for corpus in basic edge-cases; do
  echo "== fixtures/$corpus"
  ./target/debug/save-unchanged "fixtures/$corpus"
done

dirty="$(git status --porcelain -- fixtures/basic fixtures/edge-cases)"
if [ -n "$dirty" ]; then
  echo
  echo "FAIL: saving unchanged modified the corpus:"
  echo "$dirty"
  exit 1
fi
echo
echo "PASS: every file opened and saved unchanged; git status is clean"
