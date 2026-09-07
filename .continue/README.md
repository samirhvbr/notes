# `.continue/` — the queue

> **Status:** `ACTIVE` · Last reviewed 07/09/2026, repository at 0.1.0

Work in progress: drafts, plans under discussion, notes on things still being
built, briefings for picking the work back up later.

**The norm this folder obeys lives once, in the fleet standard:**
[samirhvbr/repodocs `docs/conventions.md`](https://github.com/samirhvbr/repodocs/blob/master/docs/conventions.md)
— §1 (queue vs. record) and §2 (this README is mandatory). Read it there; a copy
here would be a second source of truth.

## How it works

- **Git-tracked on purpose — deliberately NOT in `.gitignore`.** Opening the
  project on another machine brings the context along, which is the whole point:
  you can *continue* from where you stopped.
- **Nothing here is source of truth.** The moment a document describes something
  that already exists, it moves to [`../docs/`](../docs/) and the permanent
  record of *when* is [`../CHANGELOG.md`](../CHANGELOG.md).
- **A finished item leaves.** It is deleted here, not ticked off — the permanent
  record of completion is the changelog. A queue holding an already-done item
  costs more than an incomplete queue: it makes the next session redo work.
- **If an item needs half a page, it is in the wrong place.** Write it in
  `../docs/` and leave one line and a pointer here.
- **In a contradiction between this folder and a document in `../docs/`, the
  document wins.**
- The **Continue** IDE also uses this folder for its own configuration.

## 1. What is left here

<!-- One line per item. Delete a row when the item is done — do not tick it. -->

| Item | State | Who unblocks it |
|---|---|---|
| [`scope.md`](scope.md) · [`scope.md — Aplicativo Markdown Local-First.md`](scope.md%20%E2%80%94%20Aplicativo%20Markdown%20Local-First.md) — the full product spec | **Written, not built.** Nothing in it exists as code yet, so it stays here | — |
| Scaffold milestone 0.1 — the Cargo workspace, `apps/notes-app/`, the first crates | Not started | — |
| Cut the concrete boundaries between `notes-core`, `notes-fs` and `notes-index` | Deferred to the first code that needs them, on purpose — see [ADR-003](../docs/decisions.md) | — |
| Dependabot PR #1 (`actions/checkout` 5 → 7) | Open on GitHub | Samir |
| Report the `CHANGELOG.md` header defect back to the repodocs skeleton | Found while bootstrapping this repo: the skeleton describes the commit format in Portuguese while repodocs' own root file says English. Fixed here at `0.1.0`; still ships to every new repository | Samir |

## 2. Where things went

<!-- When a document leaves for ../docs/, give it a row with a relative link.
     This is what keeps "it misleads whoever opens it" from becoming "nobody
     can find it". -->

| It was here | It is now at |
|---|---|
| _(nothing yet — nothing here has been built)_ | |

## 3. Pending decisions

| Decision | Whose | Note |
|---|---|---|
| _(nothing open)_ | | The four decisions that were here — stack, storage model, Git, and what a `Z` means — were all answered at `0.2.0`. They are ADR-002, ADR-001, ADR-006 and [../docs/versioning.md](../docs/versioning.md) respectively. Do not re-open one here; reverse it with a new ADR |
