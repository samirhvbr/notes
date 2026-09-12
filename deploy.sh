#!/usr/bin/env bash
# Compatibility entry point. Publication remains explicit with --publish.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "$ROOT/build-local.sh" "$@"
