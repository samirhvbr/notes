# Synchronization domain and pairing preview

> **Status:** ACTIVE · Causal domain in 0.19.0; server inbox in 0.19.1.
> **Milestone 0.6 remains open.** This release does not synchronize remote notes.

The `notes-sync` crate defines causal revision histories and produces plans.
`notes-core` supplies bounded inventories of real folders, and the standalone
`notes-sync-plan` command previews initial pairing. Planning never copies,
overwrites or deletes source notes. The server revision inbox below transfers immutable bytes; durable device
outboxes, source application, background scheduling and app controls remain in the queue.

## Run the preview

Build with `cargo build --locked -p notes-core --bin notes-sync-plan`, or use the
standalone Linux archive from the release. The CLI accepts two distinct,
non-nested folders already mounted on the machine:

```sh
notes-sync-plan reconcile /notes/laptop /notes/server-copy /private/sync-preview
```

The last path is an absolute operational directory outside both folders. It
holds the core's identities so repeated previews retain note IDs. It contains no
copy of source content. A rejected state location creates nothing inside the
source folders. The command uses the core's path jail and ignore rules; it does
not follow workspace symlinks or inspect hidden internals. Output is JSON
containing relative paths, identities and content hashes, never note text.

The modes are deliberately distinct:

| Mode | Precondition | Preview |
|---|---|---|
| `upload` | The remote inventory is empty | Upload local notes |
| `download` | The local inventory is empty | Download remote notes |
| `reconcile` | Either or both may contain notes | Link equal same-path content, identify conflicts and list unique notes |

The commands above describe preview actions, not network requests. “Remote”
here names the second mounted folder. No URL, bearer token or listening port is
used by this first block. A populated target is never treated as a download or
upload destination to be replaced wholesale. Matching content at unrelated
paths does not silently merge note identities. Initial identity links are
explicit output for the eventual pairing confirmation, not applied changes.

Inventories are limited to 10,000 Markdown files and 8 MiB per file. Hashes
cover original bytes, including BOM and line endings; mixed/non-UTF-8 data is
not decoded and re-serialized. Normal lazy explorer listing is unchanged. This
explicit inventory can assign identities to previously unopened notes in
operational state and reconcile unambiguous external renames; it does not add
note visits or save notes. The preview is not a transaction across two live
folders: any later application must revalidate the observed revisions and
filesystem capabilities.

## Causal model

Each immutable revision has a UUID, note identity, device UUID, zero to two
parents, a relative path and either a content hash or a tombstone. A note has
one genesis revision containing real content. Empty bytes have their actual
hash; they are not a deletion. A rename changes the path while keeping note
identity and ancestry. `modified_at` is absent from this model.

A journal has an explicitly paired workspace UUID and separate current heads.
Imported history does not choose a head. Validation rejects cycles, missing or
foreign-note parents, duplicate conflicting revision UUIDs, unrelated roots
claiming the same note identity and incompatible schemas. A head changes only
through compare-and-set of the observed head. Exact path collisions refuse the
change. Filesystem case/normalization constraints must additionally be checked
by the future application step using the target adapter.

The incremental planner compares ancestry:

- Equal heads need no work.
- A descendant can be pushed or pulled against the expected previous head.
- Divergent edits, rename/edit and delete/edit combinations remain conflicts.
- Equal values on divergent branches require a merge revision retaining both
  parents; equal bytes do not erase causality.
- Different note identities competing for the same path produce a collision,
  not an overwriting push/pull.
- A missing head is unknown, not a deletion. Only a tombstone requests deletion.

Explicit resolution creates a new revision with both observed parents and the
chosen path/content. Applying it still compares each peer's expected head; a
stale resolution cannot replace newer edits. Device acknowledgments name exact
revisions and cannot move backward along ancestry. Acknowledgments do not prune
history or tombstones in this block.

## Persistent metadata

`notes_sync::store::Store` holds schema-1 `journal.json` under an operator-chosen
private directory. Readers/writers use an OS lock, and writes compare the digest
returned by the read, validate the new graph, sync a private temporary file and
atomically replace the journal. A stale transaction, failed callback, invalid
graph or future/corrupt schema leaves the prior bytes intact. On Unix the
operational directory is mode 0700 and new files are mode 0600.

The journal is bounded to 100,000 revisions, 1,024 acknowledging devices and
64 MiB of serialized state. Reaching a bound is an explicit refusal, not silent
history pruning. This is metadata storage only; immutable content storage and
its retention policy are not implemented yet. There is no schema migration in
the first version. A future migration must preserve a pre-migration copy and
provide recovery before replacing state; unknown schemas are never reset.

## Validation and remaining work

Tests cover one-sided changes, divergent equal content, stale compare-and-set,
explicit two-parent resolution, delete/edit and rename/edit conflicts, path
collisions, invalid DAGs, monotonic receipts and state persistence across reopen.
Core/process tests inspect real BOM/CRLF/binary notes and verify byte preservation,
stable identity, external rename correlation and rejected populated pairing
modes. These tests exercise actual CLI execution, not only JSON fixtures.

Remaining work and owner acceptance are in
[the 0.6 queue](../.continue/0.6-sync.md). Milestone 0.7 remote MCP follows the
completed sync stage; the shipped local MCP and 0.5 REST server are unchanged.

## Server revision inbox (0.19.1)

The server now accepts and returns immutable revisions over its existing
HTTPS/authentication boundary. This is an inbox for replication, **not live
workspace synchronization**: neither publication nor download changes a file
under `workspaces/`. The device outbox, source application, conflict workflow,
attachments and background/UI integration remain open.

The authenticated OpenAPI contract describes three operations:

- `GET /v1/workspaces/{workspace}/sync/revisions?cursor=0&limit=100` returns
  the inbox workspace UUID, a page of revision metadata, current heads for the
  notes on that page, the next append-log position and `has_more`.
- `GET /v1/workspaces/{workspace}/sync/revisions/{revision}` returns the
  immutable publication with canonical base64 of the original bytes.
- `POST /v1/workspaces/{workspace}/sync/revisions` atomically accepts a
  publication containing `workspace`, `expected`, `revision` and
  `content_base64`. The response says `stored: true, applied: false`.

`expected` is the observed inbox head UUID, or null for a new note. All parents
must already be stored. A stale head, reused UUID with different facts, foreign
workspace UUID or exact path collision returns 409. The device must retain its
unaccepted revision locally; this endpoint does not yet import divergent
branches. Retrying the identical accepted publication succeeds even after the
head advances, without moving the head backward. No timestamp chooses a winner.

Read permission is mandatory. Genesis and resurrection also require Create;
live successors require Update, path changes require Move and tombstones require
Delete. Review-mode writes check both old and new paths under `scope/proposals`.
Every historical path for a note must be visible to the credential: a move out
of a subfolder hides the whole history from that subfolder's token. Hidden paths
are refused. Workspace names come from the credential, never from a supplied
filesystem path. UUIDs do not grant access. Device UUIDs are causal claims;
the authenticated credential remains the audit author.

Cursors count scanned append-log entries, including entries filtered by scope,
so they can reveal aggregate activity within the authorized workspace. Empty
pages can advance and must not terminate traversal while `has_more` is true.
Heads are current, not a snapshot across pages. Save `next_cursor` only after
processing the page and fetching required bytes. A restored older backup may
require restarting at cursor zero; already accepted revision UUIDs make retries
safe. A storage receipt is not a device application acknowledgment. The inbox
UUID is created on first authorized inventory access and retained across restart
and backup/restore.

### Storage, limits and recovery

`sync/<workspace>/vault.json` is private server data containing both the causal
journal and append-ordered publications. This bounded first implementation
stores copies of revision content as base64 in the same atomic document rather
than publishing a head before a separate blob exists. Live Markdown remains the
source of truth. Loading verifies schema, parent order, head transitions and
content hashes. Publication holds a per-workspace OS lock, writes and syncs a
private temporary file, atomically replaces the document, and syncs the directory
on Unix. A lost response is recovered by retrying the same publication. An
abandoned temporary file is never loaded as state; future/corrupt committed
state is refused without replacement. Operators can remove abandoned temporary
files while the server is stopped, after backing up the data.

Limits are 8 MiB per revision content, 10,000 revisions, 32 MiB cumulative
decoded content and 64 MiB serialized state per workspace. Identical bytes in
different revisions count separately. Capacity refusal is HTTP 507 and leaves
all accepted content intact. History and tombstones are retained for the life
of the inbox; there is no automatic garbage collection or credential-driven
purge. This conservative retention is for a bounded first transport block,
not unlimited production history. Offline full-data backup includes the vault
and excludes its process lock. Restore preserves UUIDs, cursors and original
bytes. Retention migration and device-confirmed pruning remain future work.

Tests exercise HTTP publication/fetch, exact-byte preservation, retries after
head advancement, concurrent writers, stale writes, path collisions, quota
refusal, scope and review permissions, revocation, future/corrupt state refusal,
restart and offline backup/restore. No test claims remote source application or
mobile synchronization.
