# Acceptance — milestone 0.2 (Index)

> **Status:** `ACTIVE` · Implemented in repository version 0.14.0. Automated
> evidence is separate from owner acceptance. As in 0.1d, the owner walks the
> installed Linux package and repeats the flows on the following release before
> any manual box is marked. The request to build 0.2 authorized implementation
> alongside the remaining 0.1d verification; it did not supply those observations.

## Automated evidence

| Contract | Evidence |
|---|---|
| Reindexing changes neither note bytes nor identity | `notes-core/tests/milestone02.rs`: BOM/CRLF, forced rebuild, deletion/recreation of index.db, restart, stable NoteId and recent history |
| Incremental refresh sees changes and removals | Same suite: unchanged files skip content work; changed content replaces FTS entries; removed paths disappear |
| Registry migration preserves operational identity | Same suite: legacy JSON migrates with byte-exact backup; `notes-index/tests/storage.rs`: separate DB lifecycle, newer-schema refusal, stale snapshot merge and conflict refusal |
| Search semantics stay distinct | Existing literal/regex core suites and `notes-index/tests/storage.rs`: whole tokens, accent/case folding, no stemming or implicit FTS operators; `SearchPanel.test.tsx` covers the three modes and cancellation during startup |
| Rename/move is reviewed and recoverable | Core integration tests cover incoming/outgoing links, selected vs unselected files, concurrent modification refusal and originals; `notes-markdown::rewrite` tests cover code, images, definitions and folder boundaries |
| Review cancellation makes no changes | `ReferenceReview.test.tsx`: Escape, selection, explicit fallback without rewriting links |
| A damaged index does not destroy notes | Core integration test rebuilds a corrupt derived DB and retains identity/content |

The full gate remains `tools/check.sh`. On macOS its ENOSPC fixture explicitly
skips: Linux CI supplies that platform-specific check. Bundled SQLite requires
a MinGW C compiler for the Windows cross-target step (`brew install mingw-w64`
on macOS). A native debug `.app` build is a development artifact, not a signed
macOS release or Linux installation test.

## Repeatable manual flows

| # | Flow and expected result | Installed release | Following release |
|---|---|---|---|
| X1 | Open a workspace; index progress finishes while editing remains available | ☐ | ☐ |
| X2 | Reopen unchanged files; the index reports unchanged files without reprocessing their content | ☐ | ☐ |
| X3 | Search the same expression in Words, Literal and Regex; each named mode keeps its documented meaning | ☐ | ☐ |
| X4 | Cancel indexing; results are explicitly partial; Rebuild resumes usable search | ☐ | ☐ |
| X5 | Change/create/delete a file externally; the next scan updates Words results | ☐ | ☐ |
| X6 | Delete only index.db with the app closed; reopen/rebuild; notes, IDs, recents and tabs survive | ☐ | ☐ |
| X7 | Files → Recent opens previously visited notes, including after a rename | ☐ | ☐ |
| X8 | Files → Outline follows live Markdown headings and jumps to the correct line | ☐ | ☐ |
| X9 | Rename a referenced note; inspect destinations, cancel, then apply; incoming links update only after approval | ☐ | ☐ |
| X10 | Move a note/folder across depth; outgoing relative links and incoming references are both reviewed | ☐ | ☐ |
| X11 | Deselect a reference; that file stays byte-identical; code, titles, BOM and line endings are preserved | ☐ | ☐ |
| X12 | Edit a reviewed file externally before Apply; operation refuses, or reports a per-file race with recoverable originals | ☐ | ☐ |
| X13 | Make the index unavailable; literal/regex/editing remain available; moving without link updates requires an explicit choice | ☐ | ☐ |

## Search and recovery contract

Words uses FTS5 `unicode61 remove_diacritics 2`, case-insensitive whole tokens
joined by AND. It does not expose the FTS query language, stemming or prefix
matching. Literal and Regex continue scanning saved files with their existing
case option. Quick Open still uses its independent path cache. All modes search
disk, not unsaved buffers. Word results are capped at 2,000 and marked partial
while stale, cancelled, rebuilding or skipping files. Notes over 8 MiB and
non-UTF-8 notes are skipped by the content index, not rewritten.

`registry.db` is operational; `index.db` is disposable. Never remove the former
to repair search. Rebuild quarantines a damaged index in app data; a database
from a newer schema is refused instead of downgraded. Legacy `registry.json`
and `registry.json.bak-1` remain available after migration. Use a closed-app
copy of the entire workspace app-data directory for operational backups;
copying a live SQLite main file without its WAL is not a reliable backup.

Rename/move is not a filesystem transaction. Before renaming, selected files
are checked against reviewed hashes and copied into
`reference-backups/<token>/<file-index>.md`, with `plan.json`. After moving,
each rewrite uses the normal guarded atomic-write protocol. `result.json`
records updated and failed paths; the UI reports failures. If interrupted,
compare the plan's source/destination paths with disk and those original bytes
before restoring anything. Never blindly overwrite a later edit. No automatic
retention cleanup removes these originals in 0.2.

Unsupported escaped/entity-encoded destinations are left alone; skipped
candidates are counted in review. Skipped notes mean the reference list may be
incomplete. A user can leave every checkbox clear or explicitly choose to move
without updating links if the index is unavailable. Wiki links, tags, backlinks,
graph and MCP remain 0.3 work.

## Performance observation

On 2026-09-09, the ignored read-only benchmark against `/Users/samir/x` scanned
6,707 notes in a debug test build: first pass 78,599 ms, 6,703 processed and four
skipped; second pass 3,681 ms, zero processed and 6,703 unchanged. App data was
in a temporary directory. This is a measured incremental improvement, not the
historical 56,622-note / 15.8-second result from a different dataset/build.
Reproduce with `NOTES_BENCH_ROOT=/path cargo test -p notes-core --test milestone02
measure_real_workspace -- --ignored --nocapture`.

## Development smoke test

The 0.14.0 debug macOS bundle displayed Outline headings for a temporary note,
opened its recent-note entry, and returned a Words hit for `telescope` at line 5.
These were UI interactions in a temporary workspace on 2026-09-09, not a Linux
installation or the owner's repeated acceptance. The final local gate passed;
frontend coverage totals 69 tests. Linux ENOSPC remains delegated to CI.
