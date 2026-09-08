#!/usr/bin/env bash
# Build the release tarball the AUR `notes-bin` package consumes.
#
# WHY IT IS BUILT FROM THE `.deb` AND NOT ASSEMBLED BY HAND: the binary, the
# `.desktop` entry and the icon set are produced by the Tauri bundler from
# `tauri.conf.json`. Writing a second copy of the desktop entry here would be a
# second version of one rule, and the two would drift the first time an icon
# size or a MIME type changed. Unpacking the `.deb` takes exactly what shipped.
#
#   packaging/linux/tarball.sh <version> [deb] [outdir]
#
# Produces `notes-<version>-x86_64-linux.tar.gz` laid out relative to a prefix:
#
#   notes-<version>-x86_64-linux/
#     bin/notes
#     share/applications/notes.desktop
#     share/icons/hicolor/<size>/apps/notes.png
#     LICENSE
#     README.md
#
# `PKGBUILD` installs that into `/usr`, which is why the paths are prefix-
# relative and carry no leading `usr/`.
#
# Norm: docs/ARCHITECTURE.md §15 · docs/decisions.md ADR-023
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${1:?usage: tarball.sh <version> [deb] [outdir]}"
BUNDLE="$ROOT/target/release/bundle"
DEB="${2:-}"
OUT="${3:-$ROOT/dist-release}"

if [ -z "$DEB" ]; then
  DEB="$(find "$BUNDLE/deb" -maxdepth 1 -name '*.deb' -print -quit 2>/dev/null || true)"
fi
[ -n "$DEB" ] && [ -f "$DEB" ] || { echo "tarball.sh: no .deb found (looked in $BUNDLE/deb)" >&2; exit 1; }

NAME="notes-$VERSION-x86_64-linux"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

dpkg-deb -x "$DEB" "$WORK/deb"

STAGE="$WORK/$NAME"
mkdir -p "$STAGE/bin" "$STAGE/share"
cp "$WORK/deb/usr/bin/notes" "$STAGE/bin/notes"
chmod 755 "$STAGE/bin/notes"
cp -r "$WORK/deb/usr/share/applications" "$STAGE/share/applications"
cp -r "$WORK/deb/usr/share/icons" "$STAGE/share/icons"
cp "$ROOT/LICENSE" "$STAGE/LICENSE"
cp "$ROOT/README.md" "$STAGE/README.md"

mkdir -p "$OUT"
# Reproducible enough to be worth it: sorted, one owner, one timestamp taken
# from the source rather than from the clock.
tar --sort=name \
    --owner=0 --group=0 --numeric-owner \
    --mtime="@$(git -C "$ROOT" log -1 --format=%ct 2>/dev/null || echo 0)" \
    -czf "$OUT/$NAME.tar.gz" -C "$WORK" "$NAME"

sha256sum "$OUT/$NAME.tar.gz" | awk '{print $1}' > "$OUT/$NAME.tar.gz.sha256"
echo "$OUT/$NAME.tar.gz"
