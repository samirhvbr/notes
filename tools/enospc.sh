#!/usr/bin/env bash
# The full-disk acceptance criterion, on a filesystem that is genuinely full.
#
# `docs/ACCEPTANCE-0.1a.md` §5 recorded this as *partly met* with a manual
# recipe that needed `sudo mount -o loop`. It does not: an **unprivileged user
# namespace** can mount a `tmpfs`, any unprivileged user can create one, and a
# size-capped tmpfs returns ENOSPC exactly like a full disk. So the criterion
# becomes a test that runs on a developer machine and in CI with no privileges
# at all, and `docs/DECISIONS-0.1b.md` D-03 records why that beat the loopback.
#
# The namespace matters for a second reason: the mount is private to it, so a
# failed run cannot leave a mounted filesystem behind on anyone's machine.
#
# Usage: tools/enospc.sh [size]      (default 1M — the note is a few bytes)
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SIZE="${1:-1M}"

if [ "$(uname -s)" != "Linux" ]; then
  echo "enospc: skipped — needs Linux user namespaces (this is $(uname -s))"
  exit 0
fi
if ! command -v unshare >/dev/null 2>&1; then
  echo "enospc: skipped — no unshare(1) on this machine"
  exit 0
fi

# Build outside the namespace: cargo inside one works, but a first build there
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

echo "== running against a ${SIZE} tmpfs in a user namespace"
# `-U -r` maps this user to root *inside the namespace only*, which is what
# grants CAP_SYS_ADMIN over the namespace's own mounts and nothing else. `-m`
# gives it a private mount table so the tmpfs is invisible outside and
# disappears with the process.
unshare -U -r -m bash -s "$SIZE" "$BIN" <<'INNER'
set -uo pipefail
size="$1"; bin="$2"
tiny=$(mktemp -d)
mount -t tmpfs -o "size=$size,mode=0700" tmpfs "$tiny" || {
  echo "enospc: FAILED — could not mount a tmpfs inside the namespace"
  exit 1
}
NOTES_TINY_DIR="$tiny" "$bin" --ignored --exact --nocapture \
  a_full_disk_is_reported_and_leaves_the_buffer_recoverable
rc=$?
umount "$tiny" 2>/dev/null
exit $rc
INNER
rc=$?

if [ "$rc" -eq 0 ]; then
  echo "enospc: a real ENOSPC is reported as DiskFull and the buffer survives"
else
  # A kernel with `kernel.unprivileged_userns_clone=0` (some hardened distros)
  # cannot do this, and that is a machine fact rather than a defect in the code.
  echo "enospc: FAILED (rc=$rc) — if this machine forbids unprivileged user"
  echo "        namespaces, run the manual recipe in docs/ACCEPTANCE-0.1a.md §5"
fi
exit $rc
