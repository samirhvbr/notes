# Acceptance — milestone 0.3

> **Status:** `ACTIVE` · Implementation in 0.16.0, YAML correction in 0.16.1; owner acceptance pending.
> As required for 0.1d and every later milestone, Samir walks the installed
> Linux release and repeats the flows on the following release. Automated or
> local debug checks never tick the owner columns.

The implemented contract and limits are in [KNOWLEDGE-0.3.md](KNOWLEDGE-0.3.md).
The mobile foundation from [PR #2](https://github.com/samirhvbr/notes/pull/2)
was reviewed and integrated in 0.17.0; full mobile acceptance remains in
[ACCEPTANCE-0.4.md](ACCEPTANCE-0.4.md).

## Automated evidence

| Criterion | Evidence |
|---|---|
| YAML/tags are read-only; code and destinations do not become tags | `notes-markdown::knowledge` tests (including indented block-scalar separators) and the reviewed front-matter golden |
| Backlinks and graph do not invent a homonym target | `notes-core/tests/knowledge.rs` |
| Wiki rename preserves aliases, fragments and code | Reviewed rename integration test |
| Clipboard import preserves image and note bytes, uses unique names and rejects invalid content | Core image import integration test |
| Ambiguous wiki navigation requires selection; graph supports keyboard; tags filter semantic data | `Knowledge.test.tsx` |
| Headless operation, scope confinement, separate delete permission and review writes | Real child-process `notes-mcp/tests/stdio.rs` tests |
| App and MCP refuse stale writes, including equal size and restored mtime | Parent app-core service and child MCP process regression tests |
| Concurrent app/MCP writers have exactly one winner and preserve that content | Eight racing writes through parent app core and child MCP process |
| An append retry across process restart does not duplicate text | MCP process integration test |
| Two first clients share identity without changing GUI last workspace | Two simultaneous MCP child processes and a subsequent app-core open |
| Parser upgrade only clears disposable data | Index schema 1→2 migration regression |

Run the complete `tools/check.sh` gate and the native/Windows/Linux/Arch CI
matrix. The MCP transport tests are real protocol clients, not a claim of a
configured third-party AI client's behavior. macOS debug UI validation is not
an installed Linux acceptance run.

## Owner flows

| # | Flow and expected result | Installed release | Following release |
|---|---|---|---|
| K1 | Open YAML with scalar/structured properties, malformed YAML and CRLF; properties appear or warn; untouched bytes stay identical | ☐ | ☐ |
| K2 | Mix YAML tags, inline tags, code, links and escaped hashes; Tags shows only semantic tags and filters notes | ☐ | ☐ |
| K3 | Follow unique, ambiguous and missing wiki targets, including Unicode names and a heading fragment; no arbitrary target opens | ☐ | ☐ |
| K4 | View backlinks; follow one and inspect the same edge in Graph; keyboard opens the selected node | ☐ | ☐ |
| K5 | Filter a large graph; truncation/stale state is explicit, and editing stays available | ☐ | ☐ |
| K6 | Rename/move a linked note; review wiki edits, cancel and apply; aliases/code survive and backups exist | ☐ | ☐ |
| K7 | Paste PNG/JPEG into nested notes twice; unique root attachments render via relative links; invalid formats fail visibly | ☐ | ☐ |
| K8 | Change note/workspace while an image imports; the destination path is reported without inserting into another buffer | ☐ | ☐ |
| K9 | Close the app; launch the standalone MCP in a client; list/read/search work within the configured scope | ☐ | ☐ |
| K10 | Exercise create/update/append/move; stale base refuses; retry append after restart does not duplicate text | ☐ | ☐ |
| K11 | Keep an unsaved app buffer while MCP changes the file; app shows conflict and preserves both versions | ☐ | ☐ |
| K12 | Review mode restricts writes to proposals; delete is unavailable unless explicitly granted; out-of-scope search leaks no snippet | ☐ | ☐ |
| K13 | Rebuild index; tags, links and graph return while note bytes, IDs, drafts and append retry history survive | ☐ | ☐ |

Record release numbers, platform, failures and repeat results here when Samir
performs the walk. No owner flow has been marked by an agent.

## Local observation — 2026-09-10

A separate macOS debug application with isolated app data opened a four-note
fixture. Native UI interaction confirmed YAML properties and semantic tags,
preview wiki ambiguity with both candidates and Escape cancellation, a graph
with the expected edge/unresolved count, opening the linked note from its
accessible list and directly from its SVG node, and its backlink to the
originating note. The final graph exposes each node as an accessibility button. This was a local
debug check, not owner acceptance or a clipboard/third-party client walkthrough.
