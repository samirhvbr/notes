# Runbook — notes

> **Status:** `ACTIVE` · From a clean machine to a running environment, and the
> checklists that gate a release.

<!-- Sections 1–4 are yours to fill in. Sections 5–7 are inherited from the
     fleet standard and are already correct — keep them. -->

## 1. Requirements

_Fill in: runtimes and versions, system packages, accounts and access needed._

## 2. From a clean machine to running

```bash
git clone git@github.com:samirhvbr/notes.git
cd notes
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
| Linux | `.deb`, AppImage, a tarball, and the AUR `notes-bin` package — all built and attached |
| macOS, Windows | **not published.** The jobs are written in `build.yml` behind `if: false`; each carries the list of what is missing, and in both cases it is an account or a certificate rather than code (ADR-024) |

### Building the packages locally

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
  gh workflow run build.yml --repo samirhvbr/notes -f version=0.11.12
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
