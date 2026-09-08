#!/usr/bin/env bash
# The browser's blocking script dialogs must not appear in the frontend.
#
# They belong to a browser; the WebView this application ships in is not one,
# and behind one of them a flow is dead without a sound. Six flows of milestone
# 0.1b were, and no core test could have caught it — the 0.1b criteria are
# assertions about `notes-core`, and the UI is where these live.
#
# No exclusions: the file that documents the ban does not spell the tokens.
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

hits=$(grep -rnE '\b(window\.)?(prompt|confirm|alert)\s*\(' \
        apps/notes-app/src --include='*.ts' --include='*.tsx' \
        | grep -vE '\b(askConfirm|confirmLabel)\b' || true)

if [ -n "$hits" ]; then
  echo "blocking script dialogs are banned in the frontend — use src/app/dialog.ts" >&2
  echo "$hits" >&2
  exit 1
fi
echo "no blocking script dialogs"
