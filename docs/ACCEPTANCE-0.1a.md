# Acceptance — milestone 0.1a

> **Status:** `ACTIVE` · Every acceptance criterion of milestone 0.1a
> (`.continue/SCOPE_final.md` §17), against a named automated test or a
> documented manual step. **A criterion with neither is listed as not met**, and
> one is.
>
> Measurements were taken on the development machine: Debian 13 (trixie),
> Linux 6.12, ext4 on NVMe, X11, no NVIDIA. Rust 1.96, Node 24, Tauri 2.11.5.

## Summary

| # | Criterion | Verdict |
|---|---|---|
| 1 | Tree lists in <1 s without reading content | **met** — measured |
| 2 | 1 000 kills mid-save, never truncated or empty | **met** — 1000/1000 |
| 3 | Open and save unchanged → `git status` clean | **met** — 228 files |
| 4 | Dirty buffer + external append → suspend, draft, no overwrite | **met** — automated |
| 5 | Disk full / permission denied → visible error, recoverable buffer | **partly met** — see §5 |
| 6 | No command accepts a path outside the root | **met** — automated |
| 7 | Opening a folder creates no file in it | **met** — automated |
| 8 | `cargo test` passes with no Tauri | **met** — 123 tests |

**The application window has never been launched.** Every result below comes
from the core and the corpus; the interface compiles and typechecks and has not
been driven by a human. That is stated again under *Not verified*.

---

## 1. Tree lists in under a second, without reading content

**Automated** — `notes-core`, `tests/performance.rs::a_ten_thousand_note_workspace_opens_and_lists_in_under_a_second`,
`#[ignore]`d because it needs the generated corpus:

```bash
tools/gen-large.sh
cargo test -p notes-core --test performance -- --ignored --nocapture
```

Measured against `fixtures/large` — 10 000 notes, 197.2 MiB, 10 110 entries:

```
open_workspace:       225.8 µs
list root:             45.5 µs
list whole tree:       37.5 ms
open + whole tree:     37.7 ms
```

The "whole tree" figure walks every directory at all three levels, which is what
the sidebar does as the user expands it. Two orders of magnitude under the
criterion.

**Also asserted structurally**, because a timing test on shared CI hardware is a
flake generator while the property is exact: `notes-fs`,
`tests/fixtures.rs::listing_is_one_level_and_reads_no_content` — listing returns
one level, marks notes from their extension, and opens no file. The performance
test additionally asserts the registry is still empty afterwards: if listing had
assigned identities it would have hashed 197 MiB (see
[DECISIONS-0.1a.md](DECISIONS-0.1a.md) D-09).

## 2. A thousand kills during saves

**Automated** — `tools/crash-save-loop.sh`, and in CI on every push to `master`.

```
1000/1000 rounds, 0 failures
crash-save-loop: 1000 rounds, no truncated or empty note; temp files never exceeded one
```

Each payload is self-describing (`LEN=<n>`, body, `END`), so the checker can tell
a complete note from a truncated one without knowing where the kill landed. It
also asserts the temporary-file bound, and **that assertion found the defect
recorded as D-12**: `SIGKILL` inevitably leaves the temporary file behind, and
with a random suffix they accumulated in the user's folder. The name is now
deterministic — one per note, overwritten by the next save.

## 3. Open and save unchanged, `git status` clean

**Automated, in the criterion's literal form** — `tools/byte-preservation.sh`,
run in CI on all three operating systems:

```
== fixtures/basic
saved unchanged: 206 · read-only: 1 · skipped: 0
== fixtures/edge-cases
saved unchanged: 22 · read-only: 4 · skipped: 0

PASS: every file opened and saved unchanged; git status is clean
```

**Also hermetic**, on a copy, so a failure cannot dirty the repository:
`notes-fs`, `tests/fixtures.rs::basic_corpus_survives_open_and_save_unchanged`
and `::edge_cases_corpus_survives_open_and_save_unchanged`.

The parenthesised cases are each their own assertion:

| Case | Test |
|---|---|
| CRLF | `notes-model`, `text::tests::crlf_is_hidden_from_the_editor_and_restored_on_save` |
| BOM | `notes-model`, `text::tests::bom_is_hidden_from_the_editor_and_restored_on_save` |
| No final newline | `notes-model`, `text::tests::round_trips_every_line_ending_shape` |
| NFD | `notes-model`, `path::tests::compare_key_folds_nfd_onto_nfc` — the path is **never** normalised; only the comparison key is |
| Mixed EOL opens read-only | `notes-fs`, `tests::the_edge_cases_that_must_open_read_only_do`, and `notes-core`, `protocol::a_read_only_note_refuses_to_be_saved` |

`.gitattributes` marks the corpus `-text` — without it git would normalise the
very line endings under test and the criterion would measure git
([DECISIONS-0.1a.md](DECISIONS-0.1a.md) D-02).

## 4. Dirty buffer plus an external append

**Automated in the core**, as the criterion requires — `notes-core`,
`tests/protocol.rs::an_external_append_suspends_autosave_and_writes_a_draft_without_overwriting`.
It asserts all four halves: the save returns `Conflict`, the external bytes are
still on disk untouched, autosave is suspended for that note, and the draft in
app data holds exactly what was typed.

`::a_suspended_note_keeps_its_edits_going_to_the_draft` covers the rule that
follows it — while suspended, the debounce writes to the draft and never to the
note.

## 5. Disk full / permission denied — **partly met**

**Permission denied: automated** (Unix) — `notes-core`,
`tests/protocol.rs::a_denied_write_is_reported_and_leaves_the_buffer_recoverable`.
The directory is made unwritable, the save returns
`WriteFailed { kind: PermissionDenied }` rather than an error, and the draft on
disk is asserted to contain the buffer verbatim.

**Disk full: not automated.** `IoKind::classify` is unit-tested for ENOSPC and
EDQUOT (`notes-model`, `error::tests::disk_full_is_distinguishable_from_permission_denied`
and `::quota_reads_as_disk_full`), but **no test fills a filesystem**, so the
path from a real ENOSPC to a visible error and a recoverable buffer is unproven.

Manual step, until it is automated:

```bash
# Linux: a 1 MiB filesystem, mounted, then filled.
truncate -s 1M /tmp/tiny.img && mkfs.ext4 -q /tmp/tiny.img
mkdir -p /tmp/tiny && sudo mount -o loop /tmp/tiny.img /tmp/tiny
sudo chown "$USER" /tmp/tiny && printf '# n\n' > /tmp/tiny/n.md
# Open /tmp/tiny as a workspace, type more than 1 MiB, and check:
#   the status bar reads `error` with "no space left on the disk"
#   the draft exists under <data>/workspaces/<id>/drafts/
#   reopening the note offers to restore it
sudo umount /tmp/tiny
```

**Verdict: partly met.** The error model distinguishes the two causes and the
draft path is proven for one of them; the full-disk path is documented and
unexercised. Automating it needs a loopback filesystem in CI, which is a decision
about CI privileges rather than about this milestone.

## 6. No command accepts a path outside the root

**Automated in the core** — `notes-core`,
`tests/protocol.rs::no_command_accepts_a_path_outside_the_root`, plus the two
halves separately:

- the string half: `notes-model`, `path::tests::rejects_every_escape_shape`
  (`..`, `sub/../../`, absolute, drive letter, backslash, NUL);
- the disk half: `notes-fs`, `tests/jail.rs::a_symlink_out_of_the_root_is_refused_not_followed`
  and `::a_symlinked_directory_is_refused_mid_path`, which also assert the file
  outside the root is byte-identical afterwards.

Both halves are needed: a symlink is a perfectly well-formed relative path that
resolves somewhere else, and the check runs on **every** call rather than at open
time.

## 7. Opening a folder creates no file in it

**Automated** — `notes-core`,
`tests/protocol.rs::opening_a_folder_creates_nothing_inside_it`. It snapshots
every path and size under the folder, opens the workspace, lists it, opens a
note, and compares the snapshot.

This is the criterion that made the case-sensitivity probe read-only
([DECISIONS-0.1a.md](DECISIONS-0.1a.md) D-01).

## 8. `cargo test` passes with no Tauri

**Automated** — `cargo test --workspace`, 123 tests, no window and no display.
No crate under `crates/` depends on `tauri`, and the suite is **green on Ubuntu,
macOS, Windows and an Arch container against rolling `webkit2gtk-4.1`**.

`tools/check.sh` runs the whole gate locally, including a Windows cross-check
that needs no MSVC toolchain.

---

## Scope items

Everything listed under 0.1a in `.continue/SCOPE_final.md` §17:

| Item | Where |
|---|---|
| Open / create workspace | `workspace_open`, `workspace_create` |
| Lazy tree | `tree_list`, one level per call; `src/explorer/Tree.tsx` lists on expand |
| Open, edit, create note and folder | `note_open`, `note_save`, `note_create`, `dir_create` |
| Syntax highlighting | CodeMirror 6 with `@codemirror/lang-markdown` |
| Atomic autosave with `BaseRev` | `WorkspaceService::save_note`, `LocalFs::write_atomic` |
| Recoverable draft | `draft_write`, `draft_list`, `draft_resolve` |
| Status bar | `DocStatus`, seven states, each with a word and a glyph |
| Persist the last workspace | `workspaces.json.last_workspace`, `workspace_restore_last` |
| Dark theme | `src/styles.css`, one theme |

## Not verified

- **The window has never been launched.** The interface compiles, typechecks and
  bundles; no human has driven it. Every result above is from the core and the
  corpus. To run it: `cd apps/notes-app && npm run tauri dev`.
- **The capability matrix is still a specification.** The CI matrix is green on
  Ubuntu, macOS, Windows and Arch as of `0.7.3`, which means the suite passes on
  ext4, APFS and NTFS — but no run exists on SMB, NFS, exFAT or a FUSE mount, so
  the rows of `ARCHITECTURE.md` §11 for those backends are asserted rather than
  observed.
- **The permission-denied criterion does not run as root**, and the Arch CI
  container is root — it skips there rather than passing for the wrong reason
  ([DECISIONS-0.1a.md](DECISIONS-0.1a.md) D-21). It runs on Ubuntu and macOS.

### What four CI rounds found, and none of it was a failing test

Every problem the matrix surfaced stopped the build or the checkout before a test
could run, which is why none of them could have been caught locally:

| Round | Platform | What happened |
|---|---|---|
| 1 | Windows | **`actions/checkout` aborted**: `invalid path 'fixtures/edge-cases/trailing-dot.md.'`. A committed file whose name ends in a dot cannot exist on NTFS, so the repository was unclonable |
| 1 | macOS · Arch · contracts | A path comparison against an unresolved `/var`, a permission test run as root, and a job that built the Tauri app to run a grep |
| 2 | Windows | **Failed to compile**: `native_id` called `windows_by_handle`, an unstable API |
| 2 | macOS | `Duplicate.md` + `duplicate.md` are one file on APFS, and APFS normalises names to NFD — the corpus could not be checked out faithfully |
| 3 | Windows | **Failed to compile**: three helpers used only under `#[cfg(unix)]` are dead code on Windows, which `-D warnings` makes fatal |

The response to the last one was `tools/check.sh`, which cross-checks the Windows
target locally ([DECISIONS-0.1a.md](DECISIONS-0.1a.md) D-25) — one `rustup target
add`, no MSVC toolchain, and it would have caught both compile failures in
seconds.
- **The full-disk path**, as §5 states.
- **Milestone 0.0 remains open**, on hardware this machine does not have —
  [SPIKE-0.0.md](SPIKE-0.0.md). It is orthogonal to this milestone and blocks
  nothing here.
