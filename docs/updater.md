# Signed desktop updates

> **Status:** ACTIVE · 1.1.0 · [ADR-074](decisions.md#adr-074--signed-desktop-updates-with-explicit-installation)

## Application behavior

Release builds check 20 seconds after launch and every six hours. Welcome and
Settings also expose a manual check. A new version shows its release notes and
an **Install and restart** action; **Later** suppresses automatic notices for
that version. A manual check can show it again. Automatic failures stay silent;
manual failures show a retryable status. Downloads/installations are never
started by a timer.

Close the workspace through its normal save/conflict flow before installation.
Both frontend and native code enforce this condition. The editing/IPC barrier
blocks new work during installation and is released on failure. The native
plugin downloads over HTTPS, verifies the payload against the pinned Tura public
key, installs and restarts. AppImage replaces the running image; deb/rpm may
require the system's administrator authentication. AUR packages carry a pacman
marker and use their package manager instead. Debug, mobile and Windows builds
do not offer this update channel.

Versions predating 1.1.0 cannot discover updates: manually install the first
updater-capable release. A GitHub Release alone does not activate its updater
feed; the signed local publisher does that.

## Release builder setup

Tura uses an independent updater signing key. The public key is committed in
`apps/notes-app/src-tauri/tauri.conf.json`; the private key belongs outside Git,
by default at `~/.config/tura-notes/updater.key` with mode 600 (directory 700).
The same private key must be provisioned securely on each release builder.
Do not generate a different key for each machine or release: installed clients
trust the public key already embedded in their application.

For a key stored elsewhere, set `TAURI_SIGNING_PRIVATE_KEY_PATH`. The CLI's
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are also
supported. Never place key material in commands, documentation, artifacts or
logs. Apple Developer ID signing and notarization remain separate requirements
for macOS. Preflight signs a disposable probe and verifies it against the pinned
public key before the expensive application build.

```bash
./deploy.sh --publish                     # native macOS or Linux
./deploy.sh --bundles deb,appimage,rpm --publish   # Linux only
./deploy.sh --publish                     # retry a failed upload, reuse build
./deploy.sh --no-sign                     # local test, publication refused
```

Linux stores `.sig` and `.updater.json` beside each package and includes their
hashes in the completed-build record. macOS retains the notarized `.app`, creates
`Tura Notes.app.tar.gz` and signs that archive separately from the DMG. Missing
or changed signing records invalidate reuse. A valid publication retry needs
neither a private signing key nor npm/Cargo compilation. `--force` rebuilds.

## Hosting and publication

Upload remains SCP/SSH to `b3sys@100.64.100.242`. Public clients read:

```
https://samirhv.com.br/updates/tura-notes/{{target}}-{{arch}}-{{bundle_type}}.json
```

Examples: `darwin-aarch64-app.json`, `linux-x86_64-deb.json`,
`linux-aarch64-appimage.json`, `linux-x86_64-rpm.json`. Each feed contains version,
notes, HTTPS payload URL and signature. Payload names include version, platform
and a content hash so a newly published feed cannot change an in-flight download.

After normal download-service ingestion, the publisher stages files in a unique
remote directory, checks SHA-256 and uses `sudo -u www-data` to install under
`$TURA_PUBLISH_APP/public/updates/tura-notes`. It installs the payload first and
atomically replaces only that platform's feed last. Staging is cleaned even on
failure. It then checks the public feed and hashes the public download. The web
server must serve this directory directly without authentication or stale cache
for JSON feeds. Serialize releases for the same platform; publishing an older
version intentionally changes that feed, although clients never downgrade.

`TURA_PUBLISH_HOST`, `TURA_PUBLISH_STAGE`, `TURA_PUBLISH_APP` and `TURA_PUBLIC_BASE`
retain their existing meanings. Changing the public base for the publisher does
not change an already installed client's endpoint. Changing update service or
key requires an explicit migration. Keep old payloads while clients may still
be downloading them. CI's existing unsigned Linux installers do not publish a
signed feed; only this signing/publishing path does.

## Validation and remaining acceptance

Automated tests cover notice/dismiss/manual-check behavior, workspace and input
barriers, failed installation cleanup, signed-record reuse, tamper detection,
upload checksum failures and payload-before-feed ordering. The existing Linux
orchestration suite continues to cover failed publication followed by reuse.

Installed acceptance requires two signed releases: install the older one, publish
the newer one for the same architecture/format, verify automatic and manual
notices, postpone, close a dirty workspace through its save flow, install/restart
and check version and note bytes. Repeat offline, with a corrupt payload, and
with denied administrator authentication. Run separately on macOS, AppImage,
deb and rpm; confirm AUR delegates to pacman. Compilation and signature checks
do not establish this installed acceptance. Remaining work is in `.continue/`.

### Verification of 1.1.0

On 12/09/2026, the final macOS ARM64 application and DMG passed Apple
notarization, stapling and Gatekeeper checks. Its updater archive was signed
and verified against the configured key, and a repeat build reused it.
Debian 12 ARM64 produced deb, AppImage and rpm packages; a repeat invocation
reused all three. Those Linux packages were built with `--no-sign` in the local
Docker environment, then signed and verified on the macOS host, keeping the
private key out of the container. This validates native packaging and the actual
payload signatures, not an installed cross-version upgrade.

The frontend suite passed 90 tests; Linux orchestration passed 11, and updater
publication passed 5. A real signed probe verified successfully and a modified
probe was rejected. Clippy passed for the application library and standalone
signature verifier. The whole repository gate was run and still has the queued
`notes-sync-client` needless-borrow failure and watcher startup timing failure.
The changed CSS passes the contrast gate; the tracked version placeholder was
verified again after packaging restored it.

The live updater feed has **not** been published. The private host responds,
but `/srv/www/samirhv.com.br/samirhv` does not exist there. Confirm the site's
actual application directory before running `--publish`. No installed upgrade
or live updater transport is claimed by these local checks.
