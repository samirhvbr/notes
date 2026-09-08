#!/usr/bin/env bash
# Write `version.md`'s version into the places a build reads it from.
#
# WHY: `version.md` is the single source of the version (docs/versioning.md,
# ADR-011), and it is what `tools/release.sh` names the git tag and the GitHub
# Release after. A `.deb` attached to Release `0.11.1` that calls itself `0.1.0`
# is a package nobody can match to the code that produced it, and
# `tauri.conf.json` holds a second copy of the number precisely where that is
# easiest to get wrong.
#
# So the number is not maintained in two places: it is stamped, in CI, from the
# one file that has it. **What is committed is `0.0.0`**, which is valid semver
# and unmistakably not a release — a stale real number in the tree is the thing
# that misleads. `tools/check.sh` fails if the committed value is anything else,
# so a stamped tree cannot be committed by accident.
#
#   tools/stamp-version.sh            # stamp version.md's version
#   tools/stamp-version.sh 1.2.3      # stamp an explicit one
#
# Norm: docs/versioning.md · docs/decisions.md ADR-035
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONF="$ROOT/apps/notes-app/src-tauri/tauri.conf.json"

version="${1:-}"
if [ -z "$version" ]; then
  # The first semver in version.md, which is the rule release.sh already applies
  # to the two shapes that live in this fleet.
  version="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' "$ROOT/version.md" | head -1)"
fi
[ -n "$version" ] || { echo "stamp-version.sh: no version found" >&2; exit 1; }

python3 - "$CONF" "$version" <<'PY'
import json, sys
path, version = sys.argv[1], sys.argv[2]
with open(path) as f:
    conf = json.load(f)
conf["version"] = version
with open(path, "w") as f:
    json.dump(conf, f, indent=2)
    f.write("\n")
PY

echo "$version"
