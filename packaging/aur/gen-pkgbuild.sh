#!/usr/bin/env bash
# Write `packaging/aur/notes-bin/PKGBUILD` from the template.
#
# Two callers, two sources, one template:
#
#   gen-pkgbuild.sh <version> <tarball>     # a local file — what CI builds against
#   gen-pkgbuild.sh <version> --release     # the published Release URL — what the AUR gets
#
# WHY CI BUILDS AGAINST THE LOCAL FILE: the `makepkg` job runs in the same
# workflow that produces the tarball, before it is attached to the Release. A
# PKGBUILD pointing at a URL that does not exist yet cannot be built, and a job
# that only checks the syntax of a PKGBUILD is a job that has never packaged
# anything. The only line that differs between the two is `source=`.
#
# Norm: docs/ARCHITECTURE.md §15 · docs/decisions.md ADR-023
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DIR="$ROOT/packaging/aur/notes-bin"
VERSION="${1:?usage: gen-pkgbuild.sh <version> <tarball|--release>}"
SRC="${2:?usage: gen-pkgbuild.sh <version> <tarball|--release>}"
NAME="notes-$VERSION-x86_64-linux.tar.gz"

if [ "$SRC" = "--release" ]; then
  source_line="$NAME::https://github.com/samirhvbr/notes/releases/download/$VERSION/$NAME"
  sha="${SHA256:-}"
  [ -n "$sha" ] || { echo "gen-pkgbuild.sh: --release needs SHA256 in the environment" >&2; exit 1; }
else
  [ -f "$SRC" ] || { echo "gen-pkgbuild.sh: no such tarball: $SRC" >&2; exit 1; }
  cp "$SRC" "$DIR/$NAME"
  # A bare filename in `source=` is a local file to makepkg, which is what makes
  # the CI job a real build rather than a syntax check.
  source_line="$NAME"
  sha="$(sha256sum "$DIR/$NAME" | awk '{print $1}')"
fi

sed -e "s|@VERSION@|$VERSION|g" \
    -e "s|@SOURCE@|$source_line|g" \
    -e "s|@SHA256@|$sha|g" \
    "$DIR/PKGBUILD.in" > "$DIR/PKGBUILD"

echo "$DIR/PKGBUILD"
