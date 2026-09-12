#!/usr/bin/env bash
# build-local.sh — the macOS and Linux release pipeline for Tura Notes, run locally.
#
# THIS SCRIPT IS THE macOS PIPELINE, AND THAT IS DELIBERATE. `build.yml` builds
# Linux in CI and carries macOS behind `if: false` (ADR-024): the missing piece
# was never engineering, it was a Developer ID certificate, and that certificate
# lives in a keychain on a Mac rather than in a repository secret. So the Mac
# that has it is the machine that packages, signs, notarises and publishes —
# the same arrangement `shvia-desktop/build-local.sh` already uses across the
# fleet, and the reason this file reads like that one (ADR-070).
#
# Linux: .deb + .AppImage by default; --bundles deb,appimage,rpm selects targets.
# Run on Linux with --help for prerequisites and platform-specific options.
#
# USAGE (from the repository root):
#   ./build-local.sh                  # build, sign, notarise, staple
#   ./build-local.sh --no-sign        # test build: NOT signed, NOT publishable
#   ./build-local.sh --skip-npm-ci    # dependencies already installed
#   ./build-local.sh --skip-git-pull  # build this checkout, do not sync first
#   ./build-local.sh --force          # rebuild even if this version is on disk
#   ./build-local.sh --publish        # upload the DMG to samirhv.com.br
#   ./build-local.sh --publish --dest user@host  # publish somewhere else
#
# WHAT IT PRODUCES: `target/release/bundle/dmg/Tura Notes_<version>_<arch>.dmg`,
# a `.sha256` sidecar next to it, and — when a Developer ID is available — a
# notarisation ticket stapled into the image so it opens offline with no prompt.
#
# THE VERSION IS STAMPED, NOT COMMITTED. `version.md` is the single authority
# (ADR-011, ADR-035); `tools/stamp-version.sh` writes it into
# `tauri.conf.json` for the length of the build and the committed `0.0.0`
# placeholder is restored on exit, including on failure.
#
# SIGNING (macOS). Without a signature, a DMG that has been downloaded or
# AirDropped carries the quarantine attribute, and Gatekeeper offers to MOVE IT
# TO THE TRASH — the user is taught that the warning is noise, which is the
# exact lesson ADR-024 exists to avoid teaching. This script finds the
# `Developer ID Application` certificate in the keychain and exports
# `APPLE_SIGNING_IDENTITY`, so `tauri build` signs the `.app` with the hardened
# runtime. If a notarisation credential is also present, it exports
# `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` and `tauri build` notarises and
# staples on its own.
#
#   The app-specific password lives in the KEYCHAIN and never in the repository:
#     security add-generic-password -U -s tura-notarize -a YOUR_APPLE_ID -w
#   (it prompts for the password, hidden; generate one at appleid.apple.com ›
#   App-Specific Passwords).
#
#   `shvia-notarize` is accepted as a fallback, because the certificate in this
#   keychain is the same Apple team and forcing a second copy of one password
#   under a second name only creates a way for the two to drift apart.
#
#   No certificate → the build still runs and says, loudly, that it is unsigned.
#   Certificate but no notarisation credential → signed, not notarised, and it
#   says that too. Neither case is silent: "was that build signed?" must never
#   be a question you answer by inspecting the artefact afterwards.
#
# PUBLISHING (--publish). samirhv.com.br serves downloads from a PRIVATE disk
# through a counting endpoint (`/d/{file}`), so publishing is not a copy into a
# web root — the file has to be ingested by the application, which hashes it,
# records its size and creates the `ProjectFile` row. That is exactly what
# `php artisan files:add` does, so the upload is two steps over one connection:
#
#   1. `scp` the DMG (and its `.sha256`) to a staging directory on the server;
#   2. `ssh` a single `php artisan files:add … --project=tura-notes` call.
#
# Re-publishing the same filename UPDATES the existing row and keeps its
# download counter (FileIngestService), so a re-run after a partial upload is
# safe and does not duplicate the file on the downloads page.
#
# AFTER uploading, the script reads the sha256 BACK from the server and compares
# it to the local one. A truncated `scp` leaves a file that exists, that the
# page happily links, and that fails only in the user's browser — the failure
# has to be found here, not there.
#
# GIT PULL (fleet default): the script runs `git pull --ff-only` before anything
# else so an old checkout is not packaged by accident. It never fails the build
# — offline, dirty tree or a diverged branch only produce a warning. Skip it
# with --skip-git-pull.
#
# REUSING A BUILD: if a DMG for this exact version is already on disk, its
# sha256 still matches the recorded sidecar, and no source is newer than it, the
# script skips straight to publishing. Forgetting `--publish` must not cost a
# full rebuild to upload a file that already exists. The freshness check is what
# makes the shortcut safe: editing code without bumping the version would
# otherwise publish an old binary under a new version number, signed, silently.
# Override with --force.
#
# Norm: docs/runbook.md §4 · docs/decisions.md ADR-011, ADR-024, ADR-035, ADR-070
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

# Dispatch before macOS option parsing, keychain access or platform checks.
if [ "$(uname -s)" = Linux ]; then
  exec bash "$ROOT/tools/build-linux.sh" "$@"
fi

# ── Clock: total wall time, and time per step ────────────────────────────────
# The "built in Xs" printed by Vite and cargo covers one internal stage. What is
# worth comparing between machines and between releases is the whole script, so
# every step is timed and the table is printed even when the build aborts.
SECONDS=0
_PH_NAMES=(); _PH_TIMES=(); _PH_CUR=""; _PH_START=0

_fmt() {  # $1 = seconds -> "1h 02m 03s" / "4m 05s" / "37s"
  local t=$1
  if   [ "$t" -ge 3600 ]; then printf '%dh %02dm %02ds' $((t/3600)) $(((t%3600)/60)) $((t%60))
  elif [ "$t" -ge 60 ];   then printf '%dm %02ds' $((t/60)) $((t%60))
  else                         printf '%ds' "$t"; fi
}

step() {  # close the previous step, open a new one, show the running clock
  local now=$SECONDS
  if [ -n "$_PH_CUR" ]; then
    _PH_NAMES+=("$_PH_CUR"); _PH_TIMES+=($((now - _PH_START)))
  elif [ "$now" -gt 0 ]; then
    _PH_NAMES+=("preparation"); _PH_TIMES+=("$now")
  fi
  _PH_CUR="$1"; _PH_START=$now
  echo "==> [$(_fmt "$now")] $1"
}

_summary() {
  if [ -n "$_PH_CUR" ]; then
    _PH_NAMES+=("$_PH_CUR"); _PH_TIMES+=($((SECONDS - _PH_START))); _PH_CUR=""
  fi
  echo ""
  echo "⏱  time per step:"
  local i
  for i in "${!_PH_NAMES[@]}"; do
    printf '     %8s  %s\n' "$(_fmt "${_PH_TIMES[$i]}")" "${_PH_NAMES[$i]}"
  done
  echo "     ────────"
  printf '     %8s  TOTAL\n' "$(_fmt "$SECONDS")"
}

usage() { awk 'NR>1{ if($0=="set -euo pipefail") exit; sub(/^# ?/,""); print }' "$0"; }

_sha256() {
  if   command -v shasum   >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"   | awk '{print $1}'
  else echo ""; fi
}

# ── Options ──────────────────────────────────────────────────────────────────
SKIP_NPM_CI=0
SKIP_GIT_PULL=0
NO_SIGN=0
FORCE_BUILD=0
PUBLISH=0

# Destination and public base are documented constants, overridable by
# environment or flag. Neither is a secret: the host is reachable only over the
# private network and the base is the address the downloads page already uses.
# The scp password is never a variable and never a file — scp asks for it, or
# `ssh-copy-id <host>` once makes it stop asking.
PUBLISH_HOST="${TURA_PUBLISH_HOST:-b3sys@100.64.100.242}"
PUBLISH_STAGE="${TURA_PUBLISH_STAGE:-/tmp}"
PUBLISH_APP="${TURA_PUBLISH_APP:-/srv/www/samirhv.com.br/samirhv}"
PUBLISH_SLUG="${TURA_PUBLISH_SLUG:-tura-notes}"
PUBLIC_BASE="${TURA_PUBLIC_BASE:-https://samirhv.com.br}"

while [ $# -gt 0 ]; do
  case "$1" in
    --skip-npm-ci)   SKIP_NPM_CI=1 ;;
    --skip-git-pull) SKIP_GIT_PULL=1 ;;
    --no-sign)       NO_SIGN=1 ;;
    --force|-f)      FORCE_BUILD=1 ;;
    --publish)       PUBLISH=1 ;;
    --dest)          shift; PUBLISH_HOST="${1:-}" ;;
    --base-url)      shift; PUBLIC_BASE="${1:-}" ;;
    -h|--help)       usage; exit 0 ;;
    *) echo "build-local.sh: unknown option: $1 (use --help)" >&2; exit 2 ;;
  esac
  shift
done

if [ "$(uname -s)" != "Darwin" ]; then
  echo "build-local.sh: supported build hosts are macOS and Linux." >&2
  exit 1
fi

# One EXIT trap, set once, doing both things it has to do — and reading `$?`
# FIRST. Chaining a second trap over the first was the obvious shape and the
# wrong one: whatever ran before the status was read would overwrite it, and a
# failed build would report the exit code of the cleanup instead of its own.
CONFIG_BACKUP=""
CONFIG_PATH=""
_on_exit() {
  local code=$?
  if [ -n "$CONFIG_BACKUP" ] && [ -f "$CONFIG_BACKUP" ]; then
    cp "$CONFIG_BACKUP" "$CONFIG_PATH"
    rm -f "$CONFIG_BACKUP"
    CONFIG_BACKUP=""
  fi
  if [ "$code" -ne 0 ]; then
    echo "" >&2
    echo "❌ build aborted after $(_fmt "$SECONDS") (exit $code)" >&2
  fi
}
# INT and TERM as well as EXIT: a Ctrl-C in the middle of a ten-minute
# notarisation is the ordinary way this script ends, and the shell does not run
# an EXIT trap when it dies on a signal. Without these two, the stamped
# `tauri.conf.json` stayed in the tree — and `tools/check.sh` rejects a stamped
# tree, so an interrupted build broke the next commit instead of just itself.
trap _on_exit EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

echo "==> Tura Notes — local macOS build"

# ── Per-machine credentials, once instead of once per build ──────────────────
# Without this, every release means re-exporting the same variables by hand, and
# forgetting one is something you find out at the end of a long build. Two
# locations are accepted, first one that exists wins:
#
#   1. ./signing.env             — inside the repository, gitignored.
#   2. ~/.config/tura-notes/build.env — outside it. Survives a fresh clone, a
#      `git clean -xdf`, and deleting the tree. Same address shape the rest of
#      the fleet uses (~/.config/shvia/build.env, ~/.config/sshvterm/build.env),
#      because the release machine is the same one and one habit is less to
#      remember than three.
#
# A third: $TURA_BUILD_ENV. It announces what it loaded and from where, on
# purpose — "did this build come out signed?" must not depend on an invisible
# file, and the first line of output should already say where the answer is.
CREDS_FILE=""
for _c in "${TURA_BUILD_ENV:-}" "./signing.env" "$HOME/.config/tura-notes/build.env"; do
  [ -n "$_c" ] && [ -f "$_c" ] && { CREDS_FILE="$_c"; break; }
done
if [ -n "$CREDS_FILE" ]; then
  # shellcheck source=/dev/null
  . "$CREDS_FILE"
  echo "    credentials: $CREDS_FILE loaded"
fi

# ── Preflight: check the toolchain before the slow steps ─────────────────────
# Fails in under a second with an actionable message instead of a cryptic
# `cargo metadata: No such file or directory` five seconds in, and collects
# everything that is missing in one pass rather than one per re-run.
preflight() {
  local missing=()

  command -v node >/dev/null 2>&1 || missing+=(
    "Node.js not found. Install Node 20+ (https://nodejs.org, 'brew install node' or nvm)."
  )
  command -v npm >/dev/null 2>&1 || missing+=(
    "npm not found (it ships with Node)."
  )

  # rustup installs into ~/.cargo/bin, and a terminal opened before that has no
  # such entry in PATH. Recovering it here beats telling someone to reopen a
  # shell they have work in.
  if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
    echo "    (cargo found in ~/.cargo/bin — added to PATH for this run)"
  fi
  if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
    missing+=(
"Rust (cargo) not found — it is what Tauri compiles with.
       Install:  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
       Then:     source \"\$HOME/.cargo/env\"   (or reopen the terminal)"
    )
  fi

  xcode-select -p >/dev/null 2>&1 || missing+=(
"Xcode Command Line Tools missing (macOS clang and linker).
       Install:  xcode-select --install"
  )

  if [ "${#missing[@]}" -gt 0 ]; then
    echo "" >&2
    echo "❌ missing prerequisites — fix these and run again:" >&2
    echo "" >&2
    local m
    for m in "${missing[@]}"; do echo "   • $m" >&2; echo "" >&2; done
    exit 1
  fi
}

# ── git pull before the build (fleet default) ────────────────────────────────
# Syncs before any other step so old code is not packaged by accident.
# Fast-forward only — it never creates a merge — and it never fails the build:
# offline, local changes or a diverged branch warn and carry on with what is
# checked out. A build that refuses to run because the network is down is worse
# than a build that tells you it used the local tree.
git_sync() {
  if [ "$SKIP_GIT_PULL" -eq 1 ]; then
    echo "    (skipped: --skip-git-pull)"
    return 0
  fi
  if ! git rev-parse --git-dir >/dev/null 2>&1; then
    echo "    (not a git checkout — nothing to sync)"
    return 0
  fi
  if git pull --ff-only 2>&1 | sed 's/^/    /'; then
    return 0
  fi
  echo "    ⚠️  could not fast-forward — building the local checkout as it is."
  return 0
}

# ── macOS: signing (Developer ID) + notarisation (app-specific password) ─────
# See the header. Both halves are independent: a certificate with no
# notarisation credential still produces a signed build, and the script says so
# rather than pretending the artefact is releasable.
NOTARY_SERVICES=("tura-notarize" "shvia-notarize")
SIGN_ENABLED=0
NOTARIZE_ENABLED=0

setup_macos_signing() {
  if [ "$NO_SIGN" -eq 1 ]; then
    echo "    (--no-sign: test build, NOT signed and NOT notarised)"
    return 0
  fi

  # 1) Signing identity: the first "Developer ID Application" in the keychain,
  #    unless APPLE_SIGNING_IDENTITY already came from the environment.
  if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
    APPLE_SIGNING_IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null \
      | awk -F'"' '/Developer ID Application/{print $2; exit}' || true)"
  fi
  if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
    echo "    ⚠️  no 'Developer ID Application' certificate in the keychain — THE BUILD WILL BE UNSIGNED."
    echo "        (macOS will offer to move the downloaded app to the Trash; do not publish it — ADR-024)"
    return 0
  fi
  export APPLE_SIGNING_IDENTITY
  SIGN_ENABLED=1
  echo "    ✔ signing: $APPLE_SIGNING_IDENTITY"

  # Team ID: the (XXXXXXXXXX) at the end of the identity, unless already set.
  if [ -z "${APPLE_TEAM_ID:-}" ]; then
    APPLE_TEAM_ID="$(printf '%s' "$APPLE_SIGNING_IDENTITY" \
      | sed -n 's/.*(\([A-Z0-9]\{10\}\))$/\1/p')"
  fi

  # 2) Notarisation credential from the keychain, unless already in the
  #    environment. `tura-notarize` first, `shvia-notarize` as the documented
  #    fallback for the same Apple team.
  local svc
  for svc in "${NOTARY_SERVICES[@]}"; do
    [ -n "${APPLE_PASSWORD:-}" ] && break
    APPLE_PASSWORD="$(security find-generic-password -s "$svc" -w 2>/dev/null || true)"
    if [ -n "${APPLE_PASSWORD:-}" ] && [ -z "${APPLE_ID:-}" ]; then
      APPLE_ID="$(security find-generic-password -s "$svc" 2>/dev/null \
        | awk -F'"' '/"acct"/{print $4}' || true)"
      NOTARY_USED="$svc"
    fi
  done

  if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
    export APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID
    NOTARIZE_ENABLED=1
    echo "    ✔ notarisation: $APPLE_ID (team $APPLE_TEAM_ID, keychain ${NOTARY_USED:-env})"
    echo "      (notarisation uploads the app to Apple and WAITS — this can take minutes)"
  else
    echo "    ⚠️  will SIGN but NOT notarise (no credential). Store the app password once:"
    echo "        security add-generic-password -U -s tura-notarize -a YOUR_APPLE_ID -w"
    echo "        (unnotarised, the app opens but still needs approval in Settings › Privacy)"
  fi
}

# macOS: detach .dmg images from THIS repository left mounted by an earlier run.
# `bundle_dmg.sh` creates a temporary rw.*.dmg, mounts it, arranges the window
# over AppleScript and detaches. If that process is killed midway the image stays
# attached, and the next build runs `tell disk "Tura Notes"` with two volumes of
# that name mounted — ambiguous, so it errors, and Tauri reports only "failed to
# run bundle_dmg.sh". Filtered by image path inside our own bundle directory, so
# a DMG the user mounted from somewhere else is never ejected.
detach_stale_build_images() {
  command -v hdiutil >/dev/null 2>&1 || return 0
  local bundle_abs devs d
  bundle_abs="$ROOT/target/release/bundle"
  devs="$(hdiutil info 2>/dev/null | awk -v b="$bundle_abs" '
    /^image-path/            { p = (index($0, b) > 0) }
    p && /^\/dev\/disk[0-9]/ { print $1; p = 0 }
  ' || true)"
  for d in $devs; do
    echo "    image left mounted by an earlier build — ejecting $d"
    hdiutil detach "$d" >/dev/null 2>&1 || hdiutil detach -force "$d" >/dev/null 2>&1 || true
  done
}

# ── Is there a usable build of this version already on disk? ─────────────────
# "The file exists" is not proof — see the header. The test is: a DMG whose name
# carries this version, a `.sha256` sidecar that still matches its contents, and
# no source file newer than the DMG. The last clause is the one that matters:
# without it, editing code without bumping the version passes the first two and
# publishes an old binary as the new version, signed, with nothing to notice.
REUSE_DMG=""
REUSE_REASON=""
can_reuse_build() {
  local version="$1" dmg
  [ "$FORCE_BUILD" -eq 1 ] && { REUSE_REASON="--force"; return 1; }

  dmg="$(find "$ROOT/target/release/bundle/dmg" -maxdepth 1 -type f \
         -name "*_${version}_*.dmg" -print -quit 2>/dev/null || true)"
  [ -z "$dmg" ] && { REUSE_REASON="no DMG for $version on disk"; return 1; }

  [ -f "$dmg.sha256" ] || { REUSE_REASON="no .sha256 sidecar — cannot prove what that DMG is"; return 1; }
  local recorded current
  recorded="$(awk '{print $1; exit}' "$dmg.sha256")"
  current="$(_sha256 "$dmg")"
  [ -n "$current" ] && [ "$current" = "$recorded" ] || {
    REUSE_REASON="the DMG no longer matches its recorded sha256"; return 1; }

  local newer
  newer="$(find apps/notes-app/src apps/notes-app/src-tauri/src crates server \
                apps/notes-app/package.json apps/notes-app/index.html \
                apps/notes-app/vite.config.ts apps/notes-app/src-tauri/Cargo.toml \
                Cargo.toml Cargo.lock version.md \
             -type f -newer "$dmg" -print -quit 2>/dev/null || true)"
  [ -n "$newer" ] && { REUSE_REASON="a source file is newer than the build: $newer"; return 1; }

  REUSE_DMG="$dmg"
  return 0
}

# ── Post-build proof that Gatekeeper will accept it ──────────────────────────
# `--bundles dmg` makes Tauri DELETE the `.app` after folding it into the image
# ("Cleaning …/Tura Notes.app"), so there is usually no standalone `.app` left to
# inspect. The `.app` inside the DMG was already notarised and stapled before
# that cleanup; what can still be missing is a ticket on the DMG itself, which is
# what a user actually downloads — so that block runs unconditionally.
verify_macos_signature() {
  local dmg="$1" app
  if [ "$SIGN_ENABLED" -ne 1 ]; then
    echo "    (unsigned build — nothing to verify)"
    return 0
  fi

  app="$(find "$ROOT/target/release/bundle/macos" -maxdepth 1 -name '*.app' 2>/dev/null | head -1 || true)"
  if [ -n "$app" ]; then
    echo "  • codesign --verify (deep, strict):"
    if codesign --verify --deep --strict --verbose=2 "$app" >/tmp/_tura_cs.txt 2>&1; then
      echo "      ✔ signature intact"
    else
      echo "      ❌ invalid signature:"; sed 's/^/        /' /tmp/_tura_cs.txt
    fi

    echo "  • authority + hardened runtime:"
    codesign -dvvv "$app" 2>&1 \
      | grep -E 'Authority=|TeamIdentifier=|Identifier=|flags=' | sed 's/^/      /' || true

    echo "  • Gatekeeper (spctl assess):"
    spctl -a -t exec -vvv "$app" 2>&1 | sed 's/^/      /' || true
  else
    echo "    (no standalone .app — DMG-only build; Tauri already cleaned it up)"
  fi

  [ -n "$dmg" ] || return 0
  if xcrun stapler validate "$dmg" >/dev/null 2>&1; then
    echo "      ✔ .dmg stapled (opens offline, no prompt)"
  elif [ "$NOTARIZE_ENABLED" -eq 1 ]; then
    echo "      • .dmg not stapled yet — notarising the image itself (submit + staple)…"
    if xcrun notarytool submit "$dmg" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
           --team-id "$APPLE_TEAM_ID" --wait 2>&1 | sed 's/^/        /' \
       && xcrun stapler staple "$dmg" 2>&1 | sed 's/^/        /'; then
      echo "      ✔ .dmg notarised + stapled"
    else
      echo "      ⚠️  could not notarise the .dmg — but the .app inside it is already"
      echo "          notarised and stapled, so distributing the .dmg still works."
    fi
  else
    echo "      ⚠️  .dmg NOT stapled — notarisation did not run. Do not publish (ADR-024)."
  fi
}

# ── Publishing to samirhv.com.br ─────────────────────────────────────────────
# See the header for why this is an ingest and not a copy. Refuses to publish an
# unsigned or unstapled image: the whole reason macOS artefacts were withheld
# until now is that an unsigned one teaches the user to click past Gatekeeper.
publish_release() {
  local dmg="$1" version="$2"
  local name sum remote

  name="$(basename "$dmg")"
  sum="$(_sha256 "$dmg")"

  if [ "$SIGN_ENABLED" -ne 1 ]; then
    echo "  ✗ refusing to publish an UNSIGNED build (ADR-024)." >&2
    echo "    Run without --no-sign, on a machine whose keychain holds the Developer ID." >&2
    return 1
  fi
  if ! xcrun stapler validate "$dmg" >/dev/null 2>&1; then
    echo "  ✗ refusing to publish a DMG with no notarisation ticket (ADR-024)." >&2
    echo "    Gatekeeper would still warn about it on a machine that is offline." >&2
    return 1
  fi

  echo "    host:    $PUBLISH_HOST"
  echo "    project: $PUBLISH_SLUG ($version)"
  echo "    file:    $name"
  echo "    sha256:  $sum"
  echo "    (one password — a single scp connection, then a single ssh call)"

  # One scp: one connection, one password prompt.
  scp "$dmg" "$dmg.sha256" "$PUBLISH_HOST:$PUBLISH_STAGE/"

  # Ingest. `files:add` hashes the file, writes it to the private downloads disk
  # under the project folder and creates or UPDATES the ProjectFile row — same
  # filename updates in place and keeps the download counter, so a re-run is
  # safe. Artisan runs as www-data because it writes into storage/.
  ssh "$PUBLISH_HOST" "cd '$PUBLISH_APP' && sudo -u www-data php artisan files:add \
      '$PUBLISH_STAGE/$name' --project='$PUBLISH_SLUG' --version='$version' \
      --label='Tura Notes $version — macOS (Apple silicon)'" 2>&1 | sed 's/^/      /'

  # ── Read the hash back from the server ──────────────────────────────────────
  # A truncated scp leaves a file that exists and that the page links happily;
  # the failure then belongs to whoever downloads it. It has to be found here.
  step "[publish] verify the uploaded file on the server"
  remote="$(ssh "$PUBLISH_HOST" "shasum -a 256 '$PUBLISH_STAGE/$name' 2>/dev/null \
            || sha256sum '$PUBLISH_STAGE/$name' 2>/dev/null" | awk '{print $1; exit}')" || remote=""

  if [ -n "$remote" ] && [ "$remote" = "$sum" ]; then
    echo "    ✅ uploaded intact (${sum:0:16}…)"
  else
    echo "    ✗ the uploaded file does NOT match" >&2
    echo "      expected: $sum" >&2
    echo "      got:      ${remote:-<could not read it back>}" >&2
    return 1
  fi

  # The staging copy has been ingested into the downloads disk; leaving a second
  # copy of a 20 MB image in /tmp on every release is litter, not a backup.
  ssh "$PUBLISH_HOST" "rm -f '$PUBLISH_STAGE/$name' '$PUBLISH_STAGE/$name.sha256'" || true

  echo ""
  echo "    Published. It is listed at:"
  echo "      $PUBLIC_BASE/p/$PUBLISH_SLUG"
  echo "      $PUBLIC_BASE/downloads"
}

# ── Pipeline ─────────────────────────────────────────────────────────────────
step "[git] sync with the remote (git pull --ff-only)"
git_sync

version="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' version.md | head -1)"
[ -n "$version" ] || { echo "build-local.sh: no version in version.md" >&2; exit 1; }
echo "    version: $version"

step "[reuse] is there a build of this version on disk?"
if can_reuse_build "$version"; then
  echo "    ✅ yes — $(basename "$REUSE_DMG"), sha256 verified, no newer source."
  if _dt="$(date -r "$REUSE_DMG" '+%d/%m %H:%M' 2>/dev/null)"; then
    echo "       built at $_dt. Skipping npm ci and tauri build."
  fi
  echo "       To rebuild anyway: --force (or delete target/release/bundle/dmg)."
  dmg="$REUSE_DMG"
  # Reused builds were signed by the run that produced them; record that so the
  # publish gate reads the artefact rather than this run's keychain lookup.
  if codesign --verify --strict "$dmg" >/dev/null 2>&1 \
     || xcrun stapler validate "$dmg" >/dev/null 2>&1; then
    SIGN_ENABLED=1
  fi
else
  echo "    no — $REUSE_REASON"

  step "[prerequisites] check the toolchain (Node, Rust, Xcode CLT)"
  preflight

  step "[signing] Developer ID + notarisation credential"
  setup_macos_signing

  # The committed placeholder is restored on EXIT, including on failure: a
  # stamped tauri.conf.json in the tree is what tools/check.sh rejects, and
  # leaving one behind after a failed build turns one problem into two.
  # Handing the two paths to the trap that is already installed, rather than
  # installing a second one — see _on_exit.
  CONFIG_PATH="apps/notes-app/src-tauri/tauri.conf.json"
  CONFIG_BACKUP="$(mktemp)"
  cp "$CONFIG_PATH" "$CONFIG_BACKUP"

  step "[1/3] stamp the version from version.md"
  tools/stamp-version.sh >/dev/null
  echo "    tauri.conf.json → $version (0.0.0 restored on exit)"

  step "[2/3] frontend dependencies (npm ci)"
  if [ "$SKIP_NPM_CI" -eq 1 ]; then
    echo "    (skipped: --skip-npm-ci)"
  else
    (cd apps/notes-app && npm ci)
  fi

  step "[3/3] tauri build (compile, bundle, sign, notarise, staple)"
  detach_stale_build_images
  (cd apps/notes-app && npm run tauri build -- --bundles dmg)

  artifact_dir="target/release/bundle/dmg"
  dmg="$(find "$artifact_dir" -maxdepth 1 -type f -name "*_${version}_*.dmg" -print -quit)"
  if [ -z "$dmg" ]; then
    echo "build-local.sh: Tauri finished without a DMG for $version in $artifact_dir." >&2
    exit 1
  fi

  step "[verify] codesign / Gatekeeper / notarisation ticket"
  verify_macos_signature "$dmg"

  # ── The sidecar is written LAST, and that ordering is the whole point ───────
  # `xcrun stapler staple` REWRITES the image to embed the notarisation ticket,
  # so a hash taken before that step describes a file that no longer exists.
  # Written first, it made `can_reuse_build` reject every build it had just
  # produced — a harmless symptom of a harmful bug, because the same number is
  # what a user checks the download against, and it would never have matched.
  _sha256 "$dmg" | awk -v n="$(basename "$dmg")" '{print $1"  "n}' > "$dmg.sha256"
fi

if [ "$PUBLISH" -eq 1 ]; then
  step "[publish] upload to $PUBLIC_BASE"
  publish_release "$dmg" "$version"
fi

step "done"
echo ""
echo "Tura Notes $version"
echo "  $dmg"
echo "  $(cat "$dmg.sha256" 2>/dev/null || echo '(no sha256 sidecar)')"
if [ "$SIGN_ENABLED" -eq 1 ]; then
  if xcrun stapler validate "$dmg" >/dev/null 2>&1; then
    echo "  signed + notarised + stapled — distributable."
  else
    echo "  signed, NOT notarised — opens with an approval prompt. Do not publish (ADR-024)."
  fi
else
  echo "  UNSIGNED — local verification only. Do not distribute (ADR-024)."
fi
# `if`, not `[ … ] && echo …`: this is the last statement before the summary, and
# an AND-list whose test is false exits non-zero. Here the false case is the
# normal one — a published build — so the script would report a failure it did
# not have. Spelling it as an `if` removes the question entirely.
if [ "$PUBLISH" -eq 0 ]; then
  echo "  (not published — add --publish to upload it)"
fi
_summary
