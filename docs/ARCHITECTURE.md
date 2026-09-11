# Architecture

**Status: ACTIVE** through milestones 0.1d and 0.2, implemented in 0.14.0.
Sections tagged `[0.3]` and `[0.4]` remain `PROPOSED`. Owner verification of
installed releases is tracked separately in the acceptance documents.

The decisions this document introduced are recorded as **ADR-013 … ADR-024** in
[decisions.md](decisions.md), with **ADR-025 … ADR-029** added by 0.1b. The
calls taken while building are in [DECISIONS-0.1a.md](DECISIONS-0.1a.md) and
[DECISIONS-0.1b.md](DECISIONS-0.1b.md); what each acceptance criterion is
verified by is in [ACCEPTANCE-0.1a.md](ACCEPTANCE-0.1a.md) and
[ACCEPTANCE-0.1b.md](ACCEPTANCE-0.1b.md).

This document closes the decisions the product scope leaves to architecture:
repository layout, crates, core types, the app-data layout and its schemas, the
command contract and error model, the write and concurrency protocol, the
cross-process lock, the Markdown IR, the filesystem capability matrix, and the
distribution pipeline. It is what an agent needs before writing 0.1a code.

It is not the roadmap (`docs/roadmap.md`), not the product rules
(`docs/product.md`; until built, the Portuguese scope v2.0 in
`.continue/SCOPE_final.md`), and not the security policy —
**`docs/security.md` is normative and wins any conflict with this file.**

This file replaces the earlier scope-v1 architecture page, kept as
[`architecture-v1.md`](architecture-v1.md) and superseded by the v2.0 scope on
identity, data categories, registry location, concurrency and the crate set. ADRs referenced as `ADR-NNN` live in `docs/decisions.md`; the
new decisions this document introduces are listed in §18 for recording there.

---

## 1. Repository layout

```
notes/
├── apps/
│   └── notes-app/
│       ├── src/                    React + TypeScript
│       │   ├── app/                shell, layout, keyboard map, command palette
│       │   ├── editor/             CodeMirror 6 setup and extensions
│       │   ├── explorer/           tree, context menus, drag-and-drop (later)
│       │   ├── preview/            mounts sanitized HTML from the core
│       │   ├── search/             quick open, in-file, global
│       │   ├── stores/             Zustand stores (workspace, tabs, editor, search, ui)
│       │   ├── ipc/                the ONLY place `invoke` is called; generated types
│       │   └── i18n/               en, pt-BR
│       └── src-tauri/              thin shell: commands → notes-core, capabilities, asset protocol
├── crates/
│   ├── notes-model/                ids, paths, stat, caps, errors, events — no I/O
│   ├── notes-fs/                   FileSystem trait, LocalFs, root jail, atomic write, watcher
│   ├── notes-markdown/             [0.1b] parse → Document IR, render → sanitized HTML
│   ├── notes-core/                 WorkspaceService: registry, write protocol, reconciliation, session
│   ├── notes-index/                [0.2] SQLite, FTS5, parsed links; tags at 0.3
│   └── notes-mcp/                  [0.3] stdio MCP server over notes-core
├── packages/
│   └── ui/                         shared React components; may stay empty until 0.4 needs it
├── fixtures/
│   ├── basic/                      ~200 committed notes
│   ├── edge-cases/                 committed: CRLF, BOM, NFD, mixed EOL, empty, duplicates, 5 MB, odd names
│   ├── xss/                        committed: HTML, javascript: links, remote images, file:// images
│   └── large/                      generated, gitignored — `tools/gen-large.sh` (10k files, ~200 MB)
├── packaging/
│   └── aur/                        PKGBUILD for notes-bin (and notes-git)
├── tools/                          gen-large.sh, crash-save-loop, release.sh, git-hooks/
├── docs/
└── .continue/
```

Cargo workspace at the root (`crates/*`, `apps/notes-app/src-tauri`); **npm**
for `apps/*` and `packages/*`, with `package-lock.json` committed.
`rust-toolchain.toml` pins stable at the version current when 0.1a starts;
`.nvmrc` pins Node LTS.

Dependency direction, enforced by `Cargo.toml` and checked in CI:

```
notes-model ← notes-fs ──────┐
notes-model ← notes-markdown ─┼─← notes-core ← { src-tauri, notes-mcp, notes-index }
```

No crate under `crates/` depends on `tauri`. A crate that needs it is in the
wrong place (ADR-003).

Version discipline that follows from `docs/versioning.md`: adding a crate, or
changing the `FileSystem` trait surface, is a **Y** bump; a registry or index
schema change that forces a migration or reindex is a **Y** bump.

---

## 2. Crates and responsibilities

| Crate | Owns | Must not |
|---|---|---|
| `notes-model` | `WorkspaceId`, `NoteId`, `RelPath`, `CompareKey`, `ContentHash`, `Stat`, `NativeId`, `BaseRev`, `TextProfile`, `Caps`, `CoreError`, event enums | do I/O, depend on anything but `serde`, `uuid`, `thiserror`, `ts-rs` |
| `notes-fs` | `FileSystem` trait; `LocalFs`; root jail; atomic replace; case-sensitivity probe; `notify` watcher normalized to `FsEvent` | know what a note is; touch the registry |
| `notes-markdown` `[0.1b]` | `parse(&str) -> Document`; `render_html(&str, RenderOpts) -> Rendered`; link resolution; slugging; sanitization | do I/O; know about workspaces |
| `notes-core` | `WorkspaceService` — open/close workspace, registry, text profile, write protocol, drafts, conflicts, reconciliation, identity correlation, session, settings, search-by-scan, cross-process lock | render UI strings; depend on `tauri` |
| `notes-index` `[0.2]` | separate SQLite stores, incremental cache, FTS5, parsed document facts; graph/tags at 0.3 | be required for opening or editing a note |
| `notes-mcp` `[0.3]` | stdio server; permission scopes; base-rev enforcement | contain any note logic not in `notes-core` |

`notes-core` is the only public API. `src-tauri` and `notes-mcp` are clients of
it and contain no policy of their own.

---

## 3. Core types

```rust
// notes-model
pub struct WorkspaceId(Uuid);   // v4, assigned when a root is registered
pub struct NoteId(Uuid);        // v4, assigned the first time a note is seen

/// '/'-separated, relative to the root, exactly as the name is on disk.
/// Never normalized, never rewritten. Constructed only via `RelPath::parse`.
pub struct RelPath(String);

/// Comparison key: NFC, plus Unicode case-fold when the root's filesystem is
/// case-insensitive. Used to detect collisions and to match watcher events.
/// Never written to disk, never shown to the user.
pub struct CompareKey(String);

pub struct ContentHash([u8; 32]);   // blake3

pub enum NativeId {
    Unix    { dev: u64, ino: u64 },
    Windows { volume: u64, index: u64 },
    Provider(String),                // SAF document id, iOS bookmark id [0.4]
}

pub struct Stat {
    pub size: u64,
    pub mtime_ns: i128,              // nanoseconds since epoch; i128 avoids overflow surprises
    pub native_id: Option<NativeId>,
    pub kind: EntryKind,             // File | Dir | Symlink | Other
}

/// What the open buffer was read against. Size+mtime are the cheap check;
/// hash is the decision.
pub struct BaseRev { pub size: u64, pub mtime_ns: i128, pub hash: ContentHash }

pub struct TextProfile {
    pub encoding: Encoding,          // Utf8 | Unknown(read-only)
    pub bom: bool,
    pub eol: Eol,                    // Lf | CrLf | Mixed(read-only)
    pub final_newline: bool,
}

pub struct Caps {
    pub atomic_replace: bool,        // tmp + rename-over is atomic on this backend
    pub rename: bool,
    pub trash: bool,
    pub watch: bool,
    pub native_id: bool,
    pub preserve_mode: bool,
    pub create_new: bool,            // O_EXCL-style create without overwrite
    pub same_volume_move: bool,
}
```

`RelPath::parse` rejects: empty; leading `/` or drive letter; any `.` or `..`
segment; `\`; NUL or C0 control characters; a trailing `/`. It does **not**
normalize Unicode or case — the string is kept as given so it can be joined to
the root and hit the file the user actually has.

Case-sensitivity of the root is **probed, never assumed from the operating
system** — a Linux mount can be exFAT, NTFS or SMB, and macOS can be formatted
case-sensitive. The probe is read-only, because product rule 3 and the 0.1a
acceptance criterion both forbid creating anything in a folder that was merely
opened:

1. `list()` the root (needed for the tree anyway). Pick an entry whose name
   contains a cased letter.
2. `stat()` that name with its case inverted. Both succeed **and** report the
   same `native_id` → **insensitive**. The flipped name does not exist →
   **sensitive**.
3. No usable entry (empty root, or every name caseless) → **Unknown**, and
   Unknown is treated as **insensitive**, which is the safe direction: treating a
   sensitive filesystem as insensitive refuses a legitimate name, while the
   reverse lets `note_create` pass a collision check and overwrite a note.

The result is persisted in `registry.db` (`case_insensitive`) and
**self-corrects in both directions** from later observations: two entries equal
under case-fold on a root marked insensitive → flip to sensitive; a successful
case-flipped `stat` with a matching `native_id` on a root marked sensitive →
flip to insensitive. Both flips are persisted.

---

## 4. App data layout

The core does not use Tauri's `app_data_dir()`, because `notes-mcp` runs
without Tauri and both processes must agree on one directory. The core owns the
resolution:

```
default_data_dir() = dirs::data_dir()/notes      overridable by NOTES_DATA_DIR (tests, portable use)
  Linux    ~/.local/share/notes     ($XDG_DATA_HOME/notes)
  macOS    ~/Library/Application Support/notes
  Windows  %APPDATA%\notes
```

```
<data_dir>/
├── settings.json                        global settings (§4.5)
├── workspaces.json                      { schema, workspaces: [{ id, root, display_name, last_opened }] }
└── workspaces/<WorkspaceId>/
    ├── registry.db                      identity registry — OPERATIONAL (§4.1)
    ├── registry.json.bak-1              retained legacy migration backup
    ├── recent.json                     recently opened NoteIds — OPERATIONAL
    ├── reference-backups/               reviewed rewrite originals and journals
    ├── drafts/<NoteId>.md               unsaved buffer snapshot — OPERATIONAL (§4.2)
    ├── drafts/<NoteId>.json             draft sidecar
    ├── conflicts/<NoteId>/<ts>-<side>.md  non-chosen version — OPERATIONAL (§4.3)
    ├── conflicts/<NoteId>/<ts>-<side>.json
    ├── session.json                     tabs, cursor, scroll — OPERATIONAL, resettable (§4.4)
    ├── write.lock                       cross-process advisory lock, ephemeral (§6)
    ├── index.db                         DERIVED — deleting it only costs a reindex
    └── cache/                           DERIVED
```

The three categories are the product's: **operational** data has retention and
migration rules and is never deleted as "cache"; **derived** data is deletable
at any time. The UI's "Clear cache" touches `index.db` and `cache/` only.

### 4.1 `registry.db` (schema 1)

The SQLite `registry` row stores the versioned JSON identity snapshot. The
shape below remains illustrative; serialized field details are owned by
`notes-core::registry`, not this example.

```json
{
  "schema": 1,
  "workspace_id": "3f0c…",
  "root": "/home/samir/Documents/notes",
  "root_native_id": { "Unix": { "dev": 66306, "ino": 1310722 } },
  "case_insensitive": false,
  "created_at": "2026-09-07T14:00:00Z",
  "notes": {
    "8a1d…": {
      "path": "trabalho/projetos.md",
      "size": 1234,
      "mtime_ns": 1725700000000000000,
      "hash": "b3:5d41…",
      "native_id": { "Unix": { "dev": 66306, "ino": 1310980 } },
      "rev": 7,
      "first_seen": "2026-09-07T14:00:01Z",
      "last_seen": "2026-09-07T15:12:40Z"
    }
  },
  "tombstones": {}
}
```

- Persisted immediately in a SQLite transaction; no registry debounce exists.
  `BEGIN IMMEDIATE` merges changes against the caller's baseline, preserving
  unrelated notes written by another process and refusing conflicting changes.
- Operational `registry.db` is separate from derived `index.db`. Both use WAL,
  a five-second busy timeout and transactional `user_version` initialization.
  A newer schema is refused before migration or journal changes.
- Migration retains `registry.json` and first copies `registry.json.bak-1`.
  Once populated, the DB is authoritative. Corrupt operational state is an
  error, never an empty registry. Future schema upgrades require a backup before
  changing operational data; schema 1 has no later migration yet.
- `recent.json` keeps the last 100 visited NoteIds independently of the index;
  current paths resolve through the registry. Rewrite originals and journals
  live in `reference-backups/` and are not cache.

### 4.2 Drafts

A draft is a byte-exact snapshot of a dirty buffer, written to app data when
the buffer cannot be, or has not been, written to the note. Sidecar:

```json
{ "schema": 1, "note_id": "8a1d…", "path": "trabalho/projetos.md",
  "buffer_version": 412, "base_rev": { "size": 1234, "mtime_ns": …, "hash": "b3:…" },
  "reason": "conflict" | "write_failed" | "stale" | "exit",
  "written_at": "…" }
```

Written when: a save is refused (conflict) or fails (I/O); a buffer has been
dirty for more than 30 s without a confirmed save; the app exits with dirty
buffers. Removed only after a confirmed write of a `buffer_version` ≥ the
draft's. On `note_open`, an existing draft is returned alongside the disk text
and the UI asks: restore draft / keep disk / compare.

### 4.3 Conflicts

`conflicts/<NoteId>/<iso-ts>-<local|disk>.md` plus a sidecar with `path`,
`base_rev`, and `reason`. Retention: **unresolved conflicts are never
auto-deleted**; resolved ones are kept 30 days (setting) then pruned; the
directory warns at 200 MB per workspace and never deletes to make room.

### 4.4 `session.json`

`{ schema, tabs: [{ note_id, path, cursor: {line, col}, scroll_top, pinned }],
active_tab, view_mode, sidebar: { open, width } }`. Resettable. Invalid JSON is
logged and ignored; the app starts with an empty session, never fails to open.

### 4.5 `settings.json`

```json
{ "schema": 1,
  "editor":   { "font_size": 14, "line_numbers": true, "word_wrap": true, "tab_size": 2 },
  "files":    { "autosave_ms": 750, "show_hidden": false },
  "markdown": { "default_view": "source", "raw_html": false, "remote_images": false },
  "ui":       { "locale": "auto", "theme": "dark" },
  "linux":    { "webkit_dmabuf_workaround": "auto" } }
```

Per-workspace overrides live in `registry.db` under `settings` (not shown
above) and, when the user enables it, in `.notes/config.json` (§16).

---

## 5. Write protocol

The frontend owns the buffer; the core owns the disk state. Every tab carries a
monotonic `buffer_version`. The frontend debounces (`autosave_ms`) and calls
`note_save(note_id, text, buffer_version, base_rev)`; `note_flush` is the same
call with no debounce.

**`base_rev` is explicit on the wire** — the `BaseRev` the buffer was read
against, returned by `note_open` or by the previous `Saved`. Scope §9 requires
every write to carry `(buffer_version, BaseRev)`, and it makes the app and
`notes-mcp` [0.3] a single write path with one contract instead of two: the
agent already has to send the base it read (scope §15), and a core-held
`open_rev` would have given the app a second, weaker rule. Saves are queued per document inside the core; at most one save
per document is in flight.

```
note_save(note_id, text, buffer_version, base_rev)
 1. take per-document async mutex
 2. take write.lock (§6)                                    ── LockTimeout → step 9
 3. encode: text + TextProfile → bytes (re-add BOM, EOL, final newline); hash
 4. read + hash disk under the lock (metadata equality is not proof of content)
       hash == new buffer hash              → convergence; return Saved unchanged
 5. compare disk hash to base_rev.hash (sent by the caller)
       equal                                → proceed; refresh size/mtime
       different                            → CONFLICT (step 8)
 6. fs.write_atomic(path, bytes, expect: Some(base_rev))    ── fs re-hashes immediately before rename
 7. registry: size, mtime, hash, rev += 1; arm self-write expectation (path, hash, ttl 2 s);
    delete draft if draft.buffer_version ≤ buffer_version;
    release; return Saved { base_rev, buffer_version }
 8. CONFLICT: write draft(reason: conflict); snapshot disk text to conflicts/<ts>-disk.md;
    mark autosave suspended for note_id; release; return Conflict { disk_rev }
 9. FAILURE (any I/O error, lock timeout): write draft(reason: write_failed);
    release; return WriteFailed { kind }
```

The frontend marks the tab **saved only if the returned `buffer_version`
equals the tab's current one**; otherwise it stays dirty and the next debounce
saves again. A stale save completing after new keystrokes can never paint the
tab clean.

Autosave stays suspended for a note until `conflict_resolve` runs. While
suspended, edits keep going to the draft (every debounce), never to the note.

### 5.1 Text profile

On `note_open`, the core detects `TextProfile`. `encoding: Unknown` (invalid
UTF-8) or `eol: Mixed` returns `read_only: true` with a reason; the editor is
disabled until the user runs `note_convert_eol(note_id, Lf | CrLf)` or
`note_convert_encoding` explicitly — those are the user asking, so they are
allowed to change bytes. BOM is stripped before the text reaches the frontend
and re-added by `profile.bom` on save. CodeMirror is configured with
`lineSeparator: "\n"`; the profile, not the editor, decides what hits the disk.

### 5.2 Atomic replace (`LocalFs`)

1. `tmp` in the same directory: `.<name>.tmp-<8 random>` — same volume, so
   rename is atomic and `same_volume_move` holds.
2. write, `fsync(tmp)`.
3. if `caps.preserve_mode`: copy mode bits from the original.
4. if `expect` is `Some`: `stat(path)`; mismatch → remove tmp, return
   `Conflict` (the narrow window between step 5 and 6 above).
5. rename-over. Windows: retry up to 5 × 50 ms on `ERROR_SHARING_VIOLATION` /
   `ERROR_ACCESS_DENIED` (antivirus, indexers).
6. `fsync(dir)` on Unix.

Backends without `atomic_replace` (SAF, see §11) implement `write_atomic` as
*write new document → verify by read-back → delete old → rename new*, and the
adapter reports `atomic_replace: false` so the UI can say "saving on this
storage is not crash-safe".

---

## 6. Cross-process lock

`write.lock` in the workspace's app-data directory, taken with an OS advisory
lock (`flock` on Unix, `LockFileEx` on Windows — crate `fd-lock`). It guards
the stat → compare → replace sequence and the registry update, and nothing
else; hold time is milliseconds.

- Timeout 5 s → `CoreError::LockTimeout`, handled as a write failure (draft
  written, UI shows error, retried on next change).
- No stale-lock problem by construction: advisory locks are released by the
  kernel when the holder dies. This is why it is a lock, not a pid file.
- One lock per workspace, not per note. Contention between the app and
  `notes-mcp` is rare and short; simplicity wins.
- The lock coordinates **our** processes. Third-party editors do not take it;
  their writes are handled by the base-rev check, not by the lock.

Until 0.3 there is one process. The lock exists from 0.1a anyway, so that the
protocol is exercised by tests before a second process arrives.

---

## 7. Command contract (Tauri ↔ core)

**One command per operation**, not a single `dispatch`. Tauri's command macro
gives typed arguments and per-command capability scoping; a dispatch would give
up both. Types cross the boundary as `serde` JSON; `ts-rs` derives the
TypeScript for every `notes-model` type into
`apps/notes-app/src/ipc/types.ts` at build time. The frontend never hand-writes
an IPC type.

### 7.1 Commands

| Area | Command | Returns |
|---|---|---|
| workspace | `workspace_open(root: String)` | `WorkspaceInfo { id, root, caps, case_insensitive, read_only }` |
| | `workspace_create(parent: String, name: String)` | `WorkspaceInfo` |
| | `workspace_recent()` | `Vec<RecentWorkspace>` |
| | `workspace_close()` | `()` — refuses with `DirtyBuffers` while tabs are dirty |
| | `workspace_reindex()` `[0.2]` | `()` |
| tree | `tree_list(dir: RelPath)` | `Vec<Entry { path, kind, size?, is_note, is_symlink }>` — one level, lazy |
| notes | `note_open(path: RelPath)` | `OpenedNote { note_id, text, profile, base_rev, read_only: Option<reason>, draft: Option<DraftInfo> }` |
| | `note_save(note_id, text, buffer_version, base_rev)` | `SaveResult` = `Saved { base_rev, buffer_version, unchanged }` \| `Conflict { disk_rev }` \| `WriteFailed { kind }` |
| | `note_flush(note_id, text, buffer_version, base_rev)` | `SaveResult` |
| | `note_reload(note_id)` | `OpenedNote` |
| | `note_close(note_id, buffer_version)` | `()` |
| | `note_create(dir: RelPath, name: String)` | `Entry` — `AlreadyExists` on collision, by `CompareKey` |
| | `note_convert_eol(note_id, eol)` | `OpenedNote` |
| | `conflict_resolve(note_id, choice: KeepLocal \| UseDisk \| SaveAsCopy)` | `OpenedNote` |
| | `draft_restore(note_id, choice: Restore \| Discard)` | `OpenedNote` |
| entries | `dir_create(dir, name)` | `Entry` |
| | `entry_rename(path, new_name)` | `Entry` — same `NoteId` |
| | `entry_move(path, to_dir)` | `Entry` — same `NoteId`; `Unsupported` across volumes in 0.1 |
| | `entry_duplicate(path)` | `Entry` — new `NoteId`; `create_new`, never overwrites |
| | `entry_delete(path)` | `DeleteOutcome { Trashed \| Permanent }` |
| preview | `markdown_render(note_id, text)` | `Rendered { html, outline }` — text passed so unsaved buffers preview |
| | `markdown_outline(note_id, text)` | `Vec<Heading>` |
| search | `search_quick(query)` | `Vec<QuickMatch { path, score }>` — in-memory fuzzy |
| | `search_global_start(query, opts)` | `SearchId`; results arrive as events |
| | `search_cancel(id)` | `()` |
| session | `session_get()` / `session_save(Session)` | |
| settings | `settings_get()` / `settings_set(Settings)` | |

### 7.2 Events (core → frontend)

`fs:changed { path, kind, note_id? }` · `note:conflict { note_id, kind: Modified | Removed }` ·
`note:saved { note_id, base_rev }` · `workspace:unavailable { root, reason }` ·
`workspace:available` · `search:result { id, path, line, col, context }` ·
`search:done { id, total, cancelled }` · `index:progress { done, total }` `[0.2]`.

### 7.3 Error model

Every command returns `Result<T, CoreError>`. The frontend switches on `code`
and maps it to an i18n key; it never inspects `message`.

```rust
#[derive(Serialize, TS)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum CoreError {
    InvalidPath        { path: String, reason: String },
    OutsideRoot        { path: String },
    SymlinkNotFollowed { path: String },
    NotFound           { path: String },
    AlreadyExists      { path: String },
    DirtyBuffers       { note_ids: Vec<NoteId> },
    Conflict           { note_id: NoteId, disk_rev: BaseRev },
    ReadOnly           { note_id: NoteId, reason: ReadOnlyReason },   // NotUtf8 | MixedEol | Workspace
    Unavailable        { root: String, reason: String },
    LockTimeout,
    Unsupported        { cap: String },
    Io                 { op: String, path: String, kind: String, message: String },
    Internal           { message: String },
}
```

`Internal` is a bug, not a state; CI fails a test that expects it.

---

## 8. Reconciliation

Sources of truth for "did the disk change": `stat`, then hash. Everything else
is a hint that schedules a reconciliation.

Triggers: watcher event (when `caps.watch`); window focus; tab switch; manual
refresh; poll timer every 5 s in foreground when `!caps.watch`; full scan on
return from background `[0.4]`.

```
raw watcher events ─▶ notify-debouncer (200 ms) ─▶ normalize to FsEvent { Created | Modified | Removed | Renamed { from, to } }
  ─▶ drop ignored entries (§16 IGNORE_DEFAULT) and our own temp files (.*.tmp-*)
  ─▶ self-write filter: (path, hash) matches an armed expectation, not expired → consume expectation, drop event
  ─▶ per path: stat; if (size, mtime) ≠ registry → hash; if hash ≠ registry → real change
  ─▶ dispatch:
       Modified, note open, buffer clean   → registry update; emit fs:changed (frontend reloads, keeps cursor)
       Modified, note open, buffer dirty   → suspend autosave; draft; emit note:conflict { Modified }
       Modified, note not open             → registry update
       Removed,  note open, buffer dirty   → keep buffer; draft; emit note:conflict { Removed } — never recreate the path
       Removed / Created                   → identity correlation (§9), then registry update
```

The self-write expectation is **armed before the write, not after** — otherwise
there is a window exactly as long as the write in which our own save is news —
consumed on its first match and expired after 2 s, so an external write that
lands right after ours is seen, not swallowed.

**`Created` is emitted only for a hinted path.** The table above was written
against a registry that knows every file; this one is populated when a note is
*opened* (§9's `observe`, `DECISIONS-0.1a.md` D-09), so on a full scan every
note the user has never opened is "not in the registry". Announcing them would
report the whole workspace as created on every window focus. A scan re-lists the
tree, which is what the sidebar needs, and a file that appeared as half of a
rename is found by correlation, which walks for it — [ADR-026](decisions.md).

Budget: at most 50 files hashed per reconciliation tick; the rest is queued.
The UI never waits on reconciliation to open or edit a note.

`inotify` limits: on `ENOSPC` from `max_user_watches`, the watcher degrades to
poll for that workspace and the UI says so with the `sysctl` to raise it.
**Not being able to watch is a state of the workspace rather than a failure of
the call**: `FileSystem::watch()` returns a `Watch` carrying an optional
`degraded` reason, never an `Err`, so a backend with no watcher opens normally
and is reconciled by the 5 s poll and the scan on focus ([ADR-027](decisions.md)).
The poll and the focus scan run whether or not there is a watcher, so a watch
silently lost degrades to 5 s rather than to nothing.

### The one-second rule, and what runs in the background

**`workspace_open` returns and the tree appears in under one second, at any
size.** This is an acceptance criterion, not an aspiration
([ADR-034](decisions.md), `ACCEPTANCE-0.1b.md` §6, `ACCEPTANCE-0.1c.md` §3), and
it is stated in this section because reconciliation is where it was broken.

Everything that has to walk the **whole tree** runs off the critical path — on
its own thread, cancellable, with its progress visible in the status bar:

| Work | Where | Reported as |
|---|---|---|
| one watch per directory (**Linux only**) | `notes-fs::watch`, thread `notes-watch` | `WatchStatus { walking, dirs, unreadable, over_limit }` |
| the quick-open path list | `notes-core::index`, thread `notes-index` | `QuickOpen { indexed, building, unreadable }` |

**The per-directory walk is Linux's.** An inotify watch descriptor covers exactly
one directory, so `notify`'s recursive mode is a walk it performs for you.
FSEvents and `ReadDirectoryChangesW` watch a **subtree from one handle**: there
the root is watched recursively in one O(1) call and there is no walk, no skip
list and neither of the two failure modes below — running the Linux path on them
would trade one handle for hundreds of thousands (`DECISIONS-0.1c.md` D-10).

Both walks use an explicit stack and `symlink_metadata`, so a symlinked
directory is never descended into and a loop cannot be entered. Both **count and
skip** a directory they cannot read rather than failing: one unreadable
subdirectory is a number in the status bar, never a reason to stop indexing or
to demote a workspace to polling.

The watcher does not descend into `node_modules`, `target`, `vendor`, `dist`,
`build`, `.git`, `.svn`, `.hg`, `.cache` or `__pycache__`. That list is **not**
`IGNORE_DEFAULT` (§16) and must not be confused with it: `IGNORE_DEFAULT` decides
what the user *sees*, and a watch is a finite kernel resource of which a
machine-generated tree can consume hundreds of thousands. A change inside a
skipped directory still arrives through the 5 s scan
(`DECISIONS-0.1c.md` D-08).

A full watch table degrades **only the excess**. Watches already installed keep
working, the remainder is counted in `over_limit`, and the banner names the
number and the `sysctl` that raises the limit — rather than the whole workspace
dropping to polling, which is what happened before.

Where each number surfaces follows from whether it settles: the **status bar**
carries the walk's progress, which is transient and gone when the walk ends;
**banners** carry `over_limit` and `unreadable`, which do not go away and need a
sentence and a command rather than a corner.

Both background walks are **cancellable**, and dropping their owner is what
cancels them — a `Watch` for the watcher, a `PathIndex` for quick open. Closing
a workspace must not leave a thread walking a folder nobody has open.

A walk that is still running is **not** restarted by an invalidation, however.
§16's rule that every tree change drops the quick-open list was written for a
30 ms walk inside the call; against a walk of seconds it means any workspace
with a build running in it never finishes indexing. The staleness is remembered
and acted on when the walk ends (`DECISIONS-0.1c.md` D-11).

**Why this is a rule and not a note.** Opening a folder of ~160 repositories
froze the Welcome screen for over two minutes. The tree was not at fault: on
20 962 directories, `open_workspace` plus listing the root costs **1.13 ms**,
because the tree is lazy. `start_watch` cost **503 ms** and the first
`quick_open` **550 ms**, and both paid it inside a `#[tauri::command]` holding
`Mutex<WorkspaceService>` — so every other command, `tree_list` included, waited
behind them. `tools/gen-deep.sh` and `crates/notes-core/tests/deep.rs` keep that
measurement reproducible.

---

## 9. Identity correlation

Runs after any scan or Removed/Created pair. Renames the app performs itself
never enter this algorithm; they update the registry directly.

**Computed from the vanished side.** "Disk paths not in the registry" is nearly
every file here, because the registry is lazy; driving the loop from what
vanished gives the same answer and costs nothing on every tick where nothing
has ([ADR-026](decisions.md)).

```
vanished := registry paths not on disk
appeared := disk paths not in registry
for each a in appeared:
  1. if caps.native_id and exactly one v in vanished has v.native_id == a.native_id      → a takes v.NoteId
  2. else if a.size > 0 and exactly one v has v.hash == a.hash
          and a is the only appeared with that hash                                        → a takes v.NoteId
  3. else                                                                                  → a gets a new NoteId
remaining vanished → dropped from registry (0.1); become tombstones in 0.6
```

Zero-byte files are never correlated by hash. Two candidates on either side is
ambiguity, and ambiguity means a new id — re-identifying is cheaper than
attaching a note to the wrong history.

---

## 10. Markdown IR

Decision: **sanitized HTML crosses the IPC for preview; a slim `Document`
crosses for outline and links. The full AST does not.** The frontend contains
no Markdown parser; `ammonia` runs at the one boundary that matters.

```rust
// notes-markdown
pub fn parse(src: &str) -> Document;
pub fn render_html(src: &str, opts: &RenderOpts) -> Rendered;

pub struct Document {
    pub front_matter: Option<Span>,           // raw byte span; not parsed here
    pub headings: Vec<Heading { level: u8, text: String, slug: String, span: Span }>,
    pub links: Vec<Link { target: String, kind: LinkKind, span: Span, in_code: bool }>,
    pub tasks: Vec<Task { checked: bool, span: Span }>,
    pub tags: Vec<String>,                    // [0.3]
}
pub enum LinkKind { RelativePath, Url, Anchor, Wiki /* [0.3] */, Refused }

pub struct RenderOpts { pub base: RelPath, pub raw_html: bool, pub remote_images: bool, pub workspace_id: WorkspaceId }
pub struct Rendered { pub html: String, pub outline: Vec<Heading>, pub blocked_remote: Vec<String> }
```

Front matter preservation is not this crate's job: the byte policy in §5.1
keeps it intact because nothing rewrites the buffer. This crate only reports
its span, and **only for a block at byte 0** — the metadata extension is enabled
per document, because `pulldown-cmark` will otherwise swallow any `---`-fenced
block anywhere in the note.

Pipeline: strip front matter (span kept) → `pulldown-cmark` with `TABLES |
STRIKETHROUGH | TASKLISTS | FOOTNOTES` → event rewrite pass → HTML →
`ammonia` with an allowlist → `Rendered`.

**There are two layers on purpose.** The rewrite pass decides what each
destination may become; `ammonia` then applies a closed allowlist that knows
nothing about notes. Either would do on a good day; together, a mistake in one
has to coincide with a hole in the other to reach a user.

The rewrite pass is the security policy in code:

- `Html` / `InlineHtml` events → emitted as escaped text unless `raw_html`
  (which still goes through `ammonia`).
- Image URLs: relative and resolving inside the root →
  `notes-asset://<workspace_id>/<relpath>`; `http(s)` → kept only if
  `remote_images`, else replaced by
  `<span class="blocked-image" data-blocked-src="…">URL</span>` and listed in
  `Rendered.blocked_remote`; `data:` only for the raster allowlist —
  `image/png`, `image/jpeg`, `image/gif`, `image/webp`, and **never
  `image/svg+xml`**, which is a scriptable document; any other scheme, and a
  relative path that escapes the root, → dropped, **with the alt text kept as
  text** so nothing the user wrote disappears silently.
- Link URLs: relative → kept, with `data-note-path` so the frontend opens the
  note in-app, and `data-note-anchor` beside it when the link named a
  `#section` (scope §8.3's `caminho/relativo.md#titulo-opcional`); `http(s)` →
  kept, `target=_blank rel="noopener noreferrer"`, opened by the shell's
  `shell:open` which only accepts `http(s)`; **`mailto:` and every other scheme
  → dropped, link text kept.** Scope §8.4 says *"outros esquemas recusados"*,
  and the capability file agrees: a rendered `mailto:` would be a link that does
  nothing when clicked. Widening `shell:allow-open` is the owner's act
  (`DECISIONS-0.1b.md` D-06).
- A bare `https://` or `http://` in prose is linkified — scope §8.1 lists
  autolinks. Never inside code, never inside an existing link, and **`www.`
  without a scheme is not**: GFM guesses `http://` for it, and guessing an
  insecure scheme for the user is not something this application does quietly.
- Task list items → `<input type="checkbox" disabled>`. `ammonia` **forces**
  both attributes on every `<input>` rather than allowing them, so a raw
  `<input name=… value=…>` in a workspace that has opted into raw HTML cannot
  become a control that accepts anything.
- Code blocks → `<pre><code class="language-x">`. Table cells may carry a
  `style` attribute holding one of exactly three alignment strings, enforced by
  an attribute filter; every other `style` is dropped.

Heading slugs follow the GitHub algorithm (lowercase, strip punctuation, spaces
to `-`, dedupe with `-n`). `Document.links` with `in_code: true` are reported —
including link-shaped text found inside code spans and fenced blocks, so the
inventory is complete — and never rewritten by the 0.2 rename tool, nor counted
as tags in 0.3.

`notes-asset://` is a Tauri custom URI scheme registered in `src-tauri`; its
handler calls `notes-core`, which applies the same root jail as every other
path and serves only file types in the image allowlist.

**The corpora are the specification.** `fixtures/markdown/` holds an input and
its exact expected HTML and `Document` side by side; `fixtures/xss/` holds
payloads, each rendered under all four combinations of `raw_html` and
`remote_images` and checked structurally — tags and attributes read back out of
the output, never substrings, because `safe-in-code.md` must render
`javascript:alert(1)` **as text**.

---

## 11. Filesystem capability matrix

The adapter reports `Caps` per root; the core adapts behaviour, the UI states
limitations. Unknown filesystem → most conservative row.

| Backend | atomic_replace | trash | watch | native_id | preserve_mode | Notes |
|---|---|---|---|---|---|---|
| ext4 / btrfs / xfs | yes | yes (freedesktop) | inotify | dev+ino | yes | `max_user_watches` may be low → detect `ENOSPC`, fall back to poll |
| APFS / HFS+ | yes | yes | FSEvents | dev+ino | yes | case-insensitive by default; NFD names arrive from Finder; compare via `CompareKey` |
| NTFS | yes, with retry | yes (Recycle Bin) | ReadDirectoryChangesW | volume+index | n/a | replace can hit open handles → §5.2 retry |
| exFAT / FAT (removable) | yes | no | yes | unreliable | no | mtime resolution 2 s → always hash when size is equal |
| SMB / NFS mounts | rename ok, fsync unreliable | no | usually no → poll | unreliable | partial | report `atomic_replace: false` unless verified |
| App sandbox (iOS/Android) `[0.4]` | yes | no | no → poll | ino (iOS) / none | n/a | default for "Create Workspace" on mobile |
| iOS security-scoped bookmark `[0.4]` | via `NSFileCoordinator` | no | no → poll | no | no | writes coordinated; bookmark refreshed when stale |
| Android SAF tree `[0.4]` | **no** | no | no → poll | document id | no | no rename-over; `write_atomic` = write new + verify + delete old + rename; permission may vanish when the document is moved |

Detection: Linux `statfs().f_type`; macOS `pathconf` + `statfs`; Windows
`GetVolumeInformationW`. The result is cached in `registry.db` and
re-probed when `root_native_id` changes.

**Reading the id.** Unix takes `dev` and `ino` straight from the `stat` already
performed. Windows needs a **handle**, so the file is opened to ask —
`access_mode(0)` (a query, not a read, so a file another process holds open
still answers), `FILE_FLAG_BACKUP_SEMANTICS` so that a directory can be opened
at all, and `FILE_FLAG_OPEN_REPARSE_POINT` so a symlink reports its own identity
rather than its target's, matching the `symlink_metadata` the rest of `Stat` is
built from. Because that costs an open, **the id is filled in by `stat`, not by
`list`**: `Entry` carries none, and correlation asks one path at a time.

---

## 12. Tauri shell

`src-tauri` is thin by rule (ADR-003). It contains:

- `main.rs` — builds `WorkspaceService` with `default_data_dir()`, registers
  commands, the asset protocol, and the Linux startup hook.
- `commands/*.rs` — one function per §7.1 row: parse → call core → map
  `CoreError` to the response. No branching on business state.
- `asset_protocol.rs` — `notes-asset://` handler (§10).
- `linux.rs` — before the WebView exists: if `WAYLAND_DISPLAY` is set and an
  NVIDIA driver is present (`/proc/driver/nvidia/version`, `/sys/module/nvidia`,
  or `nvidia-smi` on `PATH`), set `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Logged
  once. This is the black-window / flicker fix for WebKitGTK on Wayland + NVIDIA
  and is an acceptance item of milestone 0.0.

  **At 0.0 the workaround is unconditional** on that hardware: `settings.json`
  belongs to 0.1a (§4.5, §17), so gating 0.0 on a settings key would gate it on a
  file that does not exist. From 0.1a the gate is
  `settings.linux.webkit_dmabuf_workaround` (`auto` | `off` | `force`), read
  before the WebView, and **a missing or unreadable settings file degrades to
  `auto`, never to `off`** — the failure mode of not applying it is a black
  window, and the failure mode of applying it needlessly is slightly slower
  compositing. An explicit `WEBKIT_DISABLE_DMABUF_RENDERER` already in the
  environment is never overridden.

Capabilities (`src-tauri/capabilities/default.json`): `core:default`,
`dialog:allow-open` (directories only), `clipboard-manager:allow-read-text`,
`clipboard-manager:allow-write-text`, `shell:allow-open` scoped to
`^https?://`, and the `notes-asset` scheme. **No `fs:*` permission exists in
the file.** A PR that adds one is rejected by a CI grep.

CSP (`tauri.conf.json`):

```
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' notes-asset: data:; font-src 'self'; connect-src ipc: 'self';
frame-src 'none'; object-src 'none'; form-action 'none'
```

`'unsafe-inline'` for styles is a CodeMirror requirement; scripts stay strict.

One window, one workspace, no tray, no updater in the MVP (an updater needs
network and gets its own ADR when it comes).

---

## 13. Frontend structure

- `ipc/` wraps every command in a typed function using the generated
  `types.ts`; nothing else imports `@tauri-apps/api/core`.
- Stores (Zustand): `workspace` (info, caps, availability), `tabs` (open notes,
  `buffer_version`, dirty, conflict, cursor, scroll), `editor` (settings-derived
  view), `search`, `ui` (sidebar, view mode, palette). Persistence goes through
  `session_save`, debounced 1 s, and on `beforeunload`.
- Editor: CodeMirror 6 with `lineSeparator: "\n"`, `@codemirror/lang-markdown`
  with GFM, history, search, multiple selections, `EditorView.lineWrapping`
  toggled by settings, custom keymap from `app/keymap.ts`. The document text is
  the single source for `note_save`; the store never holds a second copy.
- Preview: a `<div>` receiving `Rendered.html` via `innerHTML` (already
  sanitized by the core); a click handler intercepts `a[data-note-path]` and
  calls `note_open`. Rendering is debounced 300 ms and skipped while the tab is
  hidden.
- i18n: `i18n/en.json`, `i18n/pt-BR.json`; every user-visible string is a key;
  a CI check fails on a key missing from either file.

---

## 14. Testing

| Layer | What | Where it runs |
|---|---|---|
| `notes-fs` | root jail (`..`, absolute, symlink, NFD collision); atomic replace under kill; property test `read(write_atomic(x)) == x` for arbitrary bytes | all four CI OS |
| `notes-core` | write protocol state machine with a fake `FileSystem` (conflict, convergence, stale save, lock timeout); identity correlation cases; draft lifecycle; reconciliation with synthetic events | Linux (fast) + all OS for the real-fs subset |
| `notes-markdown` | golden HTML for `fixtures/basic`; every file in `fixtures/xss` produces no `<script>`, no `javascript:`, no remote fetch, no `file:` | Linux |
| crash | `tools/crash-save-loop`: spawns a writer against `fixtures/large`, `SIGKILL`s at random points 1000×, asserts no truncated or empty note | all four CI OS, nightly and on release |
| byte-preservation | open → save-unchanged for every file in `fixtures/basic` + `edge-cases`; `git status --porcelain` must be empty | all four CI OS |
| frontend | `vitest` for stores and `ipc/` mapping; a CI check that regenerated `ipc/types.ts` matches the committed file; WebDriver e2e after 0.1c | Linux |

CI matrix: `ubuntu-latest`, `archlinux:latest` container (installs
`webkit2gtk-4.1`, `gtk3`, `rustup`, builds the Tauri app — rolling breakage
shows here first), `macos-latest`, `windows-latest`. Steps: `cargo fmt --check`,
`cargo clippy -D warnings`, `cargo test --workspace`, `npm run lint`,
`npm run typecheck`, `npm test`, `cargo tauri build`. A desktop milestone is not closed
without green on all four (`docs/roadmap.md` §19 rule).

`fixtures/large` is generated in CI by `tools/gen-large.sh` (deterministic
seed) and never committed.

---

## 15. Distribution

`.github/workflows/release.yml` tags and publishes a GitHub Release for every
`version.md` bump via `tools/release.sh`. `.github/workflows/build.yml` builds
the artifacts and attaches them to that Release.

**They are two workflows on purpose.** Publishing a Release must not wait on, or
be failed by, a compiler: the Release is the record that a version exists, and a
build that breaks should leave a Release with notes rather than no Release at
all. `build.yml` triggers on `workflow_run` rather than `on: release`, because
`release.yml` creates the Release with the built-in `GITHUB_TOKEN` and GitHub
fires no workflow events for what a `GITHUB_TOKEN` did.

**Artifacts are built for a minor bump, and on request** ([ADR-036](decisions.md)).
Every commit is a version and every version gets a Release, so most versions are
a step inside a working session; nine minutes and a 105 MB AppImage for each of
those buys nobody anything. A version whose patch component is `0` is built, as
is any version asked for through `workflow_dispatch`. **A patch Release says in
its own description that it carries no artifacts** and how to get them, because
an empty downloads section otherwise reads as a build that failed.

Builds are also serialised — the concurrency group is `build` with
`cancel-in-progress` — so even a burst of minor bumps produces one build.
Completion is judged by the `.SRCINFO`, the last thing uploaded, so a cancelled
run is rebuilt rather than mistaken for a finished one; every upload uses
`--clobber`.

| Target | Artifact | Signing | Status |
|---|---|---|---|
| Debian / Ubuntu | `.deb` (built on `ubuntu-22.04` for the oldest glibc still supported), AppImage | none required | **shipping** |
| Arch Linux | `packaging/aur/notes-bin/PKGBUILD` consuming the release tarball (binary, `.desktop`, icons); `notes-git` optional | AUR account; `makepkg` and `--printsrcinfo` run in an `archlinux:latest` container | **shipping** — `depends=(webkit2gtk-4.1 gtk3)`, and the job installs the package and checks `ldd` resolves |
| macOS | `.dmg` (universal) | Developer ID + `notarytool` | **written and disabled** — needs an Apple Developer Program membership and the certificate; ADR-024 |
| Windows | NSIS installer (MSI later if asked) | OV code-signing certificate, `signtool` | **written and disabled** — needs the certificate; ADR-024 |
| iOS `[0.4]` | TestFlight → App Store | Apple Developer | built on the macOS runner |
| Android `[0.4]` | APK on the Release; Play later | upload key in CI secrets | built on Linux |

The disabled jobs are written out in `build.yml` behind `if: false`, each with
the list of what is missing. It is an account and a certificate, not
engineering, and a job that does not exist is a job nobody can cost.

**The version comes from `version.md` and from nowhere else.**
`tools/stamp-version.sh` writes it into `tauri.conf.json` at build time; the
committed value is `0.0.0` and both CI and `tools/check.sh` fail if it is
anything else. The `PKGBUILD` is generated the same way, checksum included
([ADR-035](decisions.md)).

The release tarball is unpacked **from the `.deb`** rather than assembled:
the binary, the `.desktop` entry and the icon set are produced by the Tauri
bundler, and a second copy of the desktop entry here would be a second version
of one rule.

Secrets (certificates, keys) live in GitHub Actions secrets, never in the
repository (CLAUDE.md golden rule 7).

---

## 16. `.notes/` portable configuration

### The default ignore list is a constant, not configuration

`IGNORE_DEFAULT` lives in `notes-core` and applies to every workspace whether or
not `.notes/` exists:

```rust
// notes-core::ignore
pub const IGNORE_DEFAULT: &[&str] = &[".notes", ".git", ".obsidian", ".trash"];
// plus, by rule: any entry whose name starts with '.', and our own temp files
// (`.<name>.tmp-<8 random>`).
```

Scope §7.6 requires this list by default, and `.notes/` is off by default, so the
list cannot live there — a default that only exists once the user opts in is not
a default. The user's `.gitignore` is never read as a visibility policy.

**`node_modules/` and `target/` are not in it, and that is deliberate.** They
hold real Markdown, and hiding a folder by name is the application deciding
which of the user's files are real. They *are* in the watcher's skip list (§8),
which is a different list answering a different question — a watch is a finite
kernel resource; visibility is not ([DECISIONS-0.1c.md](DECISIONS-0.1c.md)
D-08).

### `.notes/config.json`

Off by default; opening a folder creates nothing in it. When the user enables
"portable workspace settings", the app writes:

```json
{ "schema": 1, "ignore": ["build", "node_modules"], "attachments_dir": "attachments" }
```

Rules: read on open if present; **`ignore` extends `IGNORE_DEFAULT` and never
replaces it** — there is no way to make the app show `.git/`, because a config
file that can unhide the repository's internals is a foot-gun with no use case;
other keys override the per-workspace settings in app data; invalid JSON →
logged, defaults used, the file is left alone. Never contains ids, index data, drafts, or anything the app needs to
open the folder. Fine to commit to Git; merge conflicts inside it are the
user's, and the app tolerates them by falling back to defaults.

---

## 17. Milestone map

`●` planned, `✔` shipped.

| Section | 0.0 | 0.1a | 0.1b | 0.1c | 0.2 | 0.3 |
|---|---|---|---|---|---|---|
| §3 types, §4.1 registry (JSON), §4.2 drafts, §4.5 settings | | ● | | | | |
| §5 write protocol, §5.1 text profile, §5.2 atomic replace | | ● | | | | |
| §6 lock | | ● | | | | mcp uses it |
| §7 commands: workspace, tree, note_open/save/flush/close/create, dir_create | | ● | | | | |
| §2 notes-markdown, §10 IR, §7 markdown_render/outline (preview, split) | | | ✔ | | | |
| §7 commands: rename, move, duplicate, delete, conflict_resolve, draft_restore, convert_eol | | | ✔ | | | |
| §8 reconciliation (watcher, focus), §9 correlation, §4.3 conflicts | | | ✔ | | | |
| §7 search_*, session_*, settings_* UI, §13 i18n | | | | ● | | |
| §4.1 registry.db, §2 notes-index, link rewrite | | | | | ● | |
| §2 notes-mcp, tags, wiki links, attachments | | | | | | ● |
| §12 linux.rs workaround, §15 Arch job | ● | | | | | |
| §11 mobile rows, §4 mobile adapters | spike | | | | | 0.4 |

---

## 17.1 Points where this document overrides the Portuguese scope

Resolved by the owner on 07/09/2026; `.continue/SCOPE_final.md` is not edited
(the queue is not rewritten), so the divergence is recorded here.

| Scope | Says | This document | Resolution |
|---|---|---|---|
| §5 crate list | `notes-markdown` untagged, so 0.1a by the diagram's convention | `[0.1b]` (§2, §17) | **This document wins.** Its first consumer is the 0.1b preview; front matter stays intact at 0.1a through the byte policy (§5.1), not through a parser |
| §12 | Four resolutions: compare · keep mine · use disk · save as copy | `KeepLocal \| UseDisk \| SaveAsCopy` (§7.1) | **This document wins.** "Compare" is 0.1b UI over data the frontend already has; it changes nothing on disk, so it is not a command |
| §12 | Draft written on conflict or write failure | Also on 30 s dirty, and on exit (§4.2) | **This document wins** — a superset. More situations in which a buffer survives cannot make the guarantee weaker |
| §9 | Every write carries `(buffer_version, BaseRev)` | `note_save(..., base_rev)` (§5, §7.1) | **The scope wins**; this document was changed to match |
| §7.6 | Default ignore list | `IGNORE_DEFAULT` in `notes-core` (§16) | Same list; the scope did not say where it lives |
| §8.4 | "Outros esquemas recusados" | `mailto:` renders as **text**, not as a link | **The scope wins, and it is worth spelling out**: `shell:allow-open` permits only `http(s)`, so a rendered `mailto:` would be a link that does nothing. Widening the capability is the owner's act ([ADR-029](decisions.md)) |
| §8.1 | "autolinks" among the GFM features | A bare `https://`/`http://` is linkified; a bare `www.` is **not** | GFM guesses `http://` for `www.`, and guessing an insecure scheme for the user is not something this application does quietly |
| §7.7 | Delete reports which happened | `DeleteOutcome`, and the trash is attempted whenever `caps.trash` | Same rule; the fallback exists because a removable stick has no bin, and it is never silent ([DECISIONS-0.1b.md](DECISIONS-0.1b.md) D-11) |

## 18. Decisions this document introduces (record as ADRs)

To be added to `docs/decisions.md` in the commit that makes this file `ACTIVE`;
numbers continue from the last ADR there.

1. **Crate set extends ADR-003**: `notes-model`, `notes-markdown` added;
   `notes-index` created at 0.2, `notes-mcp` at 0.3, `notes-sync` at 0.6 —
   no crate exists before the milestone that uses it.
2. **Identity never enters a note file, and the hash is not identity.** No
   `id:` in front matter, not even when sync is enabled; identity lives in the
   registry and, later, the server; the content hash is a correlation signal
   with explicit ambiguity rules (§9). **Amends ADR-005**, whose Decision reads
   "a file eventually carries `file_id` … `content_hash` …" — both halves are
   amended by this one ADR, together with item 9, so ADR-005 is amended once.
3. **Registry is operational state, separate from the index**: JSON in 0.1,
   `registry.db` (own SQLite file, WAL) from 0.2, so that deleting the index can
   never touch identity. **The `index.db` location is already ADR-012** and is
   not recorded again here.
4. **One data directory owned by the core** (`dirs::data_dir()/notes`), shared
   by the app and `notes-mcp`; Tauri's `app_data_dir()` is not used.
5. **Cross-process coordination by advisory lock** (`write.lock`, flock /
   LockFileEx), per workspace.
6. **Preview IR is sanitized HTML; outline/links use a slim `Document`**; the
   frontend has no Markdown parser.
7. **Symlinks and junctions are not traversed** in the MVP.
8. **One Tauri command per operation; types generated with `ts-rs`.**
9. *(folded into item 2 — one ADR amends ADR-005 once.)* The rule stands: an
   external rename reconnects a `NoteId` only on a unique native-id or unique
   non-empty-hash match.
10. **Autosave and the base-rev guard ship together** in 0.1a; drafts and
    conflict snapshots are operational data with retention rules.
11. **Linux WebKitGTK workaround** applied automatically on Wayland + NVIDIA,
    unconditional at 0.0 and gated by a setting that degrades to `auto` from
    0.1a.
11b. **Arch Linux is a release target** via AUR, with its own CI job against
    rolling `webkit2gtk-4.1`. (Separate subject from 11; separate ADR.)
12. **Distribution requires signing**: no unsigned macOS or Windows artifact is
    published.

## 19. Implemented index and reference commands (0.2)

ADR-039 settles the crate boundary deferred by ADR-003. `notes-index` receives
facts and text; only core traverses and reads notes through `notes-fs`. Its
`plan()` returns cached metadata; core compares `(size, mtime)`, reads/hashes
changed files and invokes `apply()` with whether parsing is required. An
unchanged hash updates metadata only. The derived schema stores path metadata,
serialized parser documents (including links/images and headings), and FTS5.
NoteIds/revisions stay authoritative in the operational registry; independent
SQL tag/graph tables are deferred to 0.3 rather than maintained empty.

`index_start(force)`, `index_status()` and `index_cancel()` implement a worker
thread and polled progress, not an event channel. A per-workspace `index.lock`
serializes writers; readers use WAL. Complete scans prune missing paths;
unreadable directories do not authorize pruning. UI polls at 500 ms and starts
incremental checks every ten seconds while the sidebar is mounted. Saves and
reconciliation mark results stale until the next scan. Workers remain off the
service mutex, so note editing is independent of indexing.

`recent_notes()` lists operational recent history. Existing `markdown_outline`
parses the active buffer for Outline. `SearchMode::Words` queries FTS5; Literal
and Regex retain their scanner. `reference_preview(from,to)` returns an opaque
workspace-bound token and source spans; `reference_apply(token,selected)`
validates hashes, saves originals, relocates with stable IDs and applies chosen
edits with per-file guarded writes. This is explicitly not an atomic multi-file
transaction. See [ACCEPTANCE-0.2.md](ACCEPTANCE-0.2.md) for semantics, recovery,
limits and measured performance; this section supersedes earlier proposed
0.2 command names in §7.

## Milestone 0.3 implementation

[KNOWLEDGE-0.3.md](KNOWLEDGE-0.3.md) specifies the implemented parser, wiki
resolution, bounded graph, attachments and scoped stdio MCP surfaces. Index
schema 2 invalidates old derived parser documents; registry schema stays 1.
`notes-mcp` depends on core and has no Tauri or frontend dependency. Both
processes use the same app-data directory, enrollment lock, identity lock and
guarded save protocol. ADR-041 records these choices and their limits.

## Milestone 0.5 — optional process boundary

`server/notes-server` is the independent REST/CLI executable introduced in
0.18.0. It depends on notes-core, not Tauri. Core owns filesystem paths, scopes,
conditional mutations, identity and offline enrollment rebinding; the server
owns HTTP/authentication, request limits, audit and backup transport. Deployment
and the versioned API are in [SERVER-0.5.md](SERVER-0.5.md); ADR-043 records the
security boundary. No desktop listener or synchronization engine is added.

## Milestone 0.6 — causal domain boundary

The first 0.6 block adds `notes-sync` as a dependency of core, beside `notes-fs`.
It supplies revision/ancestry planning and locked operational metadata storage;
core supplies source inventories. `notes-sync-plan` is a core-owned CLI for
explicit pairing previews. The 0.19.1 server inbox transports immutable revisions and bytes under existing
authentication, without applying them to source files. The 0.20.0 `notes-sync-client` owns HTTPS and durable device queues, consuming
core capture and the shared publication contract (ADR-046). In 0.20.1, explicit
application uses core guards and a separate durable client checkpoint; shared
workspace activity leases protect open cooperating processes (ADR-047).
In 0.20.3, a separate client command sends durable application receipts through
the authenticated server inbox, with credential-bound devices and a resumable
local acknowledgment cursor (ADR-048). Later sections describe active-editor
application and explicit uploader conflict resolution. ADR-044 and [SYNC-0.6.md](SYNC-0.6.md) define this implemented boundary.


The 0.20.5 core seam admits an exclusive workspace before buffers are opened and
accepts observed buffer snapshots for guarded application without closing that
session (ADR-049). Version 0.20.5 supplied only the core host API; 0.20.6 wires
it to Tauri/React and the received-queue client.
The host must freeze editing and reload clean buffers; shared sessions cannot
apply or upgrade in place. See the host contract in [SYNC-0.6.md](SYNC-0.6.md).


In 0.20.6 the Tauri shell delegates received queue/session operations to
`notes-sync-client`. It shares the core service mutex with ordinary commands;
there is no second workspace service for app application. React's synchronous
input/IPC barrier spans snapshot, bounded application and verified reload, with
a persistent recovery control after unknown outcomes (ADR-050). The current
single-buffer editor supplies its complete inventory; Split is a preview.

The 0.20.7 shared publication contract adds bounded divergent branch envelopes.
Server and client replay the same history rules from `notes-sync::transfer`.
Only the enclosing resolution advances a head; branch bytes remain retrievable
inside its immutable publication. The uploader's explicit resolve command stages
chosen bytes and both observed parents without touching source files (ADR-051).

In 0.20.8 the client exposes explicit result-path and tombstone choices through
`resolve-to` and `resolve-delete` (ADR-052). They reuse the same publication
contract and do not add filesystem move/delete operations to core application.
