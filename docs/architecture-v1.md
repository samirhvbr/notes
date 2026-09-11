# Architecture — how notes is put together

> **Status:** `SUPERSEDED` · **Do not build against this file.** The architecture
> document milestone 0.1a is built against is [`ARCHITECTURE.md`](ARCHITECTURE.md)
> at the repository root — it is aligned to [SCOPE.md](SCOPE.md) v2.0 and
> closes all of its §20. This page was derived from the **v1** draft and
> contradicts v2 in several places, identity and app-data layout among them. It
> is kept only as the record of what was understood before v2 arrived.
>
> <details><summary>original status line</summary>
>
> **Status:** `PROPOSED` · **Nothing described here has been built yet.** This
> document is the worked-out form of a specification that still lives in the
> queue, [`../.continue/`](../.continue/README.md), and **the queue is the
> authority on intent while both exist** — intent changes there, and this page is
> updated when the thing is built ([ADR-009](decisions.md#adr-009--an-item-leaves-continue-only-when-it-has-been-built)).
> A section becomes `ACTIVE` when its code exists and works.
>
> </details>
>
> The structure of the system and the rules it must not
> break. What the product is lives in [product.md](product.md); the order things
> get built in, in [roadmap.md](roadmap.md); why each irreversible choice was
> taken, in [decisions.md](decisions.md).

## 1. The layering rule

Everything below is downstream of one rule, and it is the rule to check any
proposal against:

```text
Markdown files   →  source of truth
SQLite           →  index / cache
Notes Server     →  sync / remote storage
REST API         →  integrations
MCP              →  AI agents
```

Two shapes are forbidden outright:

```text
SQLite → the only copy of a note              never
proprietary format → export Markdown later    never
```

**Markdown exists on the filesystem from the first moment**, not as an export
path from something else. This is
[ADR-001](decisions.md#adr-001--markdown-files-on-the-filesystem-are-the-source-of-truth),
and it is the decision that makes every other one cheap to change.

## 2. Stack

```text
Tauri 2        application shell, desktop and mobile from one codebase
React          UI
TypeScript     interface state and logic
Rust           filesystem, indexing, search, local operations, native integration
CodeMirror 6   the editor
SQLite         index, cache, metadata
```

The reasoning, including what was given up, is in
[ADR-002](decisions.md#adr-002--tauri-2-with-react-typescript-rust-and-codemirror-6).

## 3. Repository layout

Two levels, and they are often confused because both are called "the structure":

```text
notes/
├── apps/
│   └── notes-app/          the Tauri application (desktop + mobile)
│       ├── src/            React · TypeScript
│       │   ├── components/
│       │   ├── editor/
│       │   ├── explorer/
│       │   ├── search/
│       │   ├── workspace/
│       │   ├── services/
│       │   ├── hooks/
│       │   └── stores/
│       └── src-tauri/      the Tauri shell: commands, wiring, platform glue
├── crates/
│   ├── notes-core/         domain model: workspace, note, revision, ids
│   ├── notes-fs/           the filesystem abstraction and its adapters
│   ├── notes-index/        indexer + SQLite schema + queries
│   └── notes-sync/         sync protocol, revisions, conflict model
├── packages/
│   └── ui/                 UI primitives shared between app surfaces
└── server/                 Notes Server — self-hosted, from milestone 0.5
```

**The Rust logic lives in `crates/`, not in `src-tauri/`.** `src-tauri/` is a
thin shell that exposes Tauri commands and wires them to the crates; it holds no
business logic of its own. That is what makes `server/` able to reuse
`notes-core`, `notes-index` and `notes-sync` without extracting them under
deadline pressure later — see
[ADR-003](decisions.md#adr-003--the-rust-logic-lives-in-crates-and-the-tauri-shell-stays-thin).

Nothing but `apps/notes-app/` needs to exist for milestone 0.1. The layout is
declared now so that the first crate is created in the right place.

## 4. The filesystem abstraction

The frontend must not know how the filesystem works on the platform it is
running on.

```text
UI
 │
 ▼
WorkspaceService
 │
 ▼
FileSystemAdapter
 │
 ├── DesktopFileSystem
 ├── IOSFileSystem
 ├── AndroidFileSystem
 └── RemoteFileSystem      milestone 0.5+
```

The minimum surface:

```text
list()   read()   write()   create()
rename() move()   delete()  stat()    watch()
```

**This abstraction exists from milestone 0.1, when there is exactly one adapter
behind it.** It looks like premature generality and it is not: "a folder the
user picked" is a desktop concept. iOS has no such thing — the app gets a
sandboxed container, or scoped access through the document picker, and neither
behaves like a directory you hold open. Android has scoped storage and the
Storage Access Framework. `RemoteFileSystem` is a network round-trip with
latency and failure modes that a local call does not have.

Discovering that after the UI has been written directly against local paths is a
rewrite of the UI, which is precisely the cost this seam is bought to avoid.

`watch()` is part of the minimum surface for the same reason: external
modification is a product requirement
([product.md §8](product.md#8-external-modification)), and platforms differ
enough in how they deliver it that it must sit behind the adapter rather than in
the UI.

## 5. Index

The filesystem stays the source of truth. A local SQLite index exists only to
make things fast:

```text
Filesystem → Indexer → SQLite
```

It accelerates search, tags, links, backlinks, recent files and caching.
Indexing is incremental — startup must not read every file's contents
([product.md §14](product.md#14-performance-target)).

If the database is deleted, the answer is `reindex workspace`, and the
application works normally again.

## 6. `.notes/`

The workspace may hold one reserved directory:

```text
notes/
├── .notes/
│   ├── workspace.json
│   ├── index.db
│   └── cache/
├── personal/
└── work/
```

**It holds only auxiliary data, and it must be safe to delete.** It must never
contain the only copy of a note, and nothing in it may be unrecoverable from the
Markdown files themselves —
[ADR-004](decisions.md#adr-004--notes-holds-only-data-that-can-be-rebuilt-and-must-be-deletable).

The test to apply to anything proposed for `.notes/`: delete the directory, and
does the user lose anything they wrote? If yes, it does not belong there.

## 7. Application state

UI state is separate from files. It covers the current workspace, the active
file, open tabs, sidebar state, cursor position, scroll, and recent files. A
light state manager — Zustand — rather than a framework.

The separation matters because of §1: file content flows from the filesystem
through the adapter, and UI state is a different lifetime with a different
persistence story. Mixing them is how an app ends up believing its own cache
over the disk.

## 8. Local security posture

**By default the desktop app opens no network port.** Nothing listens, nothing
dials out. Any external integration is explicitly enabled by the user, and the
API surface of milestones 0.5 and 0.7 belongs to the **server**, not to the
desktop app.

This is not merely a default — it is what lets the app be honest about being
local-first, and it is [ADR-007](decisions.md#adr-007--the-desktop-app-opens-no-network-port-by-default).
The normative security document is [security.md](security.md), which wins any
conflict with this page.

## 9. Sync model — designed for, built later

Sync is **not** in the MVP. What is required now is that nothing in the data
model makes it impossible later
([ADR-005](decisions.md#adr-005--sync-is-out-of-the-mvp-but-the-file-identity-model-is-not-foreclosed)).

**`modified_at` alone is not enough to synchronise anything.** Clocks disagree
between devices, filesystems round timestamps differently, and a restored backup
rewrites them wholesale. The eventual per-file identity:

```text
file_id        stable across renames
path           where it is now
revision       monotonic per file
content_hash   what it actually contains
modified_at    advisory, never authoritative
device_id      who wrote this revision
```

The protocol must also account for tombstones (deletions), conflicts, renames,
offline modification and version history.

### Conflicts

```text
desktop edits project.md
iphone  edits project.md
both offline
```

**A silent overwrite is a defect, not a resolution.** Whatever is built, both
versions survive it — as a second file:

```text
project.md
project (conflict iphone).md
```

or through a dedicated resolution interface.

## 10. Notes Server

From milestone 0.5. Runs in a container, owned and administered by the user:

```text
Desktop ──┐
          │
iPhone ───┼─── HTTPS ─── Notes Server (self-hosted)
          │
Android ──┘
```

It offers authentication, workspaces, files, sync, versions, an API and agent
access, with a `/data` volume. **The data stays administrable by whoever owns the
server** — there is no service we run and no account on infrastructure of ours.

Server-side version history is a server feature and **must not change the format
of the local Markdown file**. A note on disk stays a plain `.md` whatever the
server is keeping about it.

## 11. REST API

Exposed by the server, once it exists:

```text
GET    /api/v1/workspaces
GET    /api/v1/files
GET    /api/v1/files/{id}
POST   /api/v1/files
PUT    /api/v1/files/{id}
DELETE /api/v1/files/{id}
GET    /api/v1/search
```

Plus semantic operations — `append`, `prepend`, `rename`, `move` — which exist
because "append a line to this note" sent as a whole-file `PUT` is a lost update
waiting for two writers.

## 12. AI agents and MCP

An explicit goal of the project, at milestone 0.7:

```text
AI Agent → Notes API → Workspace
```

Within its granted scopes, an agent can list, search, read, create, edit, rename
and move files.

**Tokens carry scopes**, and a read-only agent must be expressible:

```text
notes:read    notes:write   notes:create
notes:delete  search:read
```

Beyond REST there is an **MCP** interface — `notes_list`, `notes_search`,
`notes_read`, `notes_create`, `notes_update`, `notes_move` — layered over the
same API, the same authentication and the same scopes. **REST stays the generic
interface; MCP is the agent-facing shape of it, never a second implementation
with its own rules.**

This is also where §1 earns its keep from the other direction: an agent writing
Markdown to disk is just another program touching the files, which is the case
[product.md §8](product.md#8-external-modification) already requires the app to
handle.
