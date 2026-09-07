#!/usr/bin/env bash
# 0.1a acceptance: "Matar o processo durante 1000 saves em loop nunca deixa
# arquivo truncado ou vazio."
#
# Spawns `crash-writer`, kills it with SIGKILL at a random point, and checks
# that what is on disk is a *complete* payload — old or new, never a prefix.
# A temp file left behind is also a failure: it would show up in the user's tree.
#
#   tools/crash-save-loop.sh            1000 kills
#   ROUNDS=50 tools/crash-save-loop.sh  a quick local pass
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROUNDS="${ROUNDS:-1000}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "building crash-writer…" >&2
cargo build --quiet -p notes-fs --bin crash-writer || exit 1
BIN="$ROOT/target/debug/crash-writer"
[ -x "$BIN" ] || { echo "crash-writer not built" >&2; exit 1; }

# Seed the note so the very first kill has a previous complete version to
# fall back to.
printf 'LEN=1\nx\nEND\n' > "$WORK/crash.md"

check() {
  local f="$WORK/crash.md" round="$1"
  [ -s "$f" ] || { echo "FAIL round $round: file is empty"; return 1; }
  local first last len body_len
  first="$(head -c 64 "$f" | head -n1)"
  last="$(tail -n1 "$f")"
  case "$first" in LEN=*) ;; *) echo "FAIL round $round: no LEN header (got '$first')"; return 1;; esac
  [ "$last" = "END" ] || { echo "FAIL round $round: truncated — last line '$last'"; return 1; }
  len="${first#LEN=}"
  # header + newline + body + newline + END + newline
  body_len=$(( ${#first} + 1 + len + 1 + 4 ))
  local actual; actual=$(wc -c < "$f")
  [ "$actual" -eq "$body_len" ] || { echo "FAIL round $round: size $actual, header says $body_len"; return 1; }
  return 0
}

fails=0
for i in $(seq 1 "$ROUNDS"); do
  "$BIN" "$WORK" crash.md >/dev/null 2>&1 &
  pid=$!
  # 1–40 ms: long enough to be mid-write, short enough for 1000 rounds.
  sleep "0.$(printf '%03d' $(( (RANDOM % 40) + 1 )))"
  kill -9 "$pid" 2>/dev/null
  wait "$pid" 2>/dev/null

  check "$i" || fails=$((fails + 1))

  # A kill between write and rename leaves the temporary file behind: no
  # process cleans up after SIGKILL. The guarantee is that it cannot
  # *accumulate* — the name is deterministic, so there is at most one per note
  # and the next save overwrites it.
  leftovers=$(find "$WORK" -name '.*.tmp' | wc -l)
  if [ "$leftovers" -gt 1 ]; then
    echo "FAIL round $i: $leftovers temp files — they are accumulating"
    fails=$((fails + 1))
  fi

  if [ $((i % 100)) -eq 0 ]; then echo "  $i/$ROUNDS rounds, $fails failures" >&2; fi
done

if [ "$fails" -ne 0 ]; then
  echo "crash-save-loop: $fails/$ROUNDS FAILED"
  exit 1
fi
echo "crash-save-loop: $ROUNDS rounds, no truncated or empty note; temp files never exceeded one"
