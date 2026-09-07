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

**Documentation only — there is no application code in this repository yet.**
What exists is the product definition, the architecture and the decisions behind
them. [docs/roadmap.md](docs/roadmap.md) says what gets built and in what order;
milestone 0.1 is a usable desktop Markdown editor.

Planned stack: Tauri 2 · React · TypeScript · Rust · CodeMirror 6 · SQLite.

## Getting started

```bash
# nothing to run yet — see docs/roadmap.md, milestone 0.1
git clone git@github.com:samirhvbr/notes.git
cd notes
git config core.hooksPath tools/git-hooks
```

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
