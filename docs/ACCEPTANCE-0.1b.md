# Acceptance — milestone 0.1b

> **Status:** `ACTIVE` · Every acceptance criterion of milestone 0.1b
> (`.continue/SCOPE_final.md` §17), against a **named automated test** or a
> **documented manual step**. A criterion with neither is listed as not met, and
> the half of one that has neither is listed as not met even where the other
> half is automated.
>
> Measurements were taken on the development machine: Debian 13 (trixie),
> Linux 6.12, ext4 on NVMe, X11, no NVIDIA. Rust 1.96, Node 24, Tauri 2.11.5.
>
> The 0.1a document is [ACCEPTANCE-0.1a.md](ACCEPTANCE-0.1a.md); its criterion 5
> moved from *partly met* to **met** during this milestone and the reasoning is
> there.

## Summary

| # | Criterion | Verdict |
|---|---|---|
| 1 | Editing in another program updates the tab in <1 s without losing the cursor | **met in the core** — the visible half is a manual step, §1 |
| 2 | Rename via the app keeps tab/cursor/id; an unambiguous external rename reconnects; an ambiguous one gets a new id | **met** — automated, as the criterion asks |
| 3 | Create/duplicate never overwrite; a colliding move asks for a resolution | **met** — automated |
| 4 | `fixtures/xss/` in the preview runs no script and loads no external resource | **met** — automated, every file under all four settings |
| 5 | Delete reports `Trashed` or `Permanent`; never deletes without saying which | **met** — automated |
| 6 | The tree appears in <1 s at any size; the watcher walks in the background and a directory it cannot read is counted, not fatal | **met** — automated and measured, §6 (added mid-milestone, from a bug) |

**The application window has still never been launched by whoever wrote this.**
Every result below comes from the core, the corpora and `vitest`. What that
leaves unverified is listed under *Not verified*, in the same terms 0.1a used,
and it is the reason criterion 1 is qualified rather than ticked.

`cargo test --workspace` is **262 tests**; `npm test` is 8. The CI matrix is
green on Ubuntu, macOS, Windows and Arch as of `0.9.1`.

---

## 1. Editing in another program updates the tab in under a second

**Automated as far as a test can take it** — `notes-core`,
`tests/reconcile.rs::a_real_watcher_reports_a_change_within_the_debounce`. It
starts a real watcher on a real directory, writes the file from outside the
application, and asserts that the path reaches the reconciler and comes back as
`FsChanged { kind: Modified }` **inside 900 ms**, measured. On a machine that
cannot watch — a kernel out of inotify budget — it skips and says so rather than
passing for the wrong reason.

The clock, end to end: the watcher's own debounce is 200 ms
(`notes-fs/src/watch.rs`), the frontend ticks every 300 ms
(`src/stores/sync.ts`), so the worst case is ~500 ms before the reload starts.

**The cursor half is asserted in the editor, not in the core.** Replacing the
text is one CodeMirror transaction with the selection clamped and preserved —
`src/editor/Editor.tsx`, `EditorBody` — rather than a rebuild of the view, which
would discard the undo history and put the caret at the top of the note.
`reloadFromDisk` refuses to touch a buffer that is not clean, so the reload
cannot race an edit.

**Not verified: that a human sees it happen.** No window has been launched.
The manual step:

```bash
cd apps/notes-app && npm run tauri dev
# 1. Open a folder with a note in it, and open the note.
# 2. In VS Code (or `printf` from a shell), append a line to the same file.
# 3. The tab updates within a second; the caret stays where it was.
# 4. Now type in the app first, leave the buffer dirty, and write from outside
#    again: the status bar reads `conflict`, nothing on disk is overwritten,
#    and "Compare" shows both versions.
```

## 2. Rename keeps identity; an external rename reconnects, ambiguity does not

**Automated in the core, as the criterion requires.**

| Half | Test |
|---|---|
| Rename via the app keeps the `NoteId` | `notes-core`, `tests/entries.rs::renaming_a_note_keeps_its_identity` |
| …including every note inside a renamed folder | `::renaming_a_folder_carries_the_notes_inside_it` |
| …and not a sibling whose name merely starts the same way | `::a_sibling_with_a_similar_name_is_not_dragged_along` |
| A move keeps it too | `::moving_a_note_keeps_its_identity_and_its_name` |
| An unambiguous **external** rename reconnects | `notes-core`, `tests/reconcile.rs::an_unambiguous_external_rename_reconnects_the_same_note` |
| An ambiguous one gets a new id | `::an_ambiguous_external_rename_yields_a_new_identity` |
| A zero-byte file is never correlated by content | `::an_empty_file_is_never_correlated_by_content` |
| A rename the app performed never enters correlation at all | `::a_rename_the_app_performed_produces_no_correlation_work` |

**The tab half** — that the buffer, the cursor and the dirty state survive — is
`useEditor.repath` in `src/stores/editor.ts`: the `NoteId` did not change, so
only the path moves and nothing is re-fetched. Not covered by a `vitest` case;
the store is exercised only through the window, which has not been launched.

## 3. Create and duplicate never overwrite; a colliding move asks

**Automated** — `notes-core`, `tests/entries.rs`:

- `::duplicating_a_note_never_overwrites_and_gets_a_new_identity` — `create_new`
  throughout, `nota (copy).md` then `nota (copy 2).md`, and the first copy is
  asserted byte-identical after the second;
- `::duplicating_a_folder_copies_the_tree`;
- `::a_move_onto_an_existing_name_is_refused_and_says_so` — `AlreadyExists`
  **naming** the path, which is what lets the interface ask instead of guess,
  and the file in the way is asserted untouched;
- `::a_rename_onto_an_existing_name_is_refused_before_anything_moves`;
- `::a_folder_cannot_be_moved_inside_itself`;
- `::a_rename_to_a_name_no_filesystem_can_hold_is_refused` — the portability
  rules of scope §7.6, so a workspace stays carryable between machines.

`note_create`'s half of "never overwrites" is 0.1a's
`protocol.rs`, unchanged.

## 4. `fixtures/xss/` executes nothing and loads nothing

**Automated, and the whole folder is the test** — `notes-markdown`,
`tests/xss.rs`. Every `.md` in `fixtures/xss/` is rendered under **all four
combinations** of `raw_html` and `remote_images` and checked against the
invariants: no forbidden tag, no `on*` attribute, no scheme outside
`http`/`https`/`mailto`/`notes-asset`/`data`, no `data:` outside the raster
allowlist, no `notes-asset://` for another workspace or containing `..`, no
remote `src` while remote images are off, and every `<input>` a checkbox with no
name and no value.

```
cargo test -p notes-markdown --test xss     # 22 tests
```

**The assertions are structural — tags and attributes read back out of the
sanitized HTML, never substrings** — and `safe-in-code.md` is why: it has to
render `javascript:alert(1)` **as text**, so a suite that greps the output for
`javascript:` would demand the opposite of what the corpus requires
([DECISIONS-0.1b.md](DECISIONS-0.1b.md) D-04's reasoning applies to both
corpora).

`every_file_in_the_corpus_is_safe_under_every_setting` is a **census**: adding a
payload to the folder is enough, and forgetting to write a test for it cannot
make it pass. Each file also keeps a named test asserting it was refused for the
right reason **and that the rest of the note still rendered** — scope §8.4,
*"bloquear recurso não impede ler o resto da nota"*.

The second layer is `notes-core`, `tests/preview.rs`: the `notes-asset://`
handler is a second entry point into the workspace and applies the same root
jail, proven with a symlink out of the root
(`::the_asset_path_obeys_the_same_root_jail_as_every_command`, which also asserts
the file it pointed at is untouched), and serves image types only, so the
preview cannot be used to read one note into another.

**Not verified: that nothing loads at runtime in a real WebView.** The
assertions are on the HTML the core produces and on the CSP in
`tauri.conf.json`; no page has been opened. The manual step:

```bash
# With the app running, open fixtures/xss/*.md one at a time and watch the
# WebView's network panel and console. Expect: no requests to example.invalid,
# no `window.__pwned`, and every note still legible.
```

## 5. Delete says which of the two it did

**Automated** — `notes-core`, `tests/entries.rs`:

- `::deleting_says_which_of_the_two_happened_and_forgets_the_note` — the outcome
  is always `Trashed` or `Permanent`, the note leaves the registry, and the ids
  come back so the tab can be closed;
- `::deleting_a_folder_forgets_every_note_beneath_it`, and nothing outside it;
- `::deleting_a_note_leaves_its_draft_alone` — a note deleted while it held
  unsaved edits is precisely the case where the draft is the only copy of them.

The fallback from the bin to a permanent delete exists — a removable exFAT
stick, a network share, a container with no session bus — and **it is never
silent**: `DeleteOutcome` carries it and the interface says a different sentence
for each ([DECISIONS-0.1b.md](DECISIONS-0.1b.md) D-11).

**Not verified: that the file is really in the desktop's bin.** The test asserts
the outcome the platform reported, not that a file manager shows it. The manual
step: delete a note from the app on a Linux desktop, then check
`~/.local/share/Trash/files/`.

---

---

## 6. The tree appears in under a second, at any size

**Added mid-milestone, from a bug.** The owner opened `~/x` — around 160
repositories with `node_modules/`, `target/` and `.git/` — and the Welcome screen
stayed on screen for **more than two minutes** before the tree appeared. That is
not a notes workload, and it does not have to be: the rule this criterion states
is that the application does not freeze on **any** folder
([ADR-034](decisions.md), [DECISIONS-0.1c.md](DECISIONS-0.1c.md) D-09).

Two halves belong to 0.1b, because both are the watcher's:

**Automated** — `notes-core`, `tests/deep.rs`:

| Test | What it holds to |
|---|---|
| `notes-core`, `deep.rs::the_tree_appears_in_well_under_a_second` | opening a workspace of 2 160 directories and listing its root, **under 1 s** |
| `notes-core`, `deep.rs::starting_the_watcher_returns_immediately_and_walks_behind` | `start_watch` returns in under a fifth of the walk it then waits for, over **7 200 directories** |
| `notes-core`, `deep.rs::an_unreadable_directory_does_not_demote_the_workspace` | a mode-000 subdirectory is **counted and skipped**; `degraded` stays `None` and the rest stays watched |
| `notes-fs`, `watch_walk.rs::the_walk_reports_its_progress_and_finishes` | the counters the status bar reads are real: over 300 directories walked, and `walking` turns off |
| `notes-fs`, `watch_walk.rs::dropping_the_watch_stops_the_walk_where_it_is` | **cancellable**: a `Watch` dropped into an 8 000-directory walk stops it there rather than finishing a workspace nobody has open |

The mode-000 tests **probe before they assert**. Root reads such a directory
anyway — `CAP_DAC_OVERRIDE` — and the Arch CI job runs the suite as root, so a
test that asserted regardless would be asserting about the runner. It creates a
mode-000 directory, checks that reading it actually fails, and skips with a
reason when it does not. The per-directory counters are Linux's, for the same
reason the walk is (`DECISIONS-0.1c.md` D-10).

The assertion on `start_watch` is a **ratio against the walk measured in the
same test**, not a millisecond budget, and that is deliberate: at this corpus
size a synchronous walk costs ~30 ms, which any absolute budget worth writing
would let through. The regression it exists to catch was verified by putting it
back — restoring the inline walk fails the test with
*"start_watch returned in 33.09 ms of a 58.45 ms walk over 7200 directories —
it is walking the tree inline"*.

**And the full fixture runs in CI too.** The Linux job generates
`fixtures/deep` — 20 962 directories, the mode-000 directory and the symlink
loop — and runs the measurement as a criterion:
`open_workspace` + `list_dir` under a second, `start_watch` and the first
`quick_open` each under 100 ms, `quick_open` returning `Ok` rather than
`PermissionDenied`, and `degraded` staying `None`. It is Linux-only for the same
two reasons the rest is: the counters are inotify's, and it is the one runner in
the matrix that is not root.

**Measured on the real shape**, `tools/gen-deep.sh` plus
`cargo test -p notes-core --test deep -- --ignored --nocapture`, against
**20 962 directories** with a mode-000 directory and a symlink loop in it:

| Step | Before | After |
|---|---|---|
| `open_workspace` + list root | 1.13 ms | 1.22 ms |
| `start_watch` | **502.72 ms**, inline, mutex held | 0.30 ms, walk in the background |
| first `quick_open` | **549.88 ms**, inline, mutex held | 0.07 ms, partial answer |
| everything | **1 053.73 ms** | **1.59 ms** |
| with the unreadable directory in place | everything aborted; `quick_open` returned `Err(PermissionDenied)` | 1 directory counted, everything else served |

The "before" column was measured with the unreadable directory temporarily made
readable, because with it in place there was nothing to measure: the run aborted
in under a millisecond.

**And on the folder itself.** `fixtures/deep` is a reconstruction; `~/x` is the
original, and `deep.rs::where_the_time_goes_on_a_real_folder` measures it
directly (`NOTES_DEEP_ROOT=~/x cargo test -p notes-core --test deep -- --ignored
--nocapture`). Nothing in that test writes to the folder — `open_workspace`
reads, and the case probe is read-only by construction (scope §2.3):

```
root:              /home/samir/x
open_workspace:        0.42 ms
list root:             0.17 ms   (44 entries)
start_watch:           0.17 ms   (degraded: None)
quick_open first:      0.05 ms   (0 matches, building true, 0 indexed)
to a usable tree:      0.59 ms
watch walk done:    7258.65 ms   dirs 49937 · unreadable 1 · over_limit 0
index done:        15756.08 ms   (56622 notes, 1 unreadable)
```

**Over two minutes to under a millisecond**, and the two walks that used to
cost it are now 7.3 s and 15.8 s of background work with the window usable
throughout. The `unreadable: 1` is `.../www/web1/ead` — the directory whose
`Permission denied (os error 13)` was in the banner. It is a number now, and it
stopped the workspace from being watched at all before.

**Not verified: a workspace that overruns the watch table.** `over_limit: 0`
above because this machine's `max_user_watches` is 1 048 576 and `~/x` needs
49 937. A default Linux ships 8 192 or 65 536, where the same folder leaves tens
of thousands of directories over the limit — the state the banner exists to
name.

Reaching it deliberately would mean lowering the sysctl, and the inotify limits
are not writable from an unprivileged user namespace on this kernel; the ENOSPC
suite does that trick for the disk, and it does not work here. What **is**
asserted is the half that decides which sentence the user reads:
`watch.rs::a_full_watch_table_is_told_apart_from_a_missing_path_and_names_the_sysctl`
— errno 28 classifies as `WatchLimit` and its message carries
`fs.inotify.max_user_watches`, errno 2 does not and must not offer a command
that would not help. The behaviour behind it — keep the watches installed, count
the remainder, do not demote — is code review, not a test.

The tree was never the problem — it costs a millisecond on 21 000 directories,
because it is lazy and reads one directory at a time. The freeze was two
whole-tree walks on the critical path, each inside a `#[tauri::command]` holding
`Mutex<WorkspaceService>`, so `tree_list` did not run slowly: it did not run at
all until they finished.

**Where the numbers appear.** The walk's progress is the status bar's, because
it is transient — *"indexing folders… N watched"*, gone the moment the walk
ends. The two states that **settle** are banners: a full watch table names how
many directories did not fit and the `sysctl` that raises the limit, and an
unreadable folder names how many were skipped. A count in a corner of the status
bar is not something a user can act on; a sentence with the command in it is.

**Not verified: that either reads correctly on screen.** The counters are
asserted in the core; the sentence a person reads is not. The manual step, which
the owner walks against the installed `.deb`:

```bash
tools/gen-deep.sh                 # or simply open ~/x, or any checkout-heavy folder
# Open it from the app. Expect: the tree on screen at once, not after a wait;
# the status bar saying "indexing folders… N watched" and then falling silent;
# and, on a machine whose watch table fills, a banner naming how many folders
# were left out and the sysctl that raises the limit.
```

**What this milestone's fixtures could not have caught.** `fixtures/large` is
10 000 notes, flat, and lists in 37 ms. The axis that broke was **directories**,
and nothing in the project measured it until `fixtures/deep` did.

## Scope items

Everything listed under 0.1b in `.continue/SCOPE_final.md` §17:

| Item | Where |
|---|---|
| rename, move, duplicate, delete (trash) | `entry_rename`, `entry_move`, `entry_duplicate`, `entry_delete`; the tree's context menu |
| watcher + reconciliation on focus | `notes-fs/src/watch.rs`, `notes-core/src/reconcile.rs`, `src/stores/sync.ts` — a 300 ms tick, a full scan on focus and every 5 s |
| conflict UI | `src/conflict/Compare.tsx` + `conflict_resolve`; scope §12's four resolutions, with *compare* as a screen (`ARCHITECTURE.md` §17.1) |
| preview and split | `Ctrl+E`, `src/preview/Preview.tsx`, `markdown_render`; the mode is remembered per workspace in `session.json` |
| search/replace in the file | `@codemirror/search` in `src/editor/Editor.tsx` — `Ctrl+F` and `Ctrl+H`, on the buffer in front of the user. Global search is 0.1c and is a different thing |

## What the preview IR costs

`ARCHITECTURE.md` §10 chooses sanitized HTML over an AST, and the instruction
for this milestone was to **record the number rather than optimise on
intuition**. `cargo test -p notes-core --test cost -- --ignored --nocapture`:

| Source | HTML | JSON | render | serialise | outline |
|---|---|---|---|---|---|
| 337 B (a typical note) | 517 B | 782 B | 0.293 ms | 0.028 ms (8.8%) | 0.055 ms |
| 2.5 KiB (the whole markdown corpus) | 5.1 KiB | 6.8 KiB | 3.35 ms | 0.224 ms (6.3%) | 0.372 ms |
| 491 KiB (that corpus ×200) | 1.0 MiB | 1.3 MiB | 647 ms | 43.9 ms (6.4%) | 70.5 ms |
| 5.0 MiB (`edge-cases/large-5mb.md`) | 5.0 MiB | 5.0 MiB | 682 ms | 151 ms (18.1%) | 163 ms |

**Turning `Rendered` into JSON is 6–9% of render-plus-serialise at any size a
person writes**, and 18% at the 5 MiB edge case. It is not where the time goes,
and nothing was engineered around it.

Two things the profile *did* say. `ammonia`'s builder was being assembled per
render — 0.385 ms → 0.293 ms for a 337-byte note once it is built once, which is
a small number and is stated small. And **the cost tracks element count rather
than bytes**: 1 MiB of dense HTML (tables, footnotes, thousands of headings)
costs about as much as 5 MiB of prose. A note of that shape re-renders in about
650 ms, which the 300 ms debounce hides from typing but not from the eye.
Nothing in 0.1b needs that faster; a note that large is the thing to measure
again if anyone complains.

---

## Verified in the running app

Everything above is a test of `notes-core`, and that is the hole this milestone
exposed: **six flows shipped dead behind a dialog the WebView does not have, and
every criterion was green.** A criterion satisfied in the core says nothing about
the interface, so this section exists and is separate.

The application was launched on Debian 13 / X11 with the Vite server running:

```bash
cd apps/notes-app && npm run dev &      # or: npm run tauri dev
NOTES_DATA_DIR=/tmp/nd ./target/debug/notes-app
```

### Verified — observed on screen

| # | Flow | How |
|---|---|---|
| V1 | The application starts and paints the Welcome screen | Fresh `NOTES_DATA_DIR`; title, subtitle and both buttons render |
| V2 | The last workspace is restored on launch, with no dialog | `workspaces.json` seeded with `last_workspace`; the tree, the editor and the status bar came up on their own |
| V3 | The tree lists a workspace and marks notes from non-notes | Directories, `.md` files and the ignored entries behaved as the core says |
| V4 | Opening a note from the sidebar loads it into CodeMirror | Content, line numbers and syntax highlighting present |
| V5 | **Split renders the preview beside the source** | The heading rendered as a heading and the paragraph as a paragraph, from `notes-markdown` through the IPC |
| V6 | The status bar reports `✓ saved` after a clean open | With the note's path beside it and the workspace name at the right |

### Not verified — the flows that need a person

**None of the twelve steps below has been walked**, and the reason is a
limitation of the machine rather than a judgement about the code: this window
manager refuses to raise the application window (`xdotool windowactivate`
returns `_NET_ACTIVE_WINDOW failed`; `windowraise` and `wmctrl -a` do nothing),
and WebKit does not act on synthetic clicks or keys delivered to an unfocused
window. The window can be photographed and cannot be driven.

Walk them with the application in front of you. **A box left unticked is a flow
nobody has seen work.**

> **The steps below describe the 0.1a–0.1c window, which 0.1d replaced.** The
> behaviour is unchanged and these expectations still stand; where you *press*
> does not. Walk them from
> [ACCEPTANCE-0.1d.md §3](ACCEPTANCE-0.1d.md), which carries the same twelve
> rows with their steps rewritten for the interface that exists — and tick them
> there, and here, only after the owner has walked them twice (ADR-037).

| # | Step | Expected |
|---|---|---|
| U1 | *New note* → type `nota de teste` → **Create** | The modal appears, the note is created and opens; `Cancel` and `Escape` each leave nothing behind |
| U2 | *New note* → leave the field empty → **Create** | Refused **in the dialog**, with "A name is required."; the core is never called |
| U3 | *New folder* → type `pasta` → **Create** | The folder appears in the tree |
| U4 | Welcome → *Create Workspace…* → pick a parent → name it | Native picker for the parent, the application's own modal for the name |
| U5 | Right-click a note → *Rename…* → change the name | The tab keeps its cursor and identity; the sidebar shows the new name |
| U6 | Right-click a note → *Move to…* → type a folder | The note moves; an **empty field means the workspace root** and must not be read as a cancellation |
| U7 | Right-click a note → *Delete…* | A **destructive** confirm; on cancel nothing happens; on confirm the status line says *trashed* or *permanent* and which |
| U8 | `Escape` on any of the above | Cancels, and focus returns to the control that opened it |
| U9 | *Source* / *Preview* / *Split* | Three modes; `Ctrl+E` cycles |
| U10 | Edit a note in another editor while it is open and dirty here | Conflict banner, autosave suspended, and the **compare screen** shows both versions |
| U11 | Resolve the conflict three ways | *Keep mine*, *use the disk*, *save as a copy* — the version not chosen lands in `conflicts/` |
| U12 | `Ctrl+F` → search and replace inside the note | Matches highlighted, replace applies, `Escape` closes |

### What is machine-checked instead

`src/app/dialog.test.ts` covers the contract those six flows depend on — that a
request resolves, that cancelling resolves `null` for text and `false` for a
confirm, that the **empty string survives as an answer** rather than collapsing
into a cancellation (U6 depends on exactly that), that the validator refuses
before the core is asked, and that a second request cancels the first instead of
stacking, so no caller is left awaiting a promise nobody will settle.

That is the part of this hole a machine can close. It does not replace U1–U12: a
dialog that resolves correctly and never renders passes every one of those tests.

`tools/no-blocking-dialogs.sh` is the other half — it fails the build if a
browser script dialog returns to the frontend, in `npm run lint`, `npm run
build`, `tools/check.sh` and CI.

---

## Not verified

- **The window has never been launched by whoever wrote this milestone.** The
  interface compiles, typechecks, bundles and has 8 `vitest` cases over the
  conflict diff. Everything else about it — that the preview paints, that
  `Ctrl+E` cycles, that the comparison screen reads well, that the context menu
  lands where the pointer is — is unobserved. To run it:
  `cd apps/notes-app && npm run tauri dev`.
- **The `notes-asset://` scheme handler has never served a byte.** Its logic is
  tested through `read_asset`; the Tauri registration, the URL shape on Windows
  (`http://notes-asset.localhost/…`) and the CSP interaction are asserted by
  reading, not by running.
- **`shell_open` has never opened a browser.** The scheme check is tested by
  inspection only.
- **The trash has never been looked at in a file manager**, as §5 says.
- **`fixtures/xss/` has never been rendered in a WebView**, as §4 says.
- ~~No CI run exists for this milestone yet.~~ **The matrix is green on all
  four platforms as of `0.9.1`** — Ubuntu, macOS, Windows and the Arch container
  against rolling `webkit2gtk-4.1` — plus the contracts job, the frontend job
  (typecheck, `vitest`, build) and the 1 000-round crash loop. It took a
  correction: the Linux leg had been red since `0.7.5` on the ENOSPC step alone,
  because a GitHub runner forbids the unprivileged user namespace the test
  mounts its filesystem in. That is the `0.9.1` entry in `CHANGELOG.md`, and the
  scope §19 rule — *"sem verde nos quatro, marco desktop não fecha"* — is
  satisfied by that run rather than by this one being written.
- **Milestone 0.0 remains open**, on hardware this machine does not have
  ([SPIKE-0.0.md](SPIKE-0.0.md)). It is orthogonal to this milestone and blocks
  nothing here.

### What this milestone found in 0.1a

Two defects, both invisible to a green 0.1a gate because they were on paths
nothing exercised:

| What | Where it was found |
|---|---|
| `RelPath::root()` **serialises** to `""` and `TryFrom<String>` **refused** `""`, so every `tree_list` of the workspace root was rejected by argument deserialisation before the command body ran — the sidebar's first call on every launch | Wiring `notes-markdown` into the IPC ([DECISIONS-0.1b.md](DECISIONS-0.1b.md) D-05) |
| `ACCEPTANCE-0.1a.md` §3 quoted 22 edge-case files saved unchanged, from before D-20 and D-23 removed the names no target filesystem could hold; the corpus is 21 | Running the byte-preservation script while wiring the ENOSPC test |
