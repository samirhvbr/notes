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
- **When an item leaves, what "produce" means, and why length is not an exit
  condition: the `QUEUE-RULE` block in [`../CLAUDE.md`](../CLAUDE.md).** It is
  regenerated from the fleet standard and is the source — it is not restated
  here, and it should not be. It was a local rule in this repository for one
  version ([ADR-009](../docs/decisions.md#adr-009--an-item-leaves-continue-only-when-it-has-been-built),
  [ADR-010](../docs/decisions.md#adr-010--continue-is-written-in-portuguese-everything-else-is-english));
  the fleet adopted both, and the ADRs stay as the record of where the decision
  was made.
- **This README is the one file here that is not queue material.** It is the
  folder's index, so it stays in English while the items around it are written
  in the language their author thinks in.
- **Nothing here is source of truth about what exists.** What already exists is
  described in [`../docs/`](../docs/); the permanent record of *when* is
  [`../CHANGELOG.md`](../CHANGELOG.md).
- **In a contradiction, an `ACTIVE` document in `../docs/` wins — a `PROPOSED`
  one does not.** A `PROPOSED` document describes something that has not been
  built, so this folder is the authority on intent for as long as both exist.
- The **Continue** IDE also uses this folder for its own configuration.

## 1. What is left here

<!-- One line per item. Delete a row when the item is done — do not tick it. -->

| Item | State | Who unblocks it |
|---|---|---|
| [`SCOPE_final.md`](SCOPE_final.md) — **the specification to build**, v2.0 | Written, not built. Supersedes the two v1 drafts below | — |
| **Milestone 0.0 — the spike** | **Open.** The application builds, is tested and lints clean; what it establishes and what it does not is [`../docs/SPIKE-0.0.md`](../docs/SPIKE-0.0.md). It stays here until the §2 checklist is marked on Arch/Wayland/NVIDIA, an iPhone and an Android device — none of which exist on the machine that wrote it | Samir |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) — my §20 proposal | **Superseded** by [`../ARCHITECTURE.md`](../ARCHITECTURE.md), Samir's v2.0-aligned document at the repository root. Kept, not deleted: it is where the questions were asked. Three of its §5 questions are answered there; the bundle identifier is answered only for the spike | — |
| [`scope.md`](scope.md) · [`scope.md — Aplicativo Markdown Local-First.md`](scope.md%20%E2%80%94%20Aplicativo%20Markdown%20Local-First.md) — the v1 drafts | Superseded by `SCOPE_final.md`; kept until the work they describe exists | — |
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
