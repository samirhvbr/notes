# Local knowledge and agents

> **Status:** `ACTIVE` · Implemented in repository version 0.16.0.
> Installed-release acceptance is tracked separately in [ACCEPTANCE-0.3.md](ACCEPTANCE-0.3.md).

## Knowledge navigation

`notes-markdown` interprets YAML front matter without rewriting it. Properties
shows scalar and structured values as text; an invalid mapping reports a
warning and leaves the entire source untouched. Interpretation is limited to
64 KiB of front matter. Indented separators inside YAML block scalars remain
content (corrected in 0.16.1). When upgrading from 0.16.0, use Rebuild index
to refresh cached tags in affected unchanged notes; properties read the live
buffer and reflect the correction immediately. `title` is exposed as metadata, never used to rename a
file. Tags combine YAML `tags` (a string or string sequence) and inline `#tag`.
Tags are lowercase, deduplicated, allow Unicode letters/numbers and `_-/`, and
must contain a letter. Code, escaped hashes, link/image destinations and raw
HTML are excluded. Metadata follows the current buffer; Tags, Backlinks and
Graph describe saved, indexed files.

Wiki links use `[[Name]]`, `[[Name#heading|label]]` or a workspace-root path
such as `[[folder/Name.md]]`. Matching uses NFC and lowercase; both `.md` and
`.markdown` work. `[[./Name.md]]` explicitly selects a file at the root.
Unqualified names search every indexed directory and the live file tree when
clicked. A homonym opens a chooser; no arbitrary first match is selected. A
missing target reports that it is missing and creates nothing. A heading
fragment jumps to the parser's heading slug in the destination editor.

Backlinks and graph edges use resolved relative Markdown and wiki links.
Ambiguous or missing wiki destinations have no fabricated edge. The graph
reports unresolved targets, offers path filtering and an accessible note list,
and opens a note by click or keyboard. Responses are bounded to 1,000 notes,
5,000 edges and 1,000 unresolved links; the graph draws at most 80 filtered
nodes. Limits, stale/cancelled/running indexes and skipped files are labeled
partial. Rebuild refreshes these derived facts; index schema 2 rebuilds schema
1 rather than trusting pre-wiki/tag documents. Registry schema is unchanged.

Reviewed rename/move extends the 0.2 flow to unambiguous wiki destinations.
The preview preserves aliases/fragments and emits an explicit root-qualified
path. Code and unsupported/ambiguous spans are skipped, never guessed. Existing
[reference backups and partial-failure recovery](ACCEPTANCE-0.2.md#search-and-recovery-contract)
still apply. MCP move does not rewrite other files; its result says so.

## Clipboard images

Pasting a PNG/JPEG image into an editable note imports its original bytes into
`attachments/image-<uuid>.png` or `.jpg` at the workspace root, using exclusive
creation. The editor inserts a relative Markdown image link. Core validates
actual image bytes (not a clipboard MIME label), at most 8 MiB, 8,192 pixels per
dimension and 64 MiB decoder allocation. SVG and other formats are refused.
If the buffer, selection, note or workspace changes during import, the image
stays on disk and a notification gives its path; no link is inserted into a
different buffer. Unreferenced attachments are never automatically deleted.

## Standalone MCP

The client starts `notes-mcp --config /absolute/path/agent.json`. Build with
`cargo build --release -p notes-mcp --locked`; no Tauri app or JavaScript runtime
is needed. Linux releases include a separate `notes-mcp-<version>-x86_64-linux.tar.gz`
and SHA-256 file. Extract the archive, verify its checksum and configure the
client to execute the absolute binary path. The app packages do not silently
register a client or enable permissions.

Create a configuration outside the workspace's note content:

```json
{
  "workspace": "/absolute/path/to/notes",
  "scope": "Projects",
  "permissions": ["read", "search", "create", "update", "move"],
  "review": true
}
```

The scope must already be a directory; `""` selects the root. In review mode,
create `Projects/proposals/` yourself: all writes, including both endpoints of
a move, are restricted there. Reads retain the selected scope. Apply a proposal
later with an explicit human edit. Permissions default to an empty list;
`delete` is a separate opt-in. Config changes take effect after restarting the
server. Configuration is capped at 64 KiB; the absolute workspace is
canonicalized at startup. Hidden directories and symlinks are refused.

An MCP client can register this process with its usual stdio configuration:

```json
{
  "mcpServers": {
    "notes": {
      "command": "/absolute/path/to/notes-mcp",
      "args": ["--config", "/absolute/path/to/agent.json"]
    }
  }
}
```

Use the same OS account and app-data directory as the app. A deliberate
`NOTES_DATA_DIR` override must be identical in both processes; different data
directories cannot share operational IDs, receipts or locks. The integration
never installs or changes this client configuration for you.

| Tool | Permission | Required arguments |
|---|---|---|
| `notes_list` | read | optional limit |
| `notes_search` | search | query, optional limit |
| `notes_read` | read | path |
| `notes_create` | create | path, text |
| `notes_update` | update | path, text, base_rev |
| `notes_append` | update | path, text, base_rev |
| `notes_move` | move | path, to, base_rev |
| `notes_delete` | delete | path, base_rev |

Paths are relative to the workspace, including the scope prefix. Unknown or
extra arguments are refused. Search is literal and case-sensitive over saved
UTF-8 content, never unsaved buffers; it returns at most 200 hits with bounded
snippets. List is also capped at 200. A query has 1–4,096 bytes and note input
and resulting text are limited to 8 MiB. Enumeration is confined to the scope.
Large trees may take time: operations are serial and there is no progress or
cancellation capability in this version.

Read returns `note_id`, `path`, `text`, `base_rev` and `read_only`, not drafts or
conflict snapshots. Copy `base_rev` unchanged into an update/append/move/delete.
Create has no existing revision and exclusively creates a new file; it refuses
collisions. All other writes compare the current content hash under the shared
workspace lock, even when size and mtime match. App and MCP enrollment also
share a global lock so simultaneous first starts use the same workspace ID.
Read/identity assignment is serialized; agent visits do not change the GUI's
last workspace or recents. Third-party editors do not take these locks; as in
the existing save protocol, a race after the final check is not universal
filesystem exclusion.

Append receipts in `agent-appends/` under workspace app data bind `(NoteId,
BaseRev)` to the exact append text. Retrying after client restart returns the
original success without appending again. Reusing the base with different text
is refused. A prepared receipt recognizes the intended output after an
interruption between replacement and receipt completion. A divergent file is
refused. Receipts have no automatic retention cleanup: deleting them discards
retry history. Include them in closed-app operational backups alongside
`registry.db`, drafts and conflicts. An operational persistence error after a
file write requires reading the current file before retrying an update; never
blindly replace a later edit.

Transport follows MCP's [stdio framing](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports),
[initialization lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)
and [tools contract](https://modelcontextprotocol.io/specification/2025-11-25/server/tools):
newline-delimited JSON-RPC, initialize then initialized, tools/list and
tools/call, tool failures with `isError`. It supports protocol versions
2025-11-25, 2025-06-18 and 2025-03-26. Messages are capped at 32 MiB. Stdout is
protocol-only and no TCP listener is opened. Note text is data and cannot alter
permissions or execute commands. These scopes govern this integration, not a
client's independent access to the user's machine.
