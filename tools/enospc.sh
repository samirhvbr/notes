#!/usr/bin/env bash
# The full-disk acceptance criterion, on a filesystem that is genuinely full.
#
# `docs/ACCEPTANCE-0.1a.md` §5 recorded this as *partly met* with a manual
# recipe that needed `sudo mount -o loop`. It does not need one on a developer's
# machine: an **unprivileged user namespace** can mount a `tmpfs`, and a
# size-capped tmpfs returns ENOSPC exactly like a full disk.
#
# It does need one on a GitHub runner. Ubuntu 24.04 ships
# `kernel.apparmor_restrict_unprivileged_userns=1`, so `unshare -U -r` fails
# with *"write failed /proc/self/uid_map: Operation not permitted"* — which is
# the hardened-kernel case this script already anticipated, arriving from the
# one machine that has passwordless `sudo` and can therefore do it the other
# way. So both are tried, in the order that needs the fewest privileges
# (`docs/DECISIONS-0.1b.md` D-03).
#
# **Skipping is loud, and CI refuses to skip.** `NOTES_REQUIRE_ENOSPC=1` — set
# on the Linux CI leg — turns "no mechanism available" into a failure, because a
# check that quietly opts out on the machine that enforces it is not a check.
# On a developer machine with neither mechanism it warns and returns 0.
#
# Usage: tools/enospc.sh [size]      (default 1M — the note is a few bytes)
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SIZE="${1:-1M}"
REQUIRED="${NOTES_REQUIRE_ENOSPC:-}"

skip() {
  echo "enospc: SKIPPED — $1"
  if [ -n "$REQUIRED" ]; then
    echo "enospc: FAILED — NOTES_REQUIRE_ENOSPC is set, so a skip is a failure"
    exit 1
  fi
  exit 0
}

[ "$(uname -s)" = "Linux" ] || skip "needs Linux (this is $(uname -s))"
command -v unshare >/dev/null 2>&1 || command -v sudo >/dev/null 2>&1 ||
  skip "neither unshare(1) nor sudo(8) on this machine"

# Build outside any namespace: cargo inside one works, but a first build there
# is slow and the failure mode if it cannot write to ~/.cargo is confusing.
echo "== building the test binary"
cargo test -p notes-core --test enospc --no-run --quiet || exit 1

# Ask cargo where it put it rather than guessing at the hash suffix.
BIN=$(cargo test -p notes-core --test enospc --no-run --message-format=json 2>/dev/null \
  | python3 -c '
import json, sys
for line in sys.stdin:
    try: m = json.loads(line)
    except ValueError: continue
    if m.get("target", {}).get("name") == "enospc" and m.get("executable"):
        print(m["executable"])
')
if [ -z "$BIN" ]; then
  echo "enospc: FAILED — could not locate the compiled test binary"
  exit 1
fi

TEST=a_full_disk_is_reported_and_leaves_the_buffer_recoverable

# ---- 1. no privileges at all -------------------------------------------
#
# `-U -r` maps this user to root *inside the namespace only*, which grants
# CAP_SYS_ADMIN over the namespace's own mounts and nothing else. `-m` gives it
# a private mount table, so the tmpfs is invisible outside and disappears with
# the process — a failed run cannot leave a filesystem mounted anywhere.
if command -v unshare >/dev/null 2>&1 &&
   unshare -U -r -m true >/dev/null 2>&1; then
  echo "== running against a ${SIZE} tmpfs in a user namespace"
  unshare -U -r -m bash -s "$SIZE" "$BIN" "$TEST" <<'INNER'
set -uo pipefail
size="$1"; bin="$2"; test="$3"
tiny=$(mktemp -d)
mount -t tmpfs -o "size=$size,mode=0700" tmpfs "$tiny" || {
  echo "enospc: could not mount a tmpfs inside the namespace"
  exit 1
}
NOTES_TINY_DIR="$tiny" "$bin" --ignored --exact --nocapture "$test"
rc=$?
umount "$tiny" 2>/dev/null
exit $rc
INNER
  rc=$?
  if [ "$rc" -eq 0 ]; then
    echo "enospc: a real ENOSPC is reported as DiskFull and the buffer survives"
    exit 0
  fi
  echo "enospc: FAILED (rc=$rc) in the user namespace"
  exit "$rc"
fi

# ---- 2. passwordless sudo ----------------------------------------------
#
# The CI path. `sudo -n` never prompts, so a developer machine with a password
# falls through to the skip rather than stopping the gate to ask for one.
if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
  echo "== user namespaces are restricted here; using sudo and a ${SIZE} tmpfs"
  tiny=$(mktemp -d)
  cleanup() { sudo -n umount "$tiny" >/dev/null 2>&1; rmdir "$tiny" 2>/dev/null; }
  trap cleanup EXIT
  if ! sudo -n mount -t tmpfs -o "size=$SIZE,mode=0777" tmpfs "$tiny"; then
    echo "enospc: FAILED — sudo is available but the mount was refused"
    exit 1
  fi
  NOTES_TINY_DIR="$tiny" "$BIN" --ignored --exact --nocapture "$TEST"
  rc=$?
  if [ "$rc" -eq 0 ]; then
    echo "enospc: a real ENOSPC is reported as DiskFull and the buffer survives"
  else
    echo "enospc: FAILED (rc=$rc) on the sudo-mounted tmpfs"
  fi
  exit "$rc"
fi

skip "this kernel restricts unprivileged user namespaces and there is no \
passwordless sudo; the manual recipe is in docs/ACCEPTANCE-0.1a.md §5"
