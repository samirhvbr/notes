# Decisions taken while building 0.1c

> **Status:** `ACTIVE` · Calls made under the standing rule — *choose the
> simplest option compatible with the scope, write the missing section into
> `ARCHITECTURE.md`, and carry on*. One row per decision: what was decided,
> which gap it closed, and **what the alternative is** if the owner disagrees.
>
> The structural ones are promoted to [`decisions.md`](decisions.md) as
> **ADR-030 … ADR-032** and **ADR-034**; the rest are here so that none of them
> is invisible.

---

## D-01 — `notes-index`, SQLite and `registry.db` are **not** in this milestone

**Decided.** 0.1c ships quick open, a scan-based global search, tabs with
restoration, the command palette, the minimum settings and `en`/`pt-BR`. The
index and the registry migration stay at 0.2.

**Why, and why it is written down rather than assumed.** The instruction that
opened this milestone asked for `notes-index`, SQLite and the `registry.db` move
of ADR-015 "because the debt falls due in this milestone". Three sources say
otherwise, and one of them is binding:

- `SCOPE_final.md` §17 lists the index, FTS5 and incremental indexing under
  **0.2**, and 0.1c without them;
- §10 says the 0.1c global search is a **scan** (`ignore` + `regex`) and that
  FTS5 takes over word search at **0.2**, with the scanner staying for literal
  and regex;
- **[ADR-015](decisions.md) is `ACTIVE`** and says the registry moves to
  `registry.db` at 0.2.

Building them here would contradict an ACTIVE ADR, which is a stop condition,
and would make the milestone's own acceptance criteria untestable as written —
the search criterion measures a scan.

**Alternative if you disagree.** Move them into 0.1c deliberately: that needs an
ADR superseding ADR-015 and an edit to `SCOPE_final.md` §17 and §10, in that
order, so the roadmap and the decision log do not disagree with the code.

---

## D-02 — One search at a time, and starting one cancels the last

**Decided.** The service holds a single `Search`. `search_start` drops the
previous, and dropping cancels it.

**Gap closed.** Nothing in `ARCHITECTURE.md` said what happens to a running scan
when a new query arrives.

**Why.** The caller is a search box. Every keystroke that reaches "search" makes
the previous query obsolete, and a walk nobody is waiting for is 197 MiB of I/O
spent on a result that will be thrown away. Cancellation on drop is what makes
this true without the caller having to remember.

A poll for a superseded id answers *done, cancelled, no hits* rather than
erroring: by the time a late poll lands the user has already typed again, and
that is not a failure.

**Alternative if you disagree.** Keep several searches alive and address them by
id, which is a feature nothing asks for and a way to have four walks running.

---

## D-03 — The scan stops at 2 000 hits, and says that it did

**Decided.** `MAX_HITS = 2_000`; the walk quits and `SearchProgress.truncated`
is set. Files over 8 MB are skipped by content search.

**Gap closed.** Neither limit was specified.

**Why.** A query of `e` over ten thousand notes is a request to stream a million
lines into a WebView. The honest answer is the first few thousand **plus the
fact that it was cut**, which is why `truncated` is a field rather than a silent
cap — a list that stops without saying so reads as "there is nothing more".

The size limit is not about notes: the corpus has a 5 MB note on purpose and it
is searched. A 200 MB file in a workspace is not a note, and scanning it stalls
the walk that the 500 ms criterion depends on.

**Alternative if you disagree.** No cap, and let the frontend virtualise a
million rows.

---

## D-04 — `Ctrl+Shift+F` is the workspace, `Ctrl+F` stays the file

**Decided.** The shifted binding opens the workspace panel; the unshifted one
continues to reach CodeMirror's in-file panel from 0.1b.

**Gap closed.** Both exist now and they compete for the same key.

**Why.** It is scope §34's own table. It also matches the distinction that
matters: `Ctrl+F` searches **what is being typed**, and the workspace search
reads **what is saved**. Giving them the same key would put a user one modifier
away from a different answer to the same question.

**Alternative if you disagree.** One box that searches both, which would have to
explain why one half of its results is stale.

---

## D-05 — Settings apply as they change, with no Save button

**Decided.** Every control writes through `settings_set` immediately.

**Why.** The same reason a note has no Save button: a panel that can be closed
with unsaved changes is a way to lose them. The application already treats "the
user did something" as the moment to persist, and a settings dialog that behaved
differently would be the one place in it that does not.

**Alternative if you disagree.** A Save button, and a confirm on close for
unsaved settings — two dialogs to avoid one write.

---

## D-06 — Editor settings rebuild the view, and are in the dependency list

**Decided.** Font size, line numbers, wrapping and tab size are in the effect
that builds the `EditorView`; changing one rebuilds it.

**Gap closed.** How settings reach an editor that was built without them.

**Why.** Line numbers and wrapping are CodeMirror **extensions**, not props;
there is no honest way to change them on a live view without a compartment and a
reconfiguration, which is more machinery than four settings justify. A rebuild
costs the undo history, and that is stated rather than hidden: it happens when
the user changes a setting, which is a moment they are not mid-thought in the
text.

**Alternative if you disagree.** A `Compartment` per setting, reconfigured in
place — correct, and four more moving parts.

---

## D-07 — A tab strip is not a reason to make the editor multi-document

**Decided.** See [ADR-030](decisions.md). Recorded here as the call it was:
tabs are a **separate store** that owns the list, and the editor keeps owning the
one loaded document.

**Alternative if you disagree.** Refactor the editor store to hold a map of
documents, which is the shape it will need if two notes are ever visible at
once — and which puts the write protocol, the draft rules and the conflict state
back in play for a navigation feature.

---

## D-08 — `node_modules/` and `target/` are **not** hidden; they are **not watched**

**Decided.** `IGNORE_DEFAULT` keeps the four entries it has — `.notes`, `.git`,
`.obsidian`, `.trash` — and gains neither `node_modules/` nor `target/`. A
separate list, `WATCH_SKIP` in `notes-fs::watch`, decides what the **watcher**
descends into, and that one holds `node_modules`, `target`, `vendor`, `dist`,
`build`, `.git`, `.svn`, `.hg`, `.cache` and `__pycache__`.

**The question, and the rule that answers it.** The owner asked whether the two
go into the default ignores, and said the answer is a product call decided by the
one-second rule (D-09) — that not ignoring them is acceptable as long as the
tree still appears in under a second. It does: on `fixtures/deep`, a corpus of
**20 962 directories** shaped like the folder that froze the application,
`open_workspace` plus listing the root costs **1.13 ms**, because the tree is
lazy and reads one directory at a time. Directory count is not what the tree
pays for. So the rule does not force the hiding, and the default answer to
"should the application hide some of the user's files by name" is no.

It matters that `node_modules/` is not empty of notes. The fixture puts a
`README.md` in every one of its 3 840 packages precisely because that is true of
a real checkout: hiding the folder hides real Markdown the user might be looking
for, and a note-taking application that decides which of your notes are real is
making a decision that is not its to make. `.git/` is different in kind — it is
a database, and its contents are not documents — which is why it is in the
visibility list and `node_modules/` is not.

Watching is a different question with a different currency. An inotify watch is
a finite kernel resource, one per directory, and this machine's
`max_user_watches` is **65 536**. A single `~/x` overruns it, and the cost of
overrunning is not paid by `node_modules/` — it is paid by the user's actual
notes, which stop being watched because a dependency tree got there first. What
a skipped directory loses is *latency*, not correctness: a change inside one
still arrives through the five-second scan and the focus scan. Losing five
seconds of freshness inside `target/debug/deps` is not a loss.

**Alternative if you disagree.** Put both in `IGNORE_DEFAULT` and the two lists
collapse into one. It is one line, and it is reversible; what it costs is that
the user's `node_modules/*/README.md` stops existing as far as the application
is concerned, with no way to see it short of a setting. The reverse alternative —
watch everything, skip nothing — is also one line, and it costs the watch table
on any workspace with a build directory in it.

---

## D-09 — The tree in under a second, and everything that needs the whole tree in the background

**Decided.** See [ADR-034](decisions.md). `workspace_open` returns and the tree
appears in **under one second at any size**. Everything that has to walk the
whole tree — the watcher's per-directory watches, the quick-open path list —
runs on its own thread, is cancellable, and reports its progress to the status
bar. Recorded here with the measurements, because this decision came out of a
bug and the numbers are the argument.

**What was actually wrong.** The owner opened `~/x` — around 160 repositories,
each with `node_modules/`, `target/` and `.git/` — and the Welcome screen stayed
on screen for **more than two minutes** before the tree appeared, with a banner
saying `Permission denied (os error 13)` about one subdirectory. The obvious
suspicion was the tree, and the obvious suspicion was wrong.
`tools/gen-deep.sh` builds the same shape locally and
`crates/notes-core/tests/deep.rs` times each step of opening it:

```
directories:        20962
open_workspace:        0.70 ms
list root:             0.43 ms   (160 entries)
start_watch:         502.72 ms   (degraded: None)
quick_open first:    549.88 ms   (20 matches)
to a usable tree:      1.13 ms
everything:         1053.73 ms
```

The tree costs a millisecond. `start_watch` and `quick_open` cost half a second
**each**, and both of them cost it inside a `#[tauri::command]` that holds
`Mutex<WorkspaceService>` — so `tree_list` did not run slowly, it did not run at
all until they finished. At 21 000 directories that is a second; `~/x` has an
order of magnitude more, and it was minutes. The freeze was never a tree
problem. It was two whole-tree walks on the critical path, serialised behind one
mutex.

The same fixture found a second bug that had nothing to do with time. With its
mode-000 directory in place — the shape of the owner's `.../www/web1/ead` —
everything aborted in under a millisecond and `quick_open` returned
`Err(Io { op: "read_dir", kind: PermissionDenied })`. One unreadable
subdirectory made quick open return **nothing at all** for a workspace of
21 000 directories, and `notify`'s recursive add did the same thing to the
watcher: it fails whole on the first directory it cannot read, and the whole
workspace was then demoted to polling because of one folder.

**What changed.** Three things, and each is a rule rather than a patch:

1. **The walks moved off the command.** `watch()` watches the root
   synchronously — if even that fails there is nothing to watch — and spawns a
   thread that installs the rest one directory at a time. `PathIndex::start`
   does the same for quick open, and `quick_open` matches whatever exists so
   far, returning `building: true` beside it. A partial answer that says it is
   partial is a better answer than a frozen window.
2. **A directory that cannot be read is counted and skipped.** Never fatal,
   never a reason to demote the workspace. The count goes to the status bar.
   `notify`'s `RecursiveMode::Recursive` cannot express this, which is why the
   walk is ours.
3. **A full watch table degrades only the excess.** When `add_watches_below`
   hits the limit it stops asking, counts the remainder in `over_limit`, and
   keeps every watch already installed. The banner names the number and the
   `sysctl` that raises it. Previously the workspace went to polling entirely.

Symlinked directories are never descended into, in either walk — the fixture has
a loop, and it exists because a walk that follows one does not return.

**Alternative if you disagree.** Keep the walks synchronous and put a spinner on
the Welcome screen. It is honest about the wait and it needs no threads; what it
does not do is make the wait shorter, and on the owner's own folder the wait was
two minutes.

---

## D-10 — One watch per directory is Linux's problem, and Linux's alone

**Decided.** `PER_DIRECTORY` in `notes-fs::watch` is `cfg!(target_os = "linux")`.
On Linux the root is watched non-recursively and a background thread installs
the rest, one directory at a time. On macOS and Windows the root is watched
**recursively, in one call**, and there is no walk.

**Why this is not one implementation for all three.** D-09 moved the walk off
the critical path because `notify`'s recursive add is a synchronous per-directory
walk. That is true of inotify, where a watch descriptor covers exactly one
directory and there is no other way. It is **not** true of the other two:
FSEvents watches a subtree from one handle, and `ReadDirectoryChangesW` takes a
`bWatchSubtree` flag. Running our walk there would have replaced an O(1) call
with 20 000 kernel objects on the deep fixture, and hundreds of thousands on the
folder that started this — the same mistake as the freeze, pointing the other
way, and it would have been introduced *by the fix for it*.

The two hazards the walk exists to handle are also Linux's. A recursive add that
fails whole on an unreadable directory is inotify's behaviour, because it is
inotify that has to enumerate; a subtree watch never reads the tree, so it has
nothing to fail on. `max_user_watches` is an inotify sysctl.

`WATCH_SKIP` follows the same line: it exists to protect a finite watch table, so
it applies only where there is one. A subtree watch covers `node_modules/`
whether or not anyone wants it to, and those events are filtered by the
reconciler like any other.

**Alternative if you disagree.** One code path everywhere. It is simpler to read
and it is what the first version of this change did; it costs a directory handle
per directory on Windows, which on a real checkout-heavy folder is a resource
failure rather than a slow start.

**What is not verified.** This machine is Linux. The macOS and Windows branch is
one `RecursiveMode::Recursive` call — the same one that shipped before 0.11.0 —
and CI runs the suite on both, but nobody has watched a subtree event arrive on
either.
