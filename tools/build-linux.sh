#!/usr/bin/env bash
# Native Linux packaging; invoked by build-local.sh and deploy.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
bundles=deb,appimage
skip_npm=0; skip_pull=0; publish=0; force=0
host="${TURA_PUBLISH_HOST:-b3sys@100.64.100.242}"
stage="${TURA_PUBLISH_STAGE:-/tmp}"
app="${TURA_PUBLISH_APP:-/srv/www/samirhv.com.br/samirhv}"
slug="${TURA_PUBLISH_SLUG:-tura-notes}"
base="${TURA_PUBLIC_BASE:-https://samirhv.com.br}"
usage() {
  cat <<'HELP'
Usage: ./build-local.sh [--bundles deb,appimage] [--skip-npm-ci]
                       [--skip-git-pull] [--publish] [--dest user@host]
Linux builds .deb and .AppImage by default. Optional targets: deb,appimage,rpm.
Existing builds are reused after version, source and checksum verification.
--force rebuilds even when reusable packages exist.
--no-sign is accepted for local builds, but cannot be combined with --publish.
--base-url URL changes the download-page URL printed after publication.
Requires Node 22.22.2+, 24.15+ or 26+, Rust, Python 3 and the Tauri Linux dependencies.
Debian/Ubuntu: build-essential pkg-config libwebkit2gtk-4.1-dev libssl-dev
  libayatana-appindicator3-dev librsvg2-dev patchelf file xdg-utils
Arch: base-devel pkgconf webkit2gtk-4.1 openssl libayatana-appindicator
  librsvg patchelf file xdg-utils
Install these with your distribution's package manager before building.
HELP
}
no_sign=0
while [ $# -gt 0 ]; do
  case "$1" in
    --bundles|--dest|--base-url)
      option="$1"; shift
      [ $# -gt 0 ] && [ -n "$1" ] || { echo "Missing value for $option" >&2; exit 2; }
      case "$option" in --bundles) bundles="$1";; --dest) host="$1";; --base-url) base="$1";; esac ;;
    --skip-npm-ci) skip_npm=1;;
    --skip-git-pull) skip_pull=1;;
    --publish) publish=1;;
    --no-sign) no_sign=1;;
    --force|-f) force=1;;
    --help|-h) usage; exit 0;;
    *) echo "Unknown option: $1" >&2; exit 2;;
  esac
  shift
done
[ "$(uname -s)" = Linux ] || { echo 'Linux builds must run on Linux.' >&2; exit 1; }
[ "$publish:$no_sign" != 1:1 ] || { echo 'Cannot publish a --no-sign test build.' >&2; exit 2; }
IFS=, read -r -a targets <<< "$bundles"
[[ "$bundles" != ,* && "$bundles" != *, && "$bundles" != *,,* ]] || exit 2
for target in "${targets[@]}"; do
  case "$target" in deb|appimage|rpm) :;; *) echo "Unsupported Linux bundle: $target" >&2; exit 2;; esac
done
if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
  echo 'Native Linux builds require CARGO_BUILD_TARGET to be unset.' >&2; exit 2
fi
if [ "$skip_pull" -eq 0 ]; then git pull --ff-only; fi
if ! command -v cargo >/dev/null && [ -x "$HOME/.cargo/bin/cargo" ]; then export PATH="$HOME/.cargo/bin:$PATH"; fi
if [ "$publish" -eq 1 ]; then
  command -v scp >/dev/null; command -v ssh >/dev/null
  [[ "$stage" =~ ^/[a-zA-Z0-9_./-]+$ ]] || { echo 'Use an absolute publish staging path without spaces or shell characters' >&2; exit 2; }
  [[ "$host" != -* && "$host" =~ ^[a-zA-Z0-9_.@-]+$ ]] || { echo 'Invalid publish host' >&2; exit 2; }
fi
# Check completed packages before requiring the compilation toolchain.
for tool in python3 rustc sha256sum; do
  command -v "$tool" >/dev/null || { echo "Missing prerequisite: $tool" >&2; exit 1; }
done
version="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' version.md | head -1)"
host_triple="$(rustc -vV | sed -n 's/^host: //p')"
[ -n "$version" ] && [ -n "$host_triple" ] || exit 1
output="$ROOT/target/local-linux/$host_triple"
export CARGO_TARGET_DIR="$output"
source_hash="$(python3 tools/linux-build-cache.py fingerprint)"
artifacts=(); pending=()
for target in "${targets[@]}"; do
  directory="$output/release/bundle/$target"
  listing="$(mktemp)"
  if [ "$force" -eq 0 ] && python3 tools/linux-build-cache.py check "$directory" "$version" "$host_triple" "$source_hash" "$no_sign" > "$listing"; then
    while IFS= read -r -d '' artifact; do artifacts+=("$artifact"); done < "$listing"
    echo "Reusing $target for $version: sources and SHA-256 verified."
  else
    pending+=("$target")
  fi
  rm -f "$listing"
done
if [ "${#pending[@]}" -gt 0 ]; then
  for tool in node npm cargo rustc python3 pkg-config cc file patchelf sha256sum; do
    command -v "$tool" >/dev/null || { echo "Missing prerequisite: $tool (see --help)" >&2; exit 1; }
  done
  node -e 'const [m,n,p]=process.versions.node.split(".").map(Number);if(!((m===22&&(n>22||(n===22&&p>=2)))||(m===24&&n>=15)||m>=26)){console.error("Use Node 22.22.2+, 24.15+ or 26+");process.exit(1)}'
  pkg-config --exists gtk+-3.0 webkit2gtk-4.1 openssl librsvg-2.0 || {
    echo 'Missing Tauri development libraries. See --help for distribution packages.' >&2; exit 1;
  }

  config=apps/notes-app/src-tauri/tauri.conf.json
  backup="$(mktemp)"
  cp "$config" "$backup"
  cleanup() { code=$?; if [ -n "$backup" ]; then cp "$backup" "$config"; rm -f "$backup"; fi; exit "$code"; }
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
  for target in "${pending[@]}"; do
    directory="$output/release/bundle/$target"
    mkdir -p "$directory"
    rm -f "$directory/.build.json"
    find "$directory" -maxdepth 1 -type f \( -name '*.deb' -o -name '*.AppImage' -o -name '*.rpm' -o -name '*.sha256' \) -delete
  done
  if [ "$skip_npm" -eq 0 ]; then (cd apps/notes-app && npm ci); fi
  tools/stamp-version.sh >/dev/null
  export APPIMAGE_EXTRACT_AND_RUN=1
  pending_bundles="$(IFS=,; echo "${pending[*]}")"
  (cd apps/notes-app && npm run tauri build -- --bundles "$pending_bundles")
  cp "$backup" "$config"; rm -f "$backup"; backup=""
  [ "$(python3 tools/linux-build-cache.py fingerprint)" = "$source_hash" ] || {
    echo 'Sources changed during the build; retry before publishing.' >&2; exit 1;
  }
  for target in "${pending[@]}"; do
    directory="$output/release/bundle/$target"
    extension="$target"; [ "$target" != appimage ] || extension=AppImage
    built=()
    while IFS= read -r -d '' artifact; do
      built+=("$artifact")
      (cd "$(dirname "$artifact")" && sha256sum "$(basename "$artifact")" > "$(basename "$artifact").sha256")
    done < <(find "$directory" -maxdepth 1 -type f -name "*.$extension" -print0)
    [ "${#built[@]}" -gt 0 ] || { echo "Build produced no $target package" >&2; exit 1; }
    python3 tools/linux-build-cache.py record "$directory" "$version" "$host_triple" "$source_hash" "$no_sign" "${built[@]}"
    artifacts+=("${built[@]}")
  done
else
  echo 'Build already complete; skipping npm ci and compilation.'
fi
# Quote each remote argument for the POSIX shell used by ssh.
quote() { python3 -c 'import shlex,sys; print(shlex.quote(sys.argv[1]))' "$1"; }
for artifact in "${artifacts[@]}"; do
  echo "Tura Notes $version: $artifact"
  if [ "$publish" -eq 1 ]; then
    name="$(basename "$artifact")"
    remote_file="$stage/$name"
    scp "$artifact" "$artifact.sha256" "$host:$stage/"
    expected="$(sha256sum "$artifact" | awk '{print $1}')"
    actual="$(ssh "$host" "sha256sum -- $(quote "$remote_file")" | awk '{print $1}')"
    [ "$actual" = "$expected" ] || { echo 'Upload checksum mismatch; not ingested.' >&2; exit 1; }
    ssh "$host" "cd $(quote "$app") && sudo -u www-data php artisan files:add $(quote "$remote_file") --project=$(quote "$slug") --version=$(quote "$version") --label=$(quote "Tura Notes $version — Linux ($(uname -m))")"
    ssh "$host" "rm -f -- $(quote "$remote_file") $(quote "$remote_file.sha256")"
  fi
done
if [ "$publish" -eq 1 ]; then echo "Published: $base/p/$slug"; fi
