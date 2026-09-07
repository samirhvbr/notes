# notes

> **Status:** `ACTIVE`

A desktop app for writing Markdown — standalone, and optionally linked to a public Git repository.

<!-- Replace everything below with the real thing. What survives from the
     skeleton is the SHAPE: what the project is, how to run it, where the docs
     are, and the language rule. -->

## Getting started

```bash
# clone, install, run — fill this in
```

## Documentation

| Path | What it is |
|---|---|
| [docs/](docs/README.md) | **The record** — architecture, decisions, security, runbooks |
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
