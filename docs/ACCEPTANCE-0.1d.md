# Acceptance — milestone 0.1d (Interface)

> **Status:** `ACTIVE` · The milestone is `.continue/0.1d-interface.md`; this is
> its acceptance, and **almost all of it is a person's**. 0.1d produced very
> little core code and a great deal of frontend, at a point where this project's
> testing strength is in the core — so the automated half is narrow and honest
> about being narrow, and everything else has a box with nobody's tick in it.
>
> **Nothing here is ticked, and a box is not ticked by whoever built it.** The
> rule the owner set for this milestone and every one after: a flow becomes
> `verified` when they have walked it **in an installed build** and then
> **repeated it on the following release**. A flow that worked once on a machine
> that had just compiled it is a smoke test with a good mood.

---

## 1. The interface, area by area

The ten rows of `.continue/0.1d-interface.md` §7, in the order the eye meets
them.

| # | Area | Expected | Verified |
|---|---|---|---|
| I1 | **The rail** | Files and Search each show their panel; clicking the icon of the panel already open collapses the sidebar; Settings opens the panel; **Graph is visibly disabled and its tooltip says 0.3** | ☐ |
| I2 | **Explorer toolbar** | New note creates *and opens* it; new folder appears; sort flips A→Z / Z→A and directories stay first; collapse-all folds every open directory | ☐ |
| I3 | **Workspace selector** | The footer shows the current workspace; the menu opens with *Open folder…*, *Create workspace…*, recents and *Close workspace* — and each of the four does what it says | ☐ |
| I4 | **Workspace selector, with a dirty buffer** | *Close* and switching both ask, **in the application's own modal**, and name the note; declining leaves the workspace exactly as it was | ☐ |
| I5 | **Tabs** | The active tab is distinguishable **by background**, not by weight; the dirty dot appears; `×` closes; `+` makes a note; the split button toggles | ☐ |
| I6 | **Note header** | Back and forward move through the notes visited, and grey out at the ends; the title is the file name without `.md`; the toggle swaps Source and Preview; `⋮` opens | ☐ |
| I7 | **The column** | The text sits in a centred column with margins that grow with the window; switching Source ↔ Preview does **not** move the text under your eye | ☐ |
| I8 | **The split divider** | Dragging resizes; double click and `Enter` even it up; **`Tab` reaches it and the arrows move it** | ☐ |
| I9 | **Status bar** | The state, the words and the characters — **and nothing else**. No backlink count | ☐ |
| I10 | **Focus** | Every menu and modal returns focus to the control that opened it; the focus ring is visible on every control, on every surface | ☐ |

### The test that cannot be automated

> *"Alguém que usa Obsidian todo dia abre o app e encontra tudo sem pensar. Se
> precisar procurar onde troca de pasta, o marco não fechou."*

☐ — and this one is the owner's alone.

---

## 2. Automated

What a machine can hold, and it is less than a fifth of the above.

| What | Where | Holds |
|---|---|---|
| Menu keyboard navigation, with focus tracked | `src/app/Menu.test.tsx`, 11 tests, in a DOM | Arrows wrap; a disabled item is never landed on; `Home`/`End`; `Enter` runs and closes; **`Escape` and `Tab` return focus to the trigger**; a click outside closes; the action runs *after* the menu is gone |
| Contrast and the palette | `tools/contrast.sh`, in `check.sh` and CI | 42 pairs: AA for every text/surface pair, AA for the focus ring and for disabled controls, an 8/255 sRGB step between the three dark levels — **and a build failure if any colour is written outside `:root`** |
| No blocking dialogs | `tools/no-blocking-dialogs.sh` | A browser script dialog anywhere in the frontend fails the build. Six flows of 0.1b were behind one |
| Switching workspace | `notes-core`, `tests/switch.rs`, 6 tests | Identity survives a switch and a restart; a dirty close is refused **and names the notes**; a clean close leaves no workspace open; every workspace opened is offered again |
| Everything the milestone inherits | `tools/check.sh` | Format, clippy on the native and the Windows target, the whole Rust suite, byte preservation, the full-disk suite, the generated TypeScript, the i18n catalogues, the frontend build and its 60 tests |

**Three things the automated half deliberately does not claim.** It does not
know whether the interface *looks* right; it does not know whether the tab bar
is legible on the owner's screen; and it cannot walk a flow. `tools/contrast.sh`
proves a ratio, not a design.

---

## 3. The twenty-five flows, re-indexed

`ACCEPTANCE-0.1b.md`'s U1–U12 and `ACCEPTANCE-0.1c.md`'s C1–C13, with **the
steps rewritten for the interface they now live in**. The behaviour is
unchanged and the expectations are the originals, word for word where they
still fit — what moved is where you press.

They stay ☐ in all three documents until the walk on the `.deb`. A flow whose
steps describe a window that no longer exists cannot be walked, which is why
they were re-indexed rather than ticked where they were (ADR-037).

### From 0.1b — entry operations, conflicts, preview

| # | Was | Now, in the 0.1d interface | Expected |
|---|---|---|---|
| U1 | *New note* in the top bar | **Explorer toolbar → the file-plus icon**, or `+` on the tab bar, or `Ctrl+N` | The modal appears, the note is created **and opens**; `Cancel` and `Escape` each leave nothing behind |
| U2 | *New note* → empty name | Same, empty field → **Create** | Refused **in the dialog**, with "A name is required."; the core is never called |
| U3 | *New folder* in the top bar | **Explorer toolbar → the folder-plus icon** | The folder appears in the tree |
| U4 | Welcome → *Create Workspace…* | **Sidebar footer → the workspace name → *Create workspace…*** — and still on Welcome, for the first one | Native picker for the parent, the application's own modal for the name |
| U5 | Right-click a note → *Rename…* | Right-click the row **or its `⋮`** → *Rename* | The tab keeps its cursor and identity; the sidebar shows the new name |
| U6 | Right-click → *Move to…* | Right-click the row **or its `⋮`** → *Move* | The note moves; an **empty field means the workspace root** and must not be read as a cancellation |
| U7 | Right-click → *Delete…* | Right-click the row **or its `⋮`** → *Delete* (last, and marked destructive) | A **destructive** confirm; on cancel nothing happens; on confirm the status line says *trashed* or *permanent* and which |
| U8 | `Escape` on any of the above | Unchanged — **and now also on every menu**, which is what `Menu.test.tsx` asserts and this confirms on screen | Cancels, and focus returns to the control that opened it |
| U9 | *Source* / *Preview* / *Split* buttons in the top bar | **The note header's toggle** for Source ↔ Preview, **the tab bar's split button** for split; `Ctrl+E` still cycles all three | Three modes; `Ctrl+E` cycles |
| U10 | Edit in another editor while dirty | Unchanged | Conflict banner, autosave suspended, and the **compare screen** shows both versions |
| U11 | Resolve the conflict three ways | Unchanged | *Keep mine*, *use the disk*, *save as a copy* — the version not chosen lands in `conflicts/` |
| U12 | `Ctrl+F` inside the note | Unchanged | Matches highlighted, replace applies, `Escape` closes |

### From 0.1c — navigation, search, tabs, settings

| # | Was | Now, in the 0.1d interface | Expected |
|---|---|---|---|
| C1 | `Ctrl+P` | Unchanged | The palette lists matching notes, best match first; `Enter` opens it |
| C2 | `Ctrl+P` → no match | Unchanged | Says so; does not close, does not open anything |
| C3 | `Ctrl+P` after creating a note | Unchanged | The new note is offered without restarting the app |
| C4 | `Ctrl+Shift+F` → search a word | **The rail's Search icon**, or `Ctrl+Shift+F` — which now *shows* the panel and never toggles it shut | Results stream in, with path, line and the matching line |
| C5 | Search with unsaved changes | Same, in the sidebar | The panel **says results come from what is on disk** |
| C6 | Cancel mid-search | Same, in the sidebar | Stops; partial results stay on screen and say they are partial |
| C7 | Switch literal ↔ regex | Same, in the sidebar | The mode is named on screen and does not change underneath a running query |
| C8 | Click a result | Same, in the sidebar | Opens that note **at that line** |
| C9 | Open three notes | Unchanged — **and the active tab now has a background**, not just a weight | Three tabs; the active one is marked; `Ctrl+W` closes one |
| C10 | Quit and reopen | Unchanged | Workspace, tabs, **active tab and cursor position** all come back |
| C11 | `Ctrl+Shift+P` | Unchanged | Lists commands; `Enter` runs one; `Escape` closes |
| C12 | Settings → the four | **The rail's Settings icon**, or `Ctrl+,` | Each applies to the editor and survives a restart |
| C13 | Switch the language | Same panel | Every visible string changes; no key is left showing raw |

### And one the interface added to the list

| # | Flow | Expected | Verified |
|---|---|---|---|
| C14 | Settings → **Diagnostics** | Platform and the dmabuf decision, which used to be in the top bar and is now where a thing you look up belongs | ☐ |

---

## 4. Out of this milestone, and not to be found in it

`.continue/0.1d-interface.md` §8, restated because a milestone's boundary is
part of its acceptance: **graph view** (0.3), **backlinks and their counter**
(0.3), a properties panel (0.3), themes, plugins, Live Preview (§18), and
anything from 0.2.

The graph icon in the rail is the one place any of this is visible, and it is
`disabled` with `0.3` in its tooltip. If it ever becomes clickable in this
milestone, that is a defect and not a bonus.

---

## 5. What this milestone changed about the core

Almost nothing, on purpose (`.continue/0.1d-interface.md` §5). Two things are
worth naming:

- **`tests/switch.rs`** — six tests over a path that was unreachable until the
  workspace selector existed. They pin behaviour, not a fix: a store-before-adopt
  was written into `open_workspace` and removed again when it turned out the
  tests passed without it (`DECISIONS-0.1d.md` D-01).
- **Nothing else.** Where the interface met a gap in the core it was written
  into `DECISIONS-0.1d.md` rather than fixed on the way past. D-01 is the one
  that matters: `ARCHITECTURE.md` §4.1 describes a registry debounce that is not
  implemented, and the day it is, `open_workspace` becomes a loss path.
