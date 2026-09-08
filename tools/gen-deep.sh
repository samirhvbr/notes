#!/usr/bin/env bash
# Generate `fixtures/deep/` — a folder shaped like the one that froze the
# application: many repositories, each with a `node_modules` tree, a `target/`
# and a `.git/`.
#
# `fixtures/large` measures **notes**: 10 000 files, few directories, all of them
# notes. This one measures **directories**, which is the axis that broke. The two
# are not interchangeable: 10 000 notes list in 37 ms and this corpus took over
# two minutes to reach the tree.
#
# Three hazards are deliberate and each one has bitten:
#
#   * a subdirectory with mode 000 — the owner's `.../www/web1/ead` — which the
#     old watcher treated as a reason to demote the whole workspace to polling;
#   * a symlink loop, which a naive recursive walk follows forever;
#   * enough directories to exhaust `max_user_watches` on a default Linux.
#
# Never committed (`.gitignore`, ADR-011); regenerated on demand.
#
#   tools/gen-deep.sh              ~160 repos, ~30 000 directories
#   REPOS=20 tools/gen-deep.sh     a quick local pass
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${OUT:-$ROOT/fixtures/deep}"
REPOS="${REPOS:-160}"
PKGS="${PKGS:-24}"

if [ -e "$OUT" ]; then
  echo "removing existing $OUT" >&2
  chmod -R u+rwX "$OUT" 2>/dev/null || true
  rm -rf "$OUT"
fi

python3 - "$OUT" "$REPOS" "$PKGS" <<'PY'
import os, pathlib, sys

out, repos, pkgs = pathlib.Path(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
out.mkdir(parents=True)

dirs = files = notes = 0
for r in range(repos):
    repo = out / f"repo-{r:03d}"
    # The parts a real checkout has, including the ones the app must not walk
    # into blindly.
    for d in (".git/objects/pack", ".git/refs/heads", "target/debug/deps", "docs"):
        (repo / d).mkdir(parents=True, exist_ok=True)
        dirs += 4
    (repo / "README.md").write_text(f"# repo {r}\n\nnota de verdade\n")
    (repo / "docs" / "guia.md").write_text("# guia\n\ncorpo\n")
    notes += 2
    files += 2
    (repo / ".git" / "HEAD").write_text("ref: refs/heads/master\n")
    (repo / "target" / "debug" / "deps" / "libfoo.rlib").write_bytes(b"\0" * 64)
    files += 2

    for p in range(pkgs):
        pkg = repo / "node_modules" / f"pkg-{p:02d}"
        for d in ("dist", "src", "lib/esm"):
            (pkg / d).mkdir(parents=True, exist_ok=True)
            dirs += 1
        # node_modules is full of Markdown, which is the whole reason hiding it
        # by name is a product decision and not an obvious one.
        (pkg / "README.md").write_text(f"# pkg {p}\n\nreadme de dependencia\n")
        (pkg / "package.json").write_text('{"name":"pkg"}\n')
        notes += 1
        files += 2

# One unreadable directory: the shape of the owner's `.../www/web1/ead`.
denied = out / "repo-000" / "docs" / "ead"
denied.mkdir(parents=True, exist_ok=True)
(denied / "segredo.md").write_text("# nao pode ler\n")
os.chmod(denied, 0o000)
dirs += 1

# A symlink loop: a walk that follows links never returns from here.
loop = out / "repo-001" / "loop"
loop.mkdir(exist_ok=True)
os.symlink("..", loop / "up")
os.symlink(str(loop), loop / "self")
dirs += 1

print(f"{dirs} directories · {files} files · {notes} of them notes · in {out}")
print("hazards: 1 unreadable directory (mode 000), 1 symlink loop")
PY

echo
echo "max_user_watches on this machine: $(cat /proc/sys/fs/inotify/max_user_watches 2>/dev/null || echo 'n/a')"
echo "directories under $OUT: $(find "$OUT" -type d 2>/dev/null | wc -l) (the unreadable one is not counted)"
