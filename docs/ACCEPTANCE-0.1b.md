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
