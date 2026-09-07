# Roadmap — the order the product is built in

> **Status:** `ACTIVE` · What is built when, and what each stage must be able to
> do before the next one starts. What the product *is* lives in
> [product.md](product.md); how it is put together, in
> [architecture.md](architecture.md).

The stage numbers below are **product milestones, not repository versions.** The
repository version is whatever `../version.md` says and moves per commit; a
milestone is reached when everything under it works. Do not read `0.3` here as
`0.3.0` there.

## The shape of the sequence

```text
0.1  desktop editor        a genuinely usable local Markdown editor
0.2  index                 SQLite, global search, external-change detection
0.3  Markdown depth        front matter, tags, links, backlinks, attachments
0.4  mobile                iOS and Android against the same core
0.5  self-hosting          Notes Server, REST API, tokens, Docker
0.6  sync                  revisions, hashes, tombstones, conflicts, offline
0.7  AI                    MCP server over the same API and the same scopes
```

Each stage is useful on its own. That is the constraint that sets the order: a
user who stops receiving updates after 0.1 still has a working Markdown editor,
and one who stops after 0.3 has a good one.

---

## 0.1 — Desktop MVP

Linux, macOS, Windows.

- launch the application;
- select a workspace (open an existing folder / create a new one);
- list directories and `.md` files;
- create, open and edit a file;
- autosave, with atomic writes;
- rename, delete, duplicate;
- create a directory; move a file;
- Markdown syntax highlighting;
- Markdown preview;
- in-file search;
- Quick Open;
- persist the selected workspace across restarts.

**Done means:** the app is a Markdown editor someone would actually use daily,
with nothing but a folder. No index, no server, no account.

## 0.2 — Index

- SQLite index under `.notes/`;
- incremental indexing;
- workspace-wide search (name, path, content);
- recent files;
- external-change detection via filesystem watching;
- tab state restored on restart;
- command palette.

**Done means:** deleting `.notes/` costs a reindex and nothing else — see
[ADR-004](decisions.md#adr-004--notes-holds-only-data-that-can-be-rebuilt-and-must-be-deletable).

## 0.3 — Markdown depth

- YAML front matter;
- tags, from both `#tag` and front matter, related in the index;
- internal links;
- backlinks;
- images and attachments;
- tables;
- preview improvements.

## 0.4 — Mobile

iOS and Android, against the same core.

- select a workspace;
- navigate, open, edit, create;
- search;
- autosave.

**The interface is adapted, not shrunk.** This is also the stage that pays for
the filesystem abstraction in
[architecture.md](architecture.md#4-the-filesystem-abstraction), and the reason
it is here rather than at 0.1 is
[ADR-008](decisions.md#adr-008--desktop-first-mobile-at-milestone-04-behind-the-same-abstraction): "a folder the
user chose" is a desktop concept, and iOS in particular has no equivalent — the
adapter is what absorbs that, and it exists from 0.1 precisely so this stage is
not a rewrite.

## 0.5 — Self-hosting

**Notes Server**, run by the user:

```bash
docker compose up -d
```

- authentication;
- remote workspaces;
- storage;
- REST API;
- tokens with scopes;
- basic versioning.

The user's data stays administrable by whoever owns the server. There is no
service we operate.

## 0.6 — Sync

- devices;
- revisions, content hashes;
- tombstones for deletions;
- incremental sync;
- offline mode;
- conflict detection and resolution;
- history.

**A silent overwrite is a defect, never a resolution** — see
[ADR-005](decisions.md#adr-005--sync-is-out-of-the-mvp-but-the-file-identity-model-is-not-foreclosed).

## 0.7 — AI

- REST API (already standing from 0.5);
- **MCP server** exposing `notes_list`, `notes_search`, `notes_read`,
  `notes_create`, `notes_update`, `notes_move`.

Both go through the same authentication and the same scopes. REST stays the
generic interface; MCP is the agent-facing layer over it, not a second
implementation.

---

## What is deliberately absent from every stage above

Collaborative editing, a full WYSIWYG editor, canvas, graph view, a plugin
system, multiple themes, web publishing, an embedded AI chat, native Git
integration, user accounts on infrastructure we run, and an official cloud.

Absent is not the same as rejected. Any of them can be argued later — with an
ADR, and against [product.md §1](product.md#1-what-it-is), which none of them
may break.
