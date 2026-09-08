# Acceptance — milestone 0.1c

> **Status:** `ACTIVE` · Written from the first commit of the milestone rather
> than at the end, and the **"verified in the running app"** section exists from
> the same moment. Milestone 0.1b shipped five green criteria over six dead
> flows because every criterion was an assertion about `notes-core`; a section
> that only appears once the work is finished is a section that agrees with
> whatever was built.
>
> **Every UI row starts unverified and is only ticked once the owner has seen
> it.** Nothing here is ticked by the machine that wrote it.

Milestone 0.1c is `.continue/SCOPE_final.md` §17: quick open · global search by
scan · tabs with restoration · command palette · minimum settings · `en`/`pt-BR`.

**Not in this milestone**, and not pulled forward: `notes-index`, SQLite, FTS5
and the `registry.db` move. §17 puts them at **0.2**, §10 says the 0.1c search is
a scan (`ignore` + `regex`) and that FTS5 takes over word search at 0.2, and
[ADR-015](decisions.md) is `ACTIVE` saying the registry moves at 0.2.

## Summary

| # | Criterion | Verdict |
|---|---|---|
| 1 | First result in `fixtures/large` in <500 ms, and cancellable | **met** — measured |
| 2 | Reopening restores workspace, tabs, active tab and cursor | **met in the core and the store** — the visible half is C10, unwalked |
| 3 | Quick open answers immediately from an index built in the background, and says when it is partial | **met** — automated and measured, §3 (added mid-milestone, from a bug) |

---

## 1. First result under 500 ms, cancellable

**Automated** — `notes-core`,
`tests/search.rs::the_first_result_arrives_in_under_500ms_and_the_search_can_be_cancelled`,
`#[ignore]`d because it needs the generated corpus:

```bash
tools/gen-large.sh
cargo test -p notes-core --test search -- --ignored --nocapture
```

Measured against `fixtures/large` — 10 000 notes, 197 MiB:

```
first result:      11.4 ms
cancel returned:   650 ns
```

Two orders of magnitude under the criterion. The shape is what makes it hold:
the walk is parallel, hits are pushed as they are found rather than collected
and returned at the end, and every worker checks the cancel flag before each
file — so cancelling is bounded by one file, not by the workspace. Dropping a
`Search` cancels it, which is why starting a new query cannot leave the previous
one scanning 197 MiB for nobody.

Fifteen more tests in the same file cover what the timing does not: literal
queries are **not** read as patterns, regex mode is separate and named, case
sensitivity is opt-in, an invalid pattern is refused instead of scanning for
nothing, only notes are searched, the ignore list is honoured, and — scope
§10 — **search reads the disk, so an unsaved buffer is not reported as found**.

## 2. Reopening restores workspace, tabs, active tab and cursor

**Automated on both sides of the IPC**, because the criterion spans both.

`notes-core`, `tests/session.rs` — five tests. Tabs, the active tab and the
cursor survive a restart through a *different* `WorkspaceService` over the same
data directory; a note keeps its `NoteId` across that restart, which is what lets
a tab find it again; an unreadable session starts empty rather than refusing to
open the workspace; a session written by a newer build costs an empty session
rather than a read-only workspace, because session state is resettable and the
registry is not; and saving a session **touches nothing in the user's folder**.

`apps/notes-app`, `src/stores/tabs.test.ts` — twenty tests over the store that
puts them back. Opening, activating, closing to the right then to the left, the
cursor remembered per tab and handed to the editor when it mounts, the session
round-trip in its own shape with the other stores' fields preserved, a tab whose
note is gone dropped rather than left failing on every click — and the two that
matter most, because a tab strip is the likeliest place to lose a buffer:
**leaving a dirty note flushes it**, and **leaving a note in conflict writes its
draft instead of saving**.

Cursor restoration is ordered, not incidental: the position is applied *after*
the editor mounts the document, and reports from a mounting editor are ignored
while a restore is in flight — otherwise the caret at 1:1 overwrites the one
being restored. A test asserts exactly that.

**What no test covers** is that the restored caret is where the user left it *on
screen*. That is C10.

---

## 3. Quick open answers immediately, from an index built in the background

**Added mid-milestone, from the same bug as
[ACCEPTANCE-0.1b.md §6](ACCEPTANCE-0.1b.md)** — the Welcome screen frozen for
over two minutes on `~/x`. The watcher's half of that is 0.1b's; this is quick
open's half, and it is the one that was doing the most damage.

**Automated** — `notes-core`,
`tests/deep.rs::quick_open_answers_immediately_and_admits_it_is_still_indexing`,
over **7 200 directories** with a mode-000 directory and a symlink loop in place:

- the first call returns in under a fifth of the index build it then waits for —
  a **ratio against the build measured in the same test**, not a millisecond
  budget, so the assertion says what the rule says: answering does not include
  walking. Restoring the inline build fails it with *"the first quick_open took
  84.54 ms of a 84.54 ms index build over 7200 directories — it is walking the
  tree inline"*;
- an answer given while the walk is still running carries `building: true` and
  the count indexed so far, and the palette shows that instead of "nothing
  matches";
- the unreadable directory is **counted, and the workspace is still searched**.
  This is the second bug the deep fixture found and it was not a timing bug:
  before this change, one mode-000 subdirectory made `quick_open` return
  `Err(Io { op: "read_dir", kind: PermissionDenied })` for a workspace of 21 000
  directories — quick open returned nothing at all, for the whole folder;
- every `README.md` under `node_modules/` is indexed. The watcher's skip list is
  **not** the visibility list, and `node_modules/` and `target/` are deliberately
  absent from `IGNORE_DEFAULT` ([DECISIONS-0.1c.md](DECISIONS-0.1c.md) D-08).

**Measured on the real shape**, `tools/gen-deep.sh` plus
`cargo test -p notes-core --test deep -- --ignored --nocapture`, **20 962
directories**: the first `quick_open` cost **549.88 ms** before and under a
millisecond after, and it cost it inside a `#[tauri::command]` holding
`Mutex<WorkspaceService>` — which is why the *tree* was what appeared to hang.

**Not verified: what the palette looks like while the index fills.** On a
workspace large enough for the window to exist, `Ctrl+P` in the first second
shows a partial list under the line *"still indexing — N notes so far"*. C1 and
C3 cover the palette itself; nobody has watched it fill.

---

## Verified in the running app

Every criterion above is an assertion about `notes-core`. **Milestone 0.1b
shipped with five of those green and six flows dead** behind a dialog the WebView
does not have, so this section is where the interface is accounted for
separately, and it exists from the first commit rather than the last.

**Nothing below is ticked.** A row becomes `verified` only after the owner has
walked it and said so.

> **What a screenshot showed, which is not a tick.** The application was started
> against a seeded session of two tabs with `view_mode: split`. It came up with
> both tabs present, the active one marked, its note loaded, Split restored and
> the preview rendering. That is evidence for part of **C10** and it is not C10:
> the caret position is not visible in a screenshot, and nobody has clicked
> anything.

| # | Flow | Expected | Verified |
|---|---|---|---|
| C1 | `Ctrl+P` → type part of a name | The palette lists matching notes, best match first; `Enter` opens it | ☐ |
| C2 | `Ctrl+P` → a query that matches nothing | Says so; does not close, does not open anything | ☐ |
| C3 | `Ctrl+P` after creating a note | The new note is offered without restarting the app | ☐ |
| C4 | `Ctrl+Shift+F` → search a word | Results stream in, with path, line and the matching line | ☐ |
| C5 | `Ctrl+Shift+F` while a note has unsaved changes | The panel **says results come from what is on disk** | ☐ |
| C6 | `Ctrl+Shift+F` → cancel mid-search | Stops; partial results stay on screen and say they are partial | ☐ |
| C7 | Switch the search mode literal ↔ regex | The mode is named on screen and does not change underneath a running query | ☐ |
| C8 | Click a result | Opens that note **at that line** | ☐ |
| C9 | Open three notes | Three tabs; the active one is marked; `Ctrl+W` closes one | ☐ |
| C10 | Quit and reopen | Workspace, tabs, **active tab and cursor position** all come back | ☐ |
| C11 | `Ctrl+Shift+P` → command palette | Lists commands; `Enter` runs one; `Escape` closes | ☐ |
| C12 | Settings → font size, line numbers, wrap, tab size | Each applies to the editor and survives a restart | ☐ |
| C13 | Switch the interface language | Every visible string changes; no key is left showing raw | ☐ |

### Standing rules this milestone inherits

- `tools/no-blocking-dialogs.sh` still fails the build if a browser script dialog
  returns to the frontend.
- Every new user-visible string is a key in **both** catalogues; CI fails when
  they diverge.
