# Synchronization domain and pairing preview

> **Status:** ACTIVE · First 0.6 block implemented in 0.19.0.
> **Milestone 0.6 remains open.** This release does not synchronize remote notes.

The `notes-sync` crate defines causal revision histories and produces plans.
`notes-core` supplies bounded inventories of real folders, and the standalone
`notes-sync-plan` command previews initial pairing. Planning never copies,
overwrites or deletes source notes. REST replication, content transfer, durable
outboxes, background scheduling and app controls remain in the queue.

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
