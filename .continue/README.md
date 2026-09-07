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
| Product shape of the desktop Markdown editor — what v1 does and does not do | Under discussion, nothing decided | Samir |
| Desktop stack choice | Blocked on the shape above | Samir |
| How an optional public Git repository is linked to the app | Under discussion | Samir |

## 2. Where things went

<!-- When a document leaves for ../docs/, give it a row with a relative link.
     This is what keeps "it misleads whoever opens it" from becoming "nobody
     can find it". -->

| It was here | It is now at |
|---|---|
| _(nothing yet)_ | |

## 3. Pending decisions

| Decision | Whose | Note |
|---|---|---|
| Desktop stack — Tauri, Electron, or native | Samir | Becomes ADR-001 with the reason, not just the name |
| Storage model — plain files on disk vs. an app-managed library | Samir | Decides whether the Git link is a feature or the storage layer |
| Git linking — what "linkable to a public repo" means operationally | Samir | Clone-and-commit locally, or API-only; also what happens with no repo at all |
| What a `Z` bump means in this project | Samir | The slots in `CLAUDE.md` stay as examples until this is answered |
