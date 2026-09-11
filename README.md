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

**Milestones 0.1, 0.2 and 0.3 are implemented.** The desktop shell now includes
Files, Recent and Outline, incremental SQLite word search, and a review of
incoming/outgoing Markdown references before rename or move. Literal and Regex
retain their scan semantics; editing and Quick Open do not depend on the index.
[Local knowledge and agents](docs/KNOWLEDGE-0.3.md) adds YAML properties, tags,
wiki links, backlinks, graph navigation, clipboard images and standalone
`notes-mcp` with scoped permissions and guarded writes.

Owner verification on installed Linux releases, repeated on the following
release, remains pending in [0.1d acceptance](docs/ACCEPTANCE-0.1d.md) and
[0.2 acceptance](docs/ACCEPTANCE-0.2.md) and
[0.3 acceptance](docs/ACCEPTANCE-0.3.md). Automated tests are recorded separately.

macOS and Windows build and are tested in CI on every push. **No artefact is
published for either**, because an unsigned one teaches its user to click past
the warning that exists to protect them
([ADR-024](docs/decisions.md)) — the missing pieces are an Apple Developer
membership and a code-signing certificate, and they are named in
`.github/workflows/build.yml`.

Stack: Tauri 2 · React · TypeScript · Rust · CodeMirror 6 · SQLite/FTS5.

## Install

Linux, from the
[latest release that carries packages](https://github.com/samirhvbr/notes/releases):
every commit is a version, and **packages are built for minor bumps** (`X.Y.0`)
and on request — a patch release says so in its own description ([ADR-036](docs/decisions.md)).

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

## Optional self-hosted server

Milestone 0.5 adds a separate `notes-server` executable with a scoped,
authenticated REST API, conditional writes and offline backup/restore. See the
[server guide](docs/SERVER-0.5.md) for local use and Docker with HTTPS, and the
[OpenAPI contract](server/notes-server/openapi.json) for integration. Desktop
sync and remote MCP are later milestones; this does not enable a desktop port.

## Synchronization preview

Milestone 0.6 is in progress. `notes-sync-plan` previews upload, download or
reconciliation between two mounted folders without changing their notes.
Build it with `cargo build -p notes-core --bin notes-sync-plan`, or use the
standalone Linux release archive. See [SYNC-0.6.md](docs/SYNC-0.6.md) for its
causal model and the remaining work before remote synchronization is available.

The 0.19.1 server also exposes a scoped immutable revision inbox. It transfers
original bytes and acknowledges storage, without applying changes to workspace
files. See [the sync contract](docs/SYNC-0.6.md#server-revision-inbox-0191).

Version 0.20.0 adds `notes-sync-client` for offline staging and resumable transfer
to/from private revision inboxes. Version 0.20.1 adds explicit application of
received creations and same-path updates while the workspace is closed and
draft-free, with local revision checks and crash recovery. Usage and limits are in [SYNC-0.6.md](docs/SYNC-0.6.md#device-transfer-client-0200);
the current remaining work is in [the queue](.continue/README.md#current-implementation-order).

Version 0.20.5 adds the Rust core foundation for an exclusively owned open sync
session, with buffer snapshot checks. The CLI applies only with the workspace
closed; app integration is available from 0.20.6. See [the host contract](docs/SYNC-0.6.md#exclusive-open-session-core-foundation-0205).

Version 0.20.6 connects prepared receive queues to the app. Close the current
workspace, choose **Open received workspace**, select the CLI state directory,
and use **Apply received revisions**. Dirty buffers/drafts are refused and input
stays paused until a safe reload after uncertain outcomes. See [the app workflow](docs/SYNC-0.6.md#apply-a-received-queue-in-the-app-0206).

Version 0.20.7 adds explicit same-path conflict resolution to upload queues:
`fetch`, `conflicts`, `export`, then `resolve` with both observed revision UUIDs
and a chosen result file. Both histories survive; publication still refuses a
stale remote head. See [the resolution workflow](docs/SYNC-0.6.md#explicit-divergent-resolution-0207).

Version 0.20.8 extends uploader conflict resolution with `resolve-to` (explicit
path and bytes) and `resolve-delete` (explicit tombstone). Source files remain
unchanged; see [rename and deletion choices](docs/SYNC-0.6.md#rename-and-deletion-choices-0208).
