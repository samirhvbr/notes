# Runbook — notes

> **Status:** `ACTIVE` · From a clean machine to a running environment, and the
> checklists that gate a release.

<!-- Sections 1–4 are yours to fill in. Sections 5–7 are inherited from the
     fleet standard and are already correct — keep them. -->

## 1. Requirements

_Fill in: runtimes and versions, system packages, accounts and access needed._

## 2. From a clean machine to running

```bash
git clone git@github.com:samirhvbr/tura-notes.git
cd tura-notes
git config core.hooksPath tools/git-hooks   # once per clone — see §5

# install, configure, run — fill this in
cp .env.example .env
```

_Say which of the paths this is: running it with Docker, or developing with the
whole chain. If there are two, say what each one requires and what to do when it
does not come up._

## 3. Configuration

_Fill in: what each variable in `.env.example` means, which ones are required,
and which ones are secrets that never get committed
([security.md §5](security.md#5-secrets-and-configuration))._

## 4. Release

There is no server. "Deploy" here means: a version gets a tag, a GitHub Release,
and packages attached to it.

**It is automatic, and the trigger is `version.md`.** Push a commit that bumps
it and `release.yml` tags the version and publishes the Release from the
CHANGELOG section with the same heading; `build.yml` then builds the artifacts
and attaches them. Nothing below has to be run by hand
([ARCHITECTURE.md §15](ARCHITECTURE.md), ADR-011, ADR-035).

What ships today, and what does not:

| | |
|---|---|
| Linux | `.deb`, AppImage, a tarball, and the AUR `notes-bin` package — all built in CI and attached to the Release. Local deb/AppImage/rpm builds and optional download-service publication are also available; see [local Linux installers](#local-linux-installers-103) |
| macOS | **published, from this repository's own `build-local.sh` rather than from CI**, and served by [samirhv.com.br](https://samirhv.com.br/p/tura-notes) rather than attached to the Release. The certificate that signs it lives in a keychain, not in a repository secret, so the machine that holds it is the machine that packages (ADR-070). The `build.yml` job stays behind `if: false` |
| Windows | **not published.** The job is written in `build.yml` behind `if: false` and carries the list of what is missing; it is an OV certificate rather than code (ADR-024) |

### The macOS release, from the repository root

```bash
./build-local.sh              # build, sign, notarise, staple
./build-local.sh --publish    # and upload it to samirhv.com.br
```

It runs `git pull --ff-only`, installs the app dependencies, stamps the version
from `version.md`, builds the DMG under `target/release/bundle/dmg/`, signs it
with the `Developer ID Application` certificate in the keychain, notarises it,
staples the ticket and writes a `.sha256` beside it. The committed `0.0.0`
configuration placeholder is restored when it exits, including on failure and on
Ctrl-C.

**The `.sha256` is written after stapling, not before.** Stapling rewrites the
image, so a hash taken earlier describes a file that no longer exists — and that
is the number a user checks their download against.

`./build-local.sh --help` prints the whole contract. The flags worth knowing:

| | |
|---|---|
| `--publish` | upload to samirhv.com.br: `scp` to the server, then one `php artisan files:add`, then read the hash back. **Refuses an unsigned or unstapled image** — ADR-024, enforced rather than remembered |
| `--no-sign` | a test build. Not signed, not publishable |
| `--force` | rebuild even when a verified DMG of this version is already on disk |
| `--skip-npm-ci`, `--skip-git-pull` | an installed dependency tree; this checkout as it is |

**Credentials are read, never typed.** The signing identity comes from
`security find-identity`; the notarisation password from the keychain entry
`tura-notarize` (or `shvia-notarize`, the same Apple team). Store it once:

```bash
security add-generic-password -U -s tura-notarize -a YOUR_APPLE_ID -w
```

Per-machine overrides go in `~/.config/tura-notes/build.env` — outside the
tree, so it survives a fresh clone — or `./signing.env`, which is gitignored.
Nothing about signing goes in the repository.

Without a certificate the build still runs and says, in as many words, that its
output is unsigned and must not be distributed (ADR-024).

The same three steps CI runs, in the same order:

```bash
tools/stamp-version.sh                      # version.md → tauri.conf.json
cd apps/notes-app && npm ci
npm run tauri build -- --bundles deb,appimage
cd ../.. && packaging/linux/tarball.sh "$(cat version.md)"
```

**`tools/stamp-version.sh` first, or the bundle calls itself `0.0.0`.** That is
the committed placeholder and CI fails if anything else is committed in its
place (ADR-035) — a local build is not a release, so `0.0.0` is the honest
default.

The Arch package, the way the CI job does it, in a container so nothing is
installed on the machine:

```bash
docker run --rm -v "$PWD:/src:ro" archlinux:latest bash -c '
  pacman -Syu --noconfirm && pacman -S --noconfirm base-devel webkit2gtk-4.1 gtk3
  useradd -m builder && echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers
  cp -r /src /work && chown -R builder /work && cd /work
  v="$(grep -oE "[0-9]+\.[0-9]+\.[0-9]+" version.md | head -1)"
  sudo -u builder packaging/aur/gen-pkgbuild.sh "$v" "dist-release/notes-$v-x86_64-linux.tar.gz"
  cd packaging/aur/notes-bin && sudo -u builder makepkg --noconfirm --syncdeps --cleanbuild
  pacman -U --noconfirm ./*.pkg.tar.zst && ldd /usr/bin/notes | grep "not found" && exit 1
  echo ok'
```

### When it fails halfway

- **A Release exists with no artifacts.** That is `release.yml` green and
  `build.yml` red, and it is the split those two workflows exist to allow. Fix
  the build and re-run `build.yml`; it refuses to upload twice, so a re-run
  after a partial upload is safe.
- **`build.yml` did nothing.** Three reasons, and the job named *what to build*
  says which as a notice: no Release for `version.md`'s version yet; the Release
  already carries its `.SRCINFO`; or the version is a **patch**, which is not
  built (ADR-036).
- **A release has no artifacts.** Expected for any `X.Y.Z` where `Z` is not `0`
  — the Release says so itself. To package one anyway, run the **Build**
  workflow by hand with that version as the input:

  ```bash
  gh workflow run build.yml --repo samirhvbr/tura-notes -f version=0.11.12
  ```
- **A version was released with the wrong number in the package.** The bundle
  version is stamped from `version.md`; if they disagree, someone committed a
  stamped `tauri.conf.json`. CI rejects that, so the more likely cause is a
  build run without `tools/stamp-version.sh`.
- **The Arch job is red and nothing here changed.** That is the job doing its
  job: Arch is rolling, `webkit2gtk-4.1` moves, and finding out here is the
  entire point of ADR-023. It is information, not flakiness to be muted.

## 5. The git hooks

Once per clone — yours, and every collaborator's:

```bash
git config core.hooksPath tools/git-hooks
```

Confirm both directions before you rely on them:

```bash
git commit --allow-empty -m "feat: teste"                      # must be REJECTED
git commit --allow-empty -m "0.1.0 - primeiro commit do repo"  # must be accepted
```

Rename `HOOK_ESCAPE_VAR` at the top of each hook to `NOTES_NO_HOOK`.
Rules: [versioning.md](versioning.md).

## 6. Pre-flight before making a repository public

A private repository accumulates content that assumed privacy. Before flipping
visibility:

- [ ] `git log -p | grep -iE 'password|secret|token|api[_-]?key'` — scan the
      **history**, not just the working tree. A secret removed from HEAD is
      still in every clone.
- [ ] Any secret ever committed has been **rotated**, not merely deleted.
- [ ] No internal hostname, private IP range or infrastructure path that should
      not be public.
- [ ] `LICENSE` is present and `NOTICE` agrees with it about the license and the
      copyright holder.
- [ ] `SECURITY.md` names a reporting address that is actually monitored.
- [ ] Every prescriptive document declares its status; nothing reads as current
      while describing something abandoned.
- [ ] `.continue/` holds no finished item and no half-page detail.
- [ ] `README.md` is in English and describes what the project **is**, not what
      it was going to be.
- [ ] No placeholder left over from the skeleton:
      `grep -rn '<[A-Z_]\+>' . --exclude-dir=.git`

## 7. Verifying the repository still conforms

```bash
# the twins are identical below the H1
diff <(tail -n +2 CLAUDE.md) <(tail -n +2 AGENTS.md)

# version.md is a bare X.Y.Z
grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' version.md && echo ok

# settings.json parses
python3 -m json.tool .claude/settings.json > /dev/null && echo ok

# every version in history has a tag and a Release (prints what is missing)
./tools/release.sh --backfill --dry-run
```

If the `diff` on the twins reports anything other than the mirroring comment,
an edit was applied to one file and not the other.

## macOS module resolution

The modal component is `src/app/DialogHost.tsx`; its state module is
`src/app/dialog.ts`. Keep their stems distinct, including when case is ignored.
On a case-insensitive filesystem, an extensionless `app/Dialog` import can
resolve to `dialog.ts` and fail TypeScript with TS2305 and TS1149.
Run `npm run build` from `apps/notes-app` to check frontend module resolution.

## Development version

`npm run tauri dev` reads the first version in the repository's `version.md`
and supplies it through a CLI config override. The native About menu therefore
identifies the running source version. Restart development after a version bump.
The same override applies to `npm run tauri ios dev` and `android dev`.

The launcher does not edit `tauri.conf.json` or stamp build commands: packaging
still follows ADR-035 and `tools/stamp-version.sh`. A development version in
About is not evidence of a signed or published package. Run the launcher tests
with `node --test tools/tauri.test.mjs`; they also run in `tools/check.sh`.

## Local Linux installers (1.0.3)

Run on Linux, from the repository root. `deploy.sh` and `build-local.sh` are
identical entry points; publication requires `--publish`.

```bash
./deploy.sh --help
./deploy.sh                         # .deb + .AppImage
./deploy.sh --bundles deb           # Debian package only
./deploy.sh --bundles deb,appimage,rpm
./deploy.sh --publish               # build and ingest into the download service
```

Install Node 22.22.2+, 24.15+ or 26+, Rust through rustup, and Python 3. On Debian/Ubuntu:

```bash
sudo apt-get install build-essential pkg-config libwebkit2gtk-4.1-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf file xdg-utils python3
```

On Arch:

```bash
sudo pacman -S --needed base-devel pkgconf webkit2gtk-4.1 openssl \
  libayatana-appindicator librsvg patchelf file xdg-utils python
```

Artifacts and SHA-256 sidecars are under
`target/local-linux/<rust-host>/release/bundle/`. This path intentionally differs
from the macOS output and the CI tarball workflow. Build on the oldest Linux
release you intend to support; packages inherit the build host's system-library
requirements. This is a native build, not a Linux cross-compile from macOS.

The script pulls with `--ff-only` and stops on a failed pull; use
`--skip-git-pull` deliberately for offline/local changes. `--skip-npm-ci` reuses
installed dependencies. Linux always rebuilds packages, so `--force` is accepted
for compatibility. `--no-sign` marks a local test build and blocks publication.
The tracked Tauri version placeholder is restored on exit and interruption.

Publishing uses the same `TURA_PUBLISH_HOST`, `TURA_PUBLISH_STAGE`,
`TURA_PUBLISH_APP`, `TURA_PUBLISH_SLUG` and `TURA_PUBLIC_BASE` settings as the
macOS pipeline, plus `--dest` and `--base-url` overrides. Set these in the shell
on Linux; Apple credential files are not loaded. Staging paths must be absolute
and contain no spaces or shell characters. A failed checksum prevents ingestion.
See ADR-072 for the platform boundary and CI's continued Arch packaging role.

### Linux build verification (1.0.3)

The Linux packaging regression suite has eight passing cases, executed on macOS
and in a Debian 12 ARM64 container. It covers Linux dispatch through `deploy.sh`,
configuration restoration after failure, stale artifacts, argument validation,
failed pulls, help without dependencies, missing libraries and checksum rejection.
A real Debian 12 ARM64 build with Node 24.15.0 and Rust 1.98.1 produced deb,
AppImage and rpm installers; the Debian
metadata reports package `tura-notes`, version `1.0.3`, architecture `arm64`, and
all three SHA-256 sidecars validate. This does not verify graphical launch or x86_64.
Publication is tested with fake transport commands; no upload was performed.

The full repository gate was run. Frontend tests/build, network smoke, byte
preservation and script checks pass. The overall gate remains red: Clippy flags
the existing needless borrow in `notes-sync-client/src/control.rs:896`.
An old Tauri build cache also referenced the pre-rename `notes` directory;
`cargo clean -p tauri` repaired that cache and `cargo check -p notes-app` passed.
A subsequent `cargo test --workspace` reached `notes-core/tests/deep.rs` and failed
`starting_the_watcher_returns_immediately_and_walks_behind` and
`an_index_that_is_still_building_is_not_restarted_by_a_change`. Later Rust tests
were not reached. These failures remain tracked in `.continue/README.md`.
