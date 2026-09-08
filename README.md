# notes

> **Status:** `ACTIVE`

A local-first Markdown note-taking app for Linux, macOS, Windows, iOS and
Android. You pick a folder; that folder is your workspace; the `.md` files
inside it are your notes.

> **The files belong to the user, not to the application.**

There is no proprietary storage format and no account. A note is a Markdown file
on your filesystem, and it stays usable from a terminal, VS Code, `git`, `rsync`,
a backup tool or any other editor. Working offline is not a mode — it is the
normal case. Remote storage, multi-device sync and an API for AI agents come
later, are opt-in, and are **self-hosted by you**; there is no cloud service run
by us.

## Status

**Milestone 0.1 is built and packaged for Linux.** The editor, the preview, the
watcher, tabs, quick open, global search and the settings all exist and are
tested; the [acceptance documents](docs/README.md) list every criterion against
the test that holds it.

**What has not happened is a person walking the interface.** Every criterion so
far is an assertion about the core, and the twenty-five interface flows in
`docs/ACCEPTANCE-0.1b.md` and `docs/ACCEPTANCE-0.1c.md` are unticked on purpose:
a flow is verified when somebody has done it, not when a test near it passes.

macOS and Windows build and are tested in CI on every push. **No artefact is
published for either**, because an unsigned one teaches its user to click past
the warning that exists to protect them
([ADR-024](docs/decisions.md)) — the missing pieces are an Apple Developer
membership and a code-signing certificate, and they are named in
`.github/workflows/build.yml`.

Stack: Tauri 2 · React · TypeScript · Rust · CodeMirror 6. SQLite arrives with
the index at 0.2.

## Install

Linux, from the [latest release](https://github.com/samirhvbr/notes/releases/latest):

```bash
# Debian, Ubuntu and derivatives
sudo apt install ./notes_<version>_amd64.deb

# Anything else: the AppImage, which needs no installation
chmod +x notes_<version>_amd64.AppImage && ./notes_<version>_amd64.AppImage
```

Arch, from the release tarball via the `notes-bin` `PKGBUILD` in
[`packaging/aur/`](packaging/aur/) — the same one CI builds and installs in an
`archlinux:latest` container on every release.

The `.deb` depends on `libwebkit2gtk-4.1-0` and `libgtk-3-0`; the AppImage
carries its own copy and is correspondingly larger.

## Building it yourself

```bash
git clone git@github.com:samirhvbr/notes.git
cd notes
git config core.hooksPath tools/git-hooks

cd apps/notes-app && npm ci
npm run tauri dev            # run it
npm run tauri build          # package it — see docs/runbook.md §4
```

`tools/check.sh` is the full local gate: format, clippy on the native and the
Windows target, the whole test suite, the byte-preservation and full-disk
suites, the generated TypeScript, and the frontend. CI runs the same checks on
Ubuntu, macOS, Windows and rolling Arch.

## Documentation

| Path | What it is |
|---|---|
| [docs/product.md](docs/product.md) | **What notes is** — the local-first constraint, the workspace model, the editor, and what the first version deliberately does not do |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | **How it is put together** — layout, crates, core types, app-data schemas, the command contract, `CoreError`, the write and concurrency protocol, `Caps`, distribution |
| [docs/roadmap.md](docs/roadmap.md) | **The order it gets built in** — seven milestones, from a desktop editor to an MCP server |
| [docs/decisions.md](docs/decisions.md) | **The ADRs** — what was decided, why, and what it cost |
| [docs/](docs/README.md) | **The record** — the full index, plus security, versioning and runbooks |
| [.continue/](.continue/README.md) | **The queue** — what is still open, and whose call it is |
| [.claude/](.claude/README.md) | Model profile and permission posture for agents |
| [CHANGELOG.md](CHANGELOG.md) | **The history** — newest first; each heading is a commit subject |
| [version.md](version.md) | **The single authority on the version** — read as the first `X.Y.Z` in the file. Every bump becomes a tag and a published Release |

## Contributing

```bash
git pull
git config core.hooksPath tools/git-hooks   # once per clone
```

Write the `CHANGELOG.md` entry, bump `version.md` in the same commit, and commit
with the entry's heading as the subject — `0.1.1 - short description`.
Rules: [docs/versioning.md](docs/versioning.md).

## Standard

The documentation structure of this repository comes from the fleet standard at
[samirhvbr/repodocs](https://github.com/samirhvbr/repodocs). The norm itself
lives there and is deliberately **not** copied here, so there is one place to
change it.

## Language

English (US) for everything in the repository — documents, commit messages,
pull requests, issues, code comments — and nothing bilingual. Exactly one thing
stays Portuguese: **end-user-facing strings**, because that is product i18n, not
repository content.

**In a repository we do not own, the upstream's conventions win** — the language
and the commit shape both. Check before opening a pull request or an issue
there; when you cannot tell, English (US). See
[conventions.md §8](https://github.com/samirhvbr/repodocs/blob/master/docs/conventions.md#8-language).
