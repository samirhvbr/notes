# Roadmap — the order the product is built in

> **Status:** delivered through the implemented portions of 0.6 at `0.20.20`.
> Milestones 0.0, 0.1d, 0.2, 0.3 and 0.5 retain owner acceptance; 0.4, the
> remaining 0.6 work and 0.7 remain queued in [`.continue/`](../.continue/README.md).

The stage numbers below are **product milestones, not repository versions.** The
repository version is whatever `../version.md` says and moves per commit; a
milestone is reached when everything under it works. Do not read `0.3` here as
`0.3.0` there.

## The shape of the sequence

```text
0.1  desktop editor        a genuinely usable local Markdown editor
     0.1a  core            editor, write protocol, identity
     0.1b  workspace       watcher, entry operations, preview, conflicts
     0.1c  navigation      tabs, quick open, global search, palette, settings
     0.1d  interface       the shell: rail, sidebar, tabs, note header, status bar
0.2  index                 SQLite, global search, external-change detection
0.3  local knowledge       metadata, tags, wiki links, graph, images, local MCP
0.4  mobile                iOS and Android against the same core
0.5  self-hosting          Notes Server, REST API, tokens, Docker
0.6  sync                  revisions, hashes, tombstones, conflicts, offline
0.7  remote MCP            MCP over the server API and the same scopes
```

Each stage is useful on its own. That is the constraint that sets the order: a
user who stops receiving updates after 0.1 still has a working Markdown editor,
and one who stops after 0.3 has a good one.

---

## 0.1 — Desktop MVP

> **0.1a–0.1d built; owner acceptance pending.** Shipped across `0.3.0`–`0.11.x` as
> 0.1a (editor and write protocol), 0.1b (watcher, entry operations, preview,
> conflicts) and 0.1c (tabs, quick open, global search, palette, settings,
> `en`/`pt-BR`). Packaged for Linux: `.deb`, AppImage and the AUR `notes-bin`;
> macOS and Windows are tested in CI and **not published**
> ([ADR-024](decisions.md)).
>
> **0.1d — Interface** was added to the scope on 08/09/2026
> ([ADR-037](decisions.md)) and the desktop MVP is now `0.1a + 0.1b + 0.1c +
> 0.1d`. The editor works; what it did not have was a way to reach it. Graph
> view moved from "out of scope" to 0.3 in the same pass
> ([ADR-038](decisions.md)).

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

Implemented in 0.14.0:

- separate operational `registry.db` and derived `index.db` in app data;
- incremental, cancellable SQLite/FTS5 indexing and explicit rebuild;
- named Words, Literal and Regex modes, with partial results labelled;
- Recent notes and live Outline navigation;
- rename/move with reviewed incoming and outgoing Markdown references,
  original-byte backups and per-file concurrency checks.

External reconciliation, tabs and the command palette remain from 0.1.
**Acceptance:** reindexing changes neither note bytes nor identity, and search
semantics never switch implicitly. See [ACCEPTANCE-0.2.md](ACCEPTANCE-0.2.md).

## 0.3 — Knowledge and local agents

Implemented in 0.16.0: interpreted YAML properties; inline/YAML tags outside
code; wiki links with explicit ambiguity; backlinks and graph; paste images
into noncolliding root attachments; standalone stdio `notes-mcp`. App/MCP
cross-process conflict refusal and append retries are exercised by real
process tests. See [the implemented contract](KNOWLEDGE-0.3.md) and
[owner acceptance](ACCEPTANCE-0.3.md). Tables were already in the shared parser.

## 0.4 — Mobile

**Foundation implemented in 0.17.0; the usable mobile app remains pending.**
The reviewed PR makes trash desktop-only and adds an iOS simulator core check.
ADRs 040/042 keep mobile in this application, starting with the iOS container.
No generated mobile projects, layout or external-folder adapter exists yet.
See [ACCEPTANCE-0.4.md](ACCEPTANCE-0.4.md) for the remaining scope.

iOS and Android, against the same core.

- select a workspace;
- navigate, open, edit, create;
- search;
- autosave.

**The interface is adapted, not shrunk.** This is also the stage that pays for
the filesystem abstraction in
[architecture.md](architecture-v1.md#4-the-filesystem-abstraction), and the reason
it is here rather than at 0.1 is
[ADR-008](decisions.md#adr-008--desktop-first-mobile-at-milestone-04-behind-the-same-abstraction): "a folder the
user chose" is a desktop concept, and iOS in particular has no equivalent — the
adapter is what absorbs that, and it exists from 0.1 precisely so this stage is
not a rewrite.

## 0.5 — Self-hosting

Implemented in **0.18.0** as the independent `notes-server` process. The
[operator guide and REST contract](SERVER-0.5.md) cover credentials, HTTPS,
conditional writes, limits and offline backup/restore. Automated acceptance
and the pending owner walk are in [ACCEPTANCE-0.5.md](ACCEPTANCE-0.5.md).

**Notes Server**, run by the user:

```bash
docker compose -f server/compose.yml up -d --build
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

In progress. Version **0.19.0** introduces the [causal domain and mounted-folder
pairing preview](SYNC-0.6.md). It does not transfer remote content; the
[remaining implementation](../.continue/0.6-sync.md) stays in the queue.

- devices;
- revisions, content hashes;
- tombstones for deletions;
- incremental sync;
- offline mode;
- conflict detection and resolution;
- history.

**A silent overwrite is a defect, never a resolution** — see
[ADR-005](decisions.md#adr-005--sync-is-out-of-the-mvp-but-the-file-identity-model-is-not-foreclosed).

The 0.19.1 sync block adds a scoped server revision inbox with atomic content,
idempotent publication and incremental metadata pages. The 0.20.0 device client adds durable queues and resumable transfer. The 0.20.1 block applies creations and same-path updates through core while the
workspace is closed and draft-free. The 0.20.3 block explicitly acknowledges
durable application receipts to the server. The 0.20.5 core API supports
exclusive open sessions with observed buffer checks. In 0.20.6 the app applies prepared receive queues behind an editing barrier.
In 0.20.7 upload queues resolve same-path divergence explicitly while preserving
both histories. In 0.20.8 upload conflicts involving renames/deletions have
explicit result choices. In 0.20.9 saved same-path receiver edits can be
captured, resolved and applied with separate superseded/deferred progress.
Desktop pairing and conflict controls ship in 0.20.14, with explicit
older-server recovery and two-device rollback regressions in 0.20.15; see [SYNC-0.6.md](SYNC-0.6.md).

The 0.20.10 sync increment restores remote rename/delete conflicts explicitly
at the receiver's applied path. Version 0.20.11 adds explicit recapture
of further saved edits, retaining prior branches. Version 0.20.12 applies explicit receiver move/delete choices and confirms
subfolder/reconciliation pairing. Version 0.20.13 adds explicit deletion capture, ordered rename cycles and
referenced binary bundles with guarded recovery. Version 0.20.14 adds desktop background transfer with conservative conditions
and pairing/history/conflict controls. Version 0.20.16 adds saved same-path
receiver publications and opt-in scheduled capture with the workspace closed.
Version 0.20.17 adds independent new-note and recognized-rename capture.
Version 0.20.18 adds audited restored-client cache/outbox recovery and two-device
move/delete receipt-loss tests. Version 0.20.20 adds scoped backup
reconciliation and retirement of devices whose owning credential was revoked.
Version 0.20.19 adds offline server pruning and matching receiver compaction for
unanimously acknowledged divergent branch payloads. Linear history, current
resolution payloads and cursor baselines remain open. Version 0.20.20 adds
restores scoped receiver queues across filtered cursor gaps. Version 0.20.22
compacts acknowledged linear payloads while retaining a current receive
baseline. Version 0.20.23 explicitly reconciles restored application identities,
and 0.20.24 verifies that recovery through real two-device TCP and HTTPS flows.
Device acceptance remains pending.

## 0.7 — AI

- REST API (already standing from 0.5);
- **MCP server** exposing `notes_list`, `notes_search`, `notes_read`,
  `notes_create`, `notes_update`, `notes_move`.

Both go through the same authentication and the same scopes. REST stays the
generic interface; MCP is the agent-facing layer over it, not a second
implementation.

---

## What is deliberately absent from every stage above

Collaborative editing, a full WYSIWYG editor, canvas, a plugin
system, multiple themes, web publishing, an embedded AI chat, native Git
integration, user accounts on infrastructure we run, and an official cloud.

Absent is not the same as rejected. Any of them can be argued later — with an
ADR, and against [product.md §1](product.md#1-what-it-is), which none of them
may break.
