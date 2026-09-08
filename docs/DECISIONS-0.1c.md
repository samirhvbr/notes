# Decisions taken while building 0.1c

> **Status:** `ACTIVE` · Calls made under the standing rule — *choose the
> simplest option compatible with the scope, write the missing section into
> `ARCHITECTURE.md`, and carry on*. One row per decision: what was decided,
> which gap it closed, and **what the alternative is** if the owner disagrees.
>
> The structural ones are promoted to [`decisions.md`](decisions.md) as
> **ADR-030 … ADR-032**; the rest are here so that none of them is invisible.

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
