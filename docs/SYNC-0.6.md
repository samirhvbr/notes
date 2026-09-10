# Synchronization domain and pairing preview

> **Status:** ACTIVE · Domain in 0.19.0, server inbox in 0.19.1, device client in 0.20.0.
> **Milestone 0.6 remains open.** Guarded CLI application was added in 0.20.1.

The `notes-sync` crate defines causal revision histories and produces plans.
`notes-core` supplies bounded inventories of real folders, and the standalone
`notes-sync-plan` command previews initial pairing. Planning never copies,
overwrites or deletes source notes. The server inbox and device client below transfer immutable bytes with durable
queues. Explicit closed-workspace application is described below; background scheduling
and app controls remain open.

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
by application using the target adapter.

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
under `workspaces/`. The client below supplies the outbox and explicit guarded application. Conflict workflow,
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

## Device transfer client (0.20.0)

`notes-sync-client` is an explicit command-line client with a persistent offline
outbox and received-content cache. **Transfer does not apply revisions to source
files.** Explicit application is a separate command added in 0.20.1. This block supports
whole-workspace, non-review credentials only; subfolder pairing and reconciliation
with an already populated remote are future client work. The server retains
its existing subfolder and review policies for other callers.

Build with `cargo build --locked -p notes-sync-client`, or use the standalone
Linux release archive. Pair an existing source folder with an empty server inbox:

```sh
notes-sync-client init-upload /private/sync-state /home/me/notes https://notes.example home /private/integration.secret
notes-sync-client stage /private/sync-state
notes-sync-client transfer /private/sync-state /private/integration.secret
notes-sync-client status /private/sync-state
```

The explicit `init-upload` command confirms the chosen source, endpoint and
workspace. It refuses a populated server inbox or an existing client state.
There is no reset-by-reinitialization. The state directory must be absolute and
outside the source folder. Pairing fixes the remote inbox UUID, source root and
endpoint in schema-1 `client.json`. The token is read only when connecting, from
an absolute regular file (private permissions on Unix); it is never persisted
in client state, placed in command arguments or printed.

`stage` works offline. Core inventories retain note identities, capture original
bytes and check them against observed hashes. Changed/new notes append immutable
publications; unambiguous external renames retain identity. Pending predecessors
remain ordered even when several edits are staged before connecting. Source
bytes that change during capture cause refusal, preserving the prior queue.
Only saved bytes are captured, never an unsaved editor buffer. Missing tracked
notes are reported and **do not infer deletion**. Automatic tombstone capture,
rename cycles and explicit deletion/application flows remain queued.

`transfer` executes one bounded batch: at most 20 publications and one page of
20 incoming revisions. Run it again to continue. Each valid storage receipt is
checkpointed separately; a lost response leaves the exact publication UUID and
bytes queued for an idempotent retry. A stale head returns a conflict and keeps
that publication and its successors. No retry loop elects a winner. Offline,
revoked credential, busy/rate-limited and capacity errors stop the batch with
accepted progress already durable. Status can be inspected without connecting.

To receive into a second device's private cache:

```sh
notes-sync-client init-receive /private/receiver-state /home/me/notes https://notes.example home /private/integration.secret
notes-sync-client transfer /private/receiver-state /private/integration.secret
notes-sync-client received /private/receiver-state
notes-sync-client export /private/receiver-state REVISION_UUID
```

`init-receive` does not promise a download into the selected folder; it binds a
future source location while received revisions remain in private state. The
receiver validates the workspace UUID, append cursor, immutable metadata,
parentage and original-byte hashes. It saves the whole page and its content
before advancing the cursor. An interrupted or invalid fetch leaves that page
unconsumed. `received` lists metadata; `export` creates a new
`received-<revision>.md` file in the state directory, never overwriting an
existing file. This makes received bytes inspectable without claiming source
application. Before explicit application, status includes `applied: false`.

### Client transport and storage limits

The operator supplies one origin, with no credentials, path, query or fragment.
The client disables redirects, automatic retries and environment/system proxies.
It resolves and validates addresses, then pins those addresses for the process
while preserving TLS hostname verification. HTTPS is required. `--allow-private`
at initialization explicitly permits a private/loopback server and also permits
plain HTTP at a literal loopback IP for local operation. Link-local, unspecified,
multicast and cloud metadata addresses remain refused. The selected exception
is persisted with the endpoint. `NOTES_SYNC_CA_FILE` can add an explicitly
selected PEM trust anchor without disabling certificate verification.

The client uses reqwest 0.13.4 with its blocking, JSON and provider-free rustls
features, selecting ring explicitly. No HTTP dependency enters the domain or
core crates. See the [reqwest transport documentation](https://docs.rs/reqwest/0.13.4/reqwest/)
for the underlying redirect, proxy and TLS defaults overridden here. Connect
and request timeouts are 10 and 30 seconds; DNS resolution also depends on the
operating system resolver. Responses are bounded to 16 MiB. The credential must
still identify the original whole workspace on every connection.

Client state has a 64 MiB serialized limit, 32 MiB cumulative decoded pending
and received content, and 10,000 pending/received publications. Source capture
is limited to 32 MiB total and 8 MiB per file. Private temporary files, OS locks,
file sync and atomic replacement protect checkpoints. Future/corrupt state is
refused unchanged. A state directory backup while the client is stopped includes
queued bytes and core identities; do not discard it to resolve a conflict.
There is no pruning or state migration yet. Received history is not a backup
policy. Transfer sends no device application acknowledgment; the explicit
`acknowledge` command below does.

Validation includes fault-injected lost receipts, restart, repeated offline
edits, rejected incoming bytes, stale/server-rebound conflicts, unsafe state
paths, future schema and over-limit capture. The server smoke runs two actual
client processes over TCP, disconnects/restarts the native server, and repeats
the exchange through the CI HTTPS proxy. The HTTPS test first refuses its
untrusted certificate, then uses the test CA explicitly. Original BOM/CRLF and
non-UTF-8 bytes are compared after receipt and export; source folders remain
unchanged.


## Guarded source application (0.20.1)

After receiving revisions, close this workspace in notes and every local MCP or
server process using it. Use the updated 0.20.1 core in those processes. Then run:

```sh
notes-sync-client apply /private/receiver-state /home/me/.local/share/notes
notes-sync-client status /private/receiver-state
```

The second argument is the **actual application data directory**, not the
client's private `core/` directory. Use the configured `NOTES_DATA_DIR` when set;
otherwise core uses the platform data directory plus `notes` (on macOS,
`~/Library/Application Support/notes`; on Windows, `%APPDATA%/notes`). It must be
outside the source workspace. The first application pins its canonical location;
a later invocation with a different directory is refused. Keep the client state
and application data when backing up or recovering a device.

Only receive-mode clients can apply, at most 20 revisions per invocation, in
received order. New Markdown files and same-path updates are supported. The
core holds an exclusive activity lease and its existing write lock, refuses any
pending draft entry, validates names/path jail/collisions, and checks the full
observed BaseRev and local identity before updating. Existing files are not
adopted merely because their bytes match. Parent directories can be created;
original bytes are never decoded or normalized. New files are published from a
synced temporary file without replacement (Unix permissions 0600). An
interruption before publication leaves no partial destination; an abrupt process
exit may leave a hidden `.notes-create-*.tmp` file, which is not a source note.

Every open core workspace holds a shared OS lease in application state. The
lease uses the native root identity when available, falling back to its canonical
path. It coordinates cooperating updated processes sharing the same data
location. Older binaries, separate application data directories, and third-party
editors do not participate. Use the same workspace path as the app. External
saved edits are checked using BaseRev/hash; this is not a transaction against an
uncooperative external writer changing the filesystem at the publication instant.

`application.json` is a separate bounded schema-1 checkpoint. Preflight checks
must succeed before a durable revision intent is stored; source writes happen
only afterward. If the write succeeded but its receipt was lost, retry accepts
the exact intended bytes without rewriting them. Differing newer content blocks
retry. A successful local receipt records remote revision, local identity and
observed BaseRev; it is persisted before advancing the application cursor. If
later work in a batch fails, earlier receipts remain committed. Inspect `status`
for progress and preserve the received queue on any error.

`applied_revisions` counts successful historical local receipts. `applied` is
true only when that count is nonzero and equals the received count. It is **not**
a live disk scan or an assertion that later local edits match the remote. These
receipts are distinct from server `stored: true, applied: false` responses; the explicit command below reports them to the server.

Renames, tombstones, divergence resolution, dirty-buffer integration and active
editor application are still refused/queued. Do not delete drafts or local
notes merely to bypass a refusal. Core tests cover byte preservation, failed
intent persistence, interrupted receipt recovery, drafts and a real second
process holding the workspace open. Client tests cover checkpoint progress,
collisions, local edits, incompatible state and rename refusal; the native TCP
and HTTPS smoke tests exercise explicit application followed by a local conflict.

## Device application acknowledgments (0.20.3)

After applying received notes, explicitly report the durable receipts:

```sh
notes-sync-client acknowledge /private/receiver-state /private/integration.secret
notes-sync-client status /private/receiver-state
```

The command requires a receive-mode client and sends at most 20 receipts in
application order. Each successful echoed response advances a durable
`acknowledged` cursor in `application.json`. `acknowledged_revisions` in status
counts confirmed historical receipts; it is not a live disk assertion. Cached
content, exports and an unfinished write intent never qualify. Application and
transfer remain offline-capable/separate operations; neither sends receipts.

The authenticated `POST /v1/workspaces/{workspace}/sync/acknowledgments` endpoint
accepts the pinned workspace UUID, device UUID and one applied revision UUID.
It requires Read and whole-history path visibility. The first accepted receipt
binds a device to that credential ID; another credential cannot claim that
device. Token replacement with a new credential ID requires future explicit
rebinding support; preserve state instead of changing the device UUID by hand.
There is no remote proof of disk contents: receipts are authenticated client
assertions and do not authorize deletion, pruning or automatic recovery.

Exact retries are idempotent. A lost response or local checkpoint failure leaves
the same receipt pending; retry sends it again without touching source notes.
An older ancestor after a newer acknowledgment is refused, preserving the
server's progress. Restoring an older client/server backup can therefore require
the still-pending recovery/reconciliation flow. No automatic rollback or reset
is attempted. Revocation, workspace mismatch, hidden/out-of-scope history and
unknown revisions are refused. Existing server request/body limits apply; the
journal allows at most 1,024 acknowledging devices and evicts none.

Server receipts and device owners are atomic with the existing vault and survive
its offline backup/restore. Existing vaults default to no owners/receipts;
existing application checkpoints default to zero acknowledgments. Once written,
the new fields are intentionally refused by older binaries rather than silently
lost. Keep the updated binaries and both state directories during recovery.
ADR-048 records the boundary. Tests cover lost responses, restart, batch bounds,
legacy checkpoints, unapplied/local-conflicting content, ownership, monotonicity,
scope, revocation, backup/restore and the real CLI over the TCP/HTTPS smoke path.


## Exclusive open-session core foundation (0.20.5)

This is a Rust host API, **not an app command or a change to the CLI**. The
ordinary app still opens shared sessions, and the CLI still requires the
workspace closed. The editor input barrier, complete pane snapshot collection,
received-queue adapter and UI controls remain unimplemented.

A host can call `WorkspaceService::open_sync_workspace` before opening any
buffers. This acquires the same exclusive activity lease used by offline apply
and holds it until closing the workspace. A second cooperating process cannot
open that root using the same app data. An already-open session cannot upgrade:
it must first be closed through the normal draft-preserving workflow. Failed
exclusive admission leaves no open workspace and does not change later shared
admission. Existing ordinary app/MCP behavior stays shared.

`sync::apply_in_workspace` accepts the existing service, the received path and
bytes, the previous local receipt, retry intent, and a snapshot of **every live
buffer**, including inactive panes. Each snapshot contains note identity,
observed BaseRev, buffer version and saved version. The host must stop editing,
settlement, navigation and workspace switching for the complete operation and
reload. Core cannot discover omitted frontend buffers.

Before calling the durable-intent callback, core rejects changed buffers,
suspended notes, duplicate snapshots, stale disk revisions and any draft entry.
It then uses the same collision, byte-preservation, atomic-write and registry
protocol as offline apply. Dirty buffers are never flushed or discarded to make
sync proceed. Successful application invalidates the quick-open path index;
`reload_note` gives the host the clean document, current revision and encoding
profile without closing its workspace. Refresh affected clean buffers before
resuming editing, including recovery after an uncertain source write. Persist
the returned receipt separately; a successful source write is not a server ack.

The new tests cover clean application/reload, dirty inactive and stale clean
buffers, unchanged source on failed intent, draft preservation, refusal of shared
sessions and competing owners, and lease release at close. The existing
cross-process and offline recovery tests run against the shared implementation.
No frontend interaction or installed-app acceptance is claimed by these tests.
