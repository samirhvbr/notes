# Changelog

Entries in the commit-message format (`version - short description in English`, see
[docs/versioning.md](docs/versioning.md)), newest first. **Each `##` heading is
literally the commit subject** — this file is the handoff artefact between
whoever does the work and whoever commits it.

Bodies are narrative: what changed, why, and what was measured. This file is
never rewritten.

## 0.20.2 - preserve disk-full classification through contextual IO errors

Classify StorageFull and QuotaExceeded even when a library adds path context
and removes the raw OS error code. The Linux ENOSPC gate caught this when
atomic note creation used tempfile: the write failed safely but was reported
as generic IO. Cover both contextual error kinds and retain the real full-disk
regression, with a diagnostic that prints any unexpected result.

## 0.20.1 - resume guarded application from durable client checkpoints

Add an explicit receive-only apply command with a pinned app data directory,
per-revision local receipts and recoverable write intent. Refuse local edits,
preexisting destinations, incompatible state and unsupported renames/deletions
without discarding received bytes. Exercise real transfer/application and local
conflict refusal, plus a lost application receipt without a second source write.
Document the closed-workspace boundary and keep active-editor integration and
server device acknowledgments in the queue. The full local gate passed with
Rust tests serialized after the unchanged watcher timing test exceeded its
100 ms limit under parallel macOS load; native and Windows clippy also passed.

## 0.20.1 - apply received content through guarded core writes

Apply received creations and same-path updates only while the workspace is
closed in cooperating core processes. Preserve drafts and reject stale local
revisions, destination collisions and unsupported operations. Persist intent
before source writes so interrupted application can resume without rewriting
newer local content. Keep application receipts distinct from transfer receipts.

## 0.20.0 - refresh the remaining milestone queue

Record the shipped domain, server inbox and device transfer client separately
from guarded source application, conflict handling, broader pairing, deletions,
attachments, scheduling, UI and retention. Keep mobile implementation and owner
installed-release acceptance visible, with remote MCP following sync. Correct
the scope index to reflect partial implementation without removing open work.

## 0.20.0 - ship the verified device transfer client

Package the standalone client and exercise two real processes through the native
server and the CI HTTPS proxy. Verify offline restart, untrusted certificate
refusal, explicit test-CA trust and exact received-byte export while preserving
source folders. Include the client in cross-target checks and build transport
binaries explicitly before smoke tests.

## 0.20.0 - resume device revision transfers from durable queues

Add an explicit sync client that captures saved source bytes through core,
queues immutable publications offline and retries unchanged UUIDs after restart.
Pin the selected server and workspace, verify bounded responses and persist
received content before advancing its cursor. Keep conflicts and failures in
the local queue, never acknowledge source application or modify dirty notes.
Document the remaining sync, mobile, MCP and installed-release acceptance queue.

## 0.19.1 - expose immutable revision transfer over authenticated HTTP

Expose incremental metadata pages, original revision fetches and idempotent
conditional publication through the existing HTTPS and bearer boundary. Reuse
workspace permissions, protect entire historical paths and report storage
separately from source application. HTTP and TCP tests cover retries, stale
writes, concurrency, quota refusal, review scope and backup recovery. The device
outbox, workspace application and sync UI remain open.

## 0.19.1 - persist scoped sync revision inboxes

Persist original revision bytes and causal heads in one bounded, atomic server
transaction, so a stored revision cannot refer to missing content. Validate
history and hashes on reopen, retain tombstones until explicit capacity refusal,
and include inbox data in offline backups while excluding its process lock.
Future or corrupt state is refused without replacement.

## 0.19.0 - preview pairing through core inventories

Expose a standalone notes-sync-plan command over bounded core inventories of
two mounted folders. Distinguish upload, download and reconciliation; preserve
raw bytes, existing identities and note visits while reporting links, unique
notes and conflicts. Reject state directories inside source folders before
creating anything. Package the preview separately and keep the unfinished
transport, outbox, content application and UI explicitly in the sync queue.

## 0.19.0 - model causal revisions for synchronization

Start milestone 0.6 with a separate notes-sync domain crate. Track immutable
revisions, parentage, renames, tombstones and device acknowledgments without
using modification time to elect a winner. Produce deterministic incremental
plans that preserve conflicting edits and refuse path collisions, stale
resolutions and unrelated histories. Persist schema-versioned metadata through
locked, atomic compare-and-set transactions that preserve invalid/future state.

## 0.18.1 - exclude process locks from portable server backups

Windows enforces locked byte ranges even when the lock file contains no data,
so archiving a live backup guard failed before the archive could be published.
Keep the guards held and omit only regenerable operational lock files; source
files, credentials, identities and SQLite state remain in the backup. Exercise
the documented offline container backup/restore path as well as HTTPS, using
an isolated one-off container without a network or competing static address.

## 0.18.0 - package the self-hosted server with HTTPS operations

Provide a non-root container, a Compose deployment behind Caddy HTTPS, a
versioned OpenAPI contract and operator backup/restore instructions. Add real
TCP and container TLS acceptance alongside the existing workspace gate. The
server remains opt-in; desktop sync and remote MCP remain later milestones.

## 0.18.0 - serve scoped notes through a conditional REST API

Add the independent notes-server executable for one owner and per-integration
credentials. Reuse core permissions, root confinement, identity locks and
durable append receipts for path-addressed CRUD and search. Enforce bounded
bodies, pagination, request rates and concurrency; preserve source formatting
and reject stale complete revisions. Persist revocable credential digests and
bounded authorship events without tokens, note paths or content. Offline
backups include source bytes and operational state, refuse a live server and
restore only into a new directory. Future state schemas are refused unchanged.

## 0.17.0 - prepare the filesystem core for mobile targets

Merge the reviewed mobile foundation after the completed local knowledge work.
Make the trash dependency and capability desktop-only while preserving the
0.3 guarded-write changes. Reconcile the PR's obsolete version and ADR numbers,
state that generated mobile projects and layouts are still pending, and add
an iOS simulator core check to CI. This integrates the foundation of milestone
0.4; it does not claim a usable mobile application or device acceptance.

## 0.16.1 - preserve indented separators in YAML properties

Only an unindented Markdown metadata fence ends front matter. An indented
`---` or `...` inside a YAML block scalar is content; trimming its indentation
silently truncated properties and could hide tags after the scalar. Keep the
original source unchanged and cover both separators and trailing tags in a
regression test. Properties update immediately; upgrading from 0.16.0 requires
Rebuild index for affected cached tags. Publish refreshed packages for this
milestone correction.

## 0.16.0 - expose knowledge navigation and package local MCP

Add Properties, Tags and Backlinks panels, wiki destination selection, an
accessible graph with usable node targets, and safe asynchronous clipboard
image insertion. Publish the standalone Linux MCP archive alongside app
packages. The complete local gate, six real MCP process tests and 72 frontend
tests pass; native debug interaction verified metadata, ambiguity, graph
navigation and backlinks. Installed Linux owner acceptance and its repeat
remain explicitly pending. Keep the independently developed mobile PR intact.

## 0.16.0 - build local knowledge and scoped agent operations

Implement the milestone 0.3 core: read-only YAML and tags, wiki resolution,
backlinks/graph data, reviewed wiki renames, validated clipboard imports and
standalone scoped stdio MCP. Rebuild derived schema 1 documents for the new
parser while retaining operational identity. Cross-process tests found and
now prevent overwrites hidden by equal size/mtime; shared enrollment and
identity locking plus durable append receipts cover concurrent starts and
retries. Version 0.15.0 remains reserved by the independent mobile PR; its merge
must reconcile version and ADR numbering. Owner acceptance remains pending.

## 0.14.1 - isolate empty-file content correlation from inode reuse

Arch CI exposed a fixture that deleted the original empty file before creating
its replacement. A reused inode legitimately entered the native-identity rule,
so the test did not isolate the content-correlation behavior named in its title.
Create both files before removing the original, and additionally assert that
opening the replacement gives it a distinct NoteId. Product behavior is unchanged.
The complete local gate is rerun; installed packages remain the 0.14.0 delivery.

## 0.14.0 - complete the desktop navigation workflow

Expose Files/Recent/Outline, named Words/Literal/Regex modes, index progress,
cancellation/rebuild, and reference review before rename/move. The review lets
users select affected files and cancel without writes; an unavailable index
requires an explicit choice to move without updating links.

Finish the 0.1d implementation pass: mount the Welcome creation dialog, trap
and restore Settings/palette focus, refresh Quick Open while its cache builds,
clean up a divider unmounted mid-drag, cancel late search-start responses, and
prevent reloads from replacing newer typing. English/Portuguese catalogues and
IPC types match. All 69 frontend tests and the full local gate pass. A debug
macOS UI smoke test exercised Outline, Recent and a Words hit at the correct
line. The installed-release owner checks remain unticked in both acceptance
documents; 0.3 is still proposed.

## 0.14.0 - implement the milestone 0.2 core

Add separate operational registry.db and derived index.db, legacy identity
migration with a retained backup, incremental/cancellable FTS5 indexing, recent
history, and guarded incoming/outgoing Markdown reference rewrites. Rewrites
keep original bytes, disclose skipped candidates and report per-file failure.
Registry transactions merge unrelated stale snapshots and refuse conflicts;
newer database schemas are never downgraded. Literal and Regex keep their
existing semantics. Image destinations enter the shared document IR; the golden
fixture changes only metadata, not rendered HTML.

The full local gate passes, including native/Windows clippy, workspace tests,
byte preservation and generated contracts; ENOSPC is Linux-only. The read-only
~/x debug benchmark scanned 6,707 notes: 78.6 seconds initially and 3.7 seconds
with zero unchanged files reprocessed on the second pass. Owner acceptance on
installed Linux packages and the following release remains pending.

## 0.13.5 - preserve focus across menu actions

Choosing a menu item removed the focused button without restoring focus, so
ordinary actions left the keyboard on the document body. Dialogs launched by a
menu also captured a disappearing return target and lost focus on cancellation.
The menu now restores its trigger synchronously before running the action.
An action can then focus its own destination without a delayed restoration
stealing focus back.

Three DOM regression cases cover ordinary selection, launching and cancelling
a dialog, and an action that focuses another control. The first two failed
before the fix. All 14 menu tests and `tools/check.sh` passed, including the
frontend suite and production build; ENOSPC skipped because this machine is
macOS. Manual installed-build acceptance remains pending.

## 0.13.4 - show the repository version in development builds

The macOS About window displayed 0.0.0 during interface acceptance because the
development CLI read the committed packaging placeholder. The npm Tauri entry
now supplies the first version from version.md through an in-memory CLI config
override for desktop and mobile dev commands. Build commands retain ADR-035's
explicit release stamping, and tauri.conf.json stays unchanged.

Five launcher tests cover version extraction, mobile commands, application
argument separation, build passthrough, and refusal of a missing version. They
run in the local gate and frontend CI. The real CLI version/help commands and
`tools/check.sh` passed; ENOSPC skipped on macOS. The existing native window
was not restarted, so visual confirmation of About remains pending.

## 0.13.3 - preserve keyboard intent inside modal dialogs

Pressing Enter on Cancel confirmed a destructive request because the modal's
parent intercepted Enter before the button could activate. Confirmation now
uses each button's native keyboard behavior. Tab and Shift+Tab wrap inside the
modal, and separate input and button refs prevent the confirmation button from
receiving an unsupported `select()` call when it opens. Modal keystrokes no
longer reach background application shortcuts.

Six DOM regression tests cover cancellation, confirmation, focus containment,
text selection, Escape with focus restoration, text submission, and shortcut
isolation. Before the
fix, three failed and confirmation dialogs raised three uncaught exceptions.
A sixth test also reproduced shortcut propagation before its fix. The manual
installed-build acceptance remains pending. `tools/check.sh` passed; its ENOSPC
check skipped because this machine is macOS. The final frontend suite passed
all 55 tests and the production build passed.

## 0.13.2 - disambiguate the dialog component on case-insensitive filesystems

A fresh macOS clone passed all 49 frontend tests but failed the production
build: TypeScript resolved the extensionless `app/Dialog` import against
`app/dialog.ts`, then reported TS2305 and TS1149. The modal component now lives
in `DialogHost.tsx`, separate from the dialog state module even when filename
case is ignored. Its exported component and behavior are unchanged.

The existing TypeScript build is the regression check on macOS; a new unit
test would not exercise filesystem module resolution. `tools/check.sh` passed,
including native and Windows clippy, the Rust suite, byte preservation, and
frontend tests and build. The ENOSPC script skipped on macOS because it needs
Linux. The development app started; interface acceptance remains pending and
this fix does not begin milestone 0.2.

## 0.13.1 - the queue says where 0.1d and 0.2 stand

Two rows in `.continue/README.md`, which is the folder's index and had neither.

`0.1d-interface.md` **stays in the queue** even though the interface is on
screen as of `0.13.0`. Its own header says it leaves when the interface exists
there — but the rule the owner set for this milestone is stricter than the one
the file was written under: a box is ticked after they have walked it on the
installed `.deb` **and repeated it on the release after**. Until then the
milestone is built and unverified, which is a state the queue can hold and a
tick cannot.

`0.2-indice.md` records where the index milestone stopped. The code written for
it — the `notes-index` crate, the SQLite plumbing with WAL and a migration
ladder, five green tests — was **discarded** rather than left on a branch,
because scope §5 says a crate exists only once the milestone that uses it
begins and 0.2 has not begun. What is kept is the part that was expensive: the
crate cut ADR-003 deferred, the argument for `notes-index` touching no
filesystem, and the two-phase `plan`/`apply` protocol that follows from it.

## 0.13.0 - milestone 0.1d ships: the interface, with twenty-six boxes nobody has ticked

The minor the milestone asks for, so `build.yml` produces a `.deb`, an AppImage,
a tarball and the AUR package (ADR-036). `.continue/0.1d-interface.md` §9: the
owner installs it and walks everything at once — the ten areas of this milestone
and the twenty-five flows of 0.1b and 0.1c, re-indexed into the interface that
now exists.

**What is on screen that was not before**

| | |
|---|---|
| A rail | Files, Search, Graph (disabled, tooltip `0.3`), Settings at the foot. The active icon collapses the sidebar |
| A sidebar | Explorer toolbar — new note, new folder, sort, collapse all — the tree, and **the workspace selector** pinned below the scroll |
| A tab bar | The 0.1c tabs, with a background on the active one, `+`, and the split toggle |
| A note header | Back/forward, the title without `.md`, Source ↔ Preview, `⋮` |
| A column | 700 px, centred, the same on both sides of a split, with the same font and rhythm |
| A divider | Draggable, **and focusable, and arrow-movable** |
| A status bar | The seven states, words, characters — and nothing else |

**And the defect that started it.** Every command needed to change workspace has
existed since 0.1a; the only surface reaching them was the Welcome screen, which
disappears the moment a folder is opened. After the first open there was no way
to change folder at all.

**What was decided, and where.** Two ADRs for the scope change — 0.1d exists
(ADR-037), graph view leaves §18 for 0.3 after backlinks (ADR-038) — and seven
calls in `DECISIONS-0.1d.md`, including the three questions the milestone left
open by name: the column is a fixed maximum rather than a fifth setting, split
is horizontal only, and Welcome stays a screen.

**What the machine holds, and what it does not.** Eleven DOM tests on the menu's
keyboard path, six core tests on switching workspace, and a contrast script over
42 pairs that fails the build if any colour is written outside `:root`. That is
less than a fifth of `ACCEPTANCE-0.1d.md`, and the document says so. The rest is
the owner's, twice — once on this `.deb` and once on the next one.

The one criterion no test will ever hold is written down too: *"Alguém que usa
Obsidian todo dia abre o app e encontra tudo sem pensar. Se precisar procurar
onde troca de pasta, o marco não fechou."*

## 0.12.5 - the editor is set in the body font, because the reading was jumping

Two things the milestone's own acceptance asks for that the build did not do,
found by launching it and looking.

**Source and Preview did not share a typography.** §4.3: *"Preview: mesma
coluna, mesma tipografia — a leitura não deve 'pular' ao alternar."* The
preview was in the interface sans and the editor was entirely in JetBrains
Mono, so switching between them moved every line — two fonts at one size do not
occupy the same space, and the column that 0.12.3 carefully matched was the
only thing that did.

§5 is explicit about which way to resolve it: *"uma sans para interface e
**corpo**, uma mono para código."* A note is body text and the editor is where
it is written, so the editor is now set in `--font-body` and **code keeps the
mono** — inline spans and fenced blocks, through the highlight style. The H1
that was a large monospace heading now looks like a heading.

Both stacks became tokens, which also removed the last three places a font was
written out by hand. Nothing is downloaded: the CSP forbids a remote font, and
an application that needs the network to look right is not local-first.

**The collapse-all icon read as a close button.** `ChevronsDownUp` at 15 px is
two chevrons pointing at each other, which is an ✕ to anyone not looking for
it — in a toolbar, beside a tree, that is an invitation to lose your expansion
state on purpose. It is `ListCollapse` now.

Both were found the same way: building the `.deb`, launching it, and taking a
picture of the window. Synthetic clicks still do nothing on this machine — the
window manager refuses to raise the window and WebKit ignores events delivered
to an unfocused one, which is the same wall `ACCEPTANCE-0.1b.md` recorded — so
the session was **seeded through the core** instead, and the shell photographed
with a real note open in split.

## 0.12.4 - polish, and ACCEPTANCE-0.1d.md with the twenty-five flows re-indexed

Steps 4 and 5 of 0.1d.

**Polish.** One focus ring for every control, on `:focus-visible` so a mouse
click leaves nothing behind and a `Tab` always does. Colour transitions at
90 ms — short enough to read as a response rather than an animation, and only
on colour, because nothing that moves the layout should be animated under a
click that is on its way. `prefers-reduced-motion` turns all of it off: that is
a preference the operating system already knows and the application has no
business second-guessing. Scrollbars joined the palette. The Welcome screen —
the first thing anyone sees and the piece with the least attention — got the
same tokens and rhythm as the shell it leads into.

The selected tree row moved from `color-mix(accent)` to `--selected`, because
`color-mix()` resolves against whatever it lands on and its result is not a
token the contrast script can read — and a selected row is the surface most
likely to be carrying dim text.

And the script gained the pair it was missing: **the accent is also a
surface.** The primary button paints a label on it, and the accent had only
ever been checked as a foreground. A colour is not safe because one of its two
roles is.

**`ACCEPTANCE-0.1d.md`.** Ten areas, none ticked, plus the one that cannot be
automated — *"alguém que usa Obsidian todo dia abre o app e encontra tudo sem
pensar"* — and an automated section that says plainly it holds less than a
fifth of the rest.

The **twenty-five flows of 0.1b and 0.1c are re-indexed into it**, each with a
*was* and a *now*: `New note` moved from the top bar to the explorer toolbar,
the entry menu is reachable by `⋮` as well as right-click, search moved into
the sidebar, Settings is on the rail. The expectations are the originals word
for word; only where you press changed. Both older documents now point at the
new one and say why, because a flow whose steps describe a window that no
longer exists cannot be walked — which is what ADR-037 said when it re-indexed
them instead of ticking them where they were.

They stay ☐ in all three until the walk on the `.deb`, and then until the
walk on the one after it.

## 0.12.3 - the column, the type, and a divider you can move with the keyboard

Step 3 of 0.1d (`.continue/0.1d-interface.md` §6, §4.3, §5).

**The column.** Both panes now lay their content out in a centred column of
`--column` (700 px, D-03) with margins that grow with the window. In the editor
it is on `.cm-content` rather than on the scroller, and in the preview on the
children rather than on the container — in both cases so the **scrollbar stays
at the pane's edge** instead of sliding in to the column's. Line height, top
padding and heading scale are the same on both sides, which is what §4.3 means
by the reading not jumping when you switch.

Both also carry 40vh of bottom padding, so the last line of a note can be
scrolled to the middle of the screen instead of sitting on the floor.

**The type.** Markdown is now highlighted: H1 at 1.9em and 700, H2 at 1.5,
bold actually bold, code and links on the accent and the good colour, and the
punctuation Markdown is made of — `#`, `*`, backticks — dimmed rather than
removed. Sizes are `em`, so they scale with the font size the settings panel
controls instead of ignoring it. The syntax stays on screen: hiding it is Live
Preview, which is §18, and a bigger heading is not a step towards it.

**The editor's colours came off hard-coded hex and onto the tokens.** Four of
them — caret, gutter, active line, selection — were written in a TypeScript
object, which is exactly the blind spot `tools/contrast.sh` was given a guard
for one commit ago: the guard reads the stylesheet and could never have seen
them.

**The divider** is a `role="separator"` with a value, not a `<div>` with a
mousedown. It takes focus, the arrows move it two points at a time, `Home` and
`End` go to the limits, `Enter` and a double click even it up, and it reports
its position so a screen reader says something better than "5 pixels wide". A
drag handle reachable only by mouse is a control half the people using this
application cannot operate.

The drag listens on the document rather than on the handle — a fast drag leaves
a 5-pixel target behind long before the button comes up — and writes a CSS
custom property instead of React state, so a drag costs one style write per
mouse move rather than a re-render of a pane containing CodeMirror.

## 0.12.2 - the shell: a rail, a real sidebar, a note header, and a contrast check that found a bug in my own palette

Step 2 of 0.1d (`.continue/0.1d-interface.md` §6). Everything in its place;
the fine styling is step 3.

**The rail**, 44 px on the left: files, search, graph, and settings at the foot.
Clicking the icon of the panel already showing collapses the sidebar, which is
the only way to give the editor the whole window. Graph is rendered
**disabled with its milestone in the tooltip** — ADR-038 moved it from "out of
scope" to 0.3, and a promise with a date on it is worth more than a gap in the
rail.

**The sidebar** is one column with three parts: a toolbar that acts on the panel
(new note, new folder, sort, collapse all), the panel, and the workspace
selector pinned below the scroll. Global search moved *into* it — it used to be
a third column that pushed the editor sideways.

**The tree's context menu is now the shared `Menu`**, so it has arrows, `Escape`
and the focus return that 0.12.1's eleven tests cover. It also gained a `⋮`
button, revealed on hover and **always on focus**: hiding a control from the
keyboard is how a menu becomes mouse-only without anyone deciding it.

**The note header** carries back/forward, the centred title (file name without
`.md`), the Source/Preview toggle and a `⋮`. It sits below the tabs rather than
in a top bar because at split there are two of them, one per pane — a header in
the window chrome could not be. Back/forward is one history for the window
rather than one per tab, and D-06 says why: a tab here is a note, not a
viewport, so there is no navigation *within* one to have a history of.

**The status bar** moved right and gained words and characters. Characters are
code points, so an emoji counts once. **Nothing else is there**: a backlinks
counter needs the index that arrives at 0.3, and a counter with no data behind
it is a lie with the face of a feature (§3). Background work — the watcher's
walk — sits on the left, discreet, and disappears when it finishes.

The old top bar is gone; its diagnostics moved into Settings, which is where a
thing you look up rather than read belongs.

**`tools/contrast.sh`**, and it earned its place immediately: it failed on the
palette I had just written. The three dark levels were 1.05:1 apart — exactly
the *"visível numa tela ruim"* failure §5 names.

Two things were wrong and both were mine. `--line` was being held to WCAG's
3:1 for non-text contrast, which does not apply: 1.4.11 covers what identifies
a **control**, and a rule between two panels that are already different surfaces
identifies nothing — holding it to 3:1 means a near-white hairline brighter than
the text beside it. And the level check was a contrast *ratio*, which is the
wrong instrument near black: the formula adds 0.05 to both sides to model screen
flare, and that constant swamps the difference. My first floor of 1.15:1 was
unreachable by any palette that still reads as one tone of dark.

Levels and dividers are now checked as a **step in 8-bit sRGB** — 8/255, about
where a cheap panel stops merging two greys — and text and the focus ring keep
their WCAG ratios. The script says which number is a standard and which is this
project's own.

Then it found three more: `--disabled` at 2.15:1 was a smudge rather than the
legible-but-inert control §3 asks for, and `--fg-dim` on a selected row was
4.36:1. Both fixed in the palette.

And the hole underneath all of it: **a colour written outside `:root` is a
colour the checker cannot see.** Thirty-odd inline hexes were doing exactly
that. They are now role tokens — `--hover`, `--selected`, `--field`, `--scrim`,
the tint family — and the script fails the build if a raw hex appears below the
token block. 41 pairs checked, up from nine.

## 0.12.1 - 0.1d opens with the two ADRs and the bug: you can change folder again

Milestone 0.1d — Interface — enters the scope, and the desktop MVP becomes
`0.1a + 0.1b + 0.1c + 0.1d`. Two ADRs, in the first commit as the milestone
asks:

**ADR-037.** The interface is a milestone, not a finishing pass. The reference
is Obsidian's dark layout; the rule is that palette, spacing and structure are
free and **no theme file, stylesheet or asset is copied — everything is
rebuilt**. Icons are `lucide-react` (ISC). The twenty-five flows of 0.1b and
0.1c are re-indexed into `ACCEPTANCE-0.1d.md` rather than ticked where they are:
the steps move, the behaviour does not, and a flow whose steps no longer
describe the window is not a flow anyone can walk.

**ADR-038.** Graph view leaves §18's "out of scope until further order" and
becomes 0.3, after backlinks — a graph is a rendering of a link relation, and
backlinks are that relation. It is **not** part of 0.1d; the icon rail carries
it disabled with its milestone in the tooltip, which is the honest way to show
something that is coming.

**And the defect.** Every command needed to change workspace has existed since
0.1a — `workspace_open`, `workspace_create`, `workspace_recent`,
`workspace_close` — and the only surface that reached them was the Welcome
screen, which disappears the moment a folder is opened. After the first open
there was **no way to change folder at all**. A command with no route to the
user is a command that does not exist.

The selector lives in the sidebar footer, in a scroll-proof row: it is the one
control that has to be reachable at every moment, because it is how a user
leaves a workspace they opened by mistake. Switching is `close` then `open`, in
that order, so `close_workspace`'s `DirtyBuffers` refusal is **on** the path
rather than beside it; a dirty buffer asks, by name, in the application's own
modal — never `window.confirm`, which does nothing in a WebView.

`tests/switch.rs` — six tests over a path nothing had ever exercised, because
until now it was unreachable. They pin behaviour rather than a fix: a
store-before-adopt was written into `open_workspace` and then **removed**, because
the tests passed identically with and without it. `ARCHITECTURE.md` §4.1
describes a two-second registry debounce that is not implemented, so there is
nothing unwritten to lose — recorded as `DECISIONS-0.1d.md` D-01 rather than
pre-fixed, and the day the debounce lands those tests start failing, which is
the outcome to want.

`Menu.tsx` is the one popup the workspace selector, the note header's `⋮` and
the explorer's context menu will all be. Eleven tests, in a DOM, because focus
is the subject: a menu that keeps focus leaves a keyboard user on `<body>` with
no way back. `jsdom` and testing-library join the **dev** dependencies for it
(D-02) — `vitest` stays on `node` by default and a file opts into a DOM on its
first line.

The three questions `.continue/0.1d-interface.md` §10 left open are answered by
the rule and recorded: the editor column is a fixed maximum rather than a fifth
setting (D-03), split is horizontal only in this milestone (D-04), and Welcome
stays a screen rather than becoming an empty shell (D-05).

## 0.12.0 - artifacts on a minor bump, and a patch release that says why it is empty

The owner's call on yesterday's cost: *"9 min e 105 MB por commit de doc não se
justifica."* ADR-011 makes every commit a version and every version a Release,
and `build.yml` was packaging all of them — twelve full builds in one session,
three concurrent, with the CI job that gates the work queued behind an AppImage.

`build.yml` now builds a version whose patch component is `0`, plus any version
asked for through `workflow_dispatch`. A patch Release carries no artifacts and
**says so in its own description**, with how to get them — an empty downloads
section otherwise reads as a build that failed. The note is written once; a
marker keeps a re-run from appending it twice.

The rule is arithmetic on the version string rather than a diff of what changed.
A patch that touches the editor gets no artifacts even though the binary is
different, and a minor bump that only moves documents gets a full set. Deciding
by content means defining which paths count, keeping that list right, and
explaining an empty Release whose commit *looks* like code. The version number
is a decision the author already made; reading it is cheaper than
second-guessing it (ADR-036).

**This release is the first minor bump under the new rule**, which makes it the
first one built by it.

## 0.11.11 - one build at a time, because the burst was starving CI

An operational consequence of yesterday's release pipeline, observed rather than
predicted: every commit is a version, every version gets a Release, and
`build.yml` queued a nine-minute build for each. Three ran concurrently while
the CI job that actually gates the work sat queued behind an AppImage.

The concurrency group is now `build` with `cancel-in-progress`. A burst of
commits produces one build — the newest. What it costs is that an intermediate
version can end up with no artifacts, and that is the right trade: nobody
installs the middle of a working session, and what has to be installable is
exactly the one this rule always builds. Filling an older one in afterwards is a
manual `workflow_dispatch` with its version as the input.

Cancelling mid-run needed the completion check to get stricter. It looked for a
`.deb`, which a run cancelled halfway will have already uploaded; it now looks
for the `.SRCINFO`, which the Arch job uploads last. Every upload already used
`--clobber`, so rebuilding over a partial set is safe.

## 0.11.10 - the churn test measured the runner, not the rule

`an_index_that_is_still_building_is_not_restarted_by_a_change` was green here
and red on Ubuntu, Windows and Arch — and this time the code was right and the
test was too big.

It is the one test whose **main thread competes with the walk for the disk**: it
creates a note per iteration precisely to keep invalidating. On a two-core
runner with four other tests building corpora beside it, a 7 200-directory tree
means the walk gets no I/O and the assertion fires on a runner's contention
rather than on a restart. The evidence is in the failure itself —
`indexed: 2398, building: true` after 1 632 changes: climbing steadily, which is
exactly not what a restart looks like.

The corpus drops to 120 repositories and the ceiling rises to two minutes. The
property is size-independent, and both directions are re-verified at the new
size: 1.5 s green with the rule, and with the old rule put back,
`indexed: 0, building: true` after 6 606 changes and the full two minutes.

Two stale index lines went with it: `docs/README.md` now describes the runbook's
release section, and `.continue/README.md` no longer says the repository is at
`0.1.0`.

## 0.11.9 - the deep fixture becomes a CI criterion, not a local measurement

The criteria added at 0.11.0 build their own corpus so they can run on every
push; the 20 962-directory fixture the freeze was actually measured on was a
local `--ignored` run and nothing enforced it. Generating it costs 2.8 s, so
now CI does, on the Linux job, and the measurement is a criterion:

- `open_workspace` + `list_dir` on 20 962 directories, **under a second**;
- `start_watch` and the first `quick_open`, **under 100 ms each** — they cost
  502.72 ms and 549.88 ms before ADR-034;
- `quick_open` returns `Ok`, not `Err(PermissionDenied)`, with the mode-000
  directory in place;
- `degraded` stays `None` — one unreadable directory does not demote the
  workspace to polling.

Linux only, for the two reasons the rest of it is: the per-directory counters
are inotify's (D-10), and it is the one runner in the matrix that is not root,
which is what the Arch job taught at 0.11.3.

The whole sequence on that fixture is now **1.59 ms**, against 1 053.73 ms
before — and the "before" number had to be taken with the unreadable directory
temporarily made readable, because with it in place there was nothing to
measure: the run aborted in under a millisecond.

## 0.11.8 - the front door stops saying there is no application

`README.md` said **"Documentation only — there is no application code in this
repository yet"** while the repository built a Markdown editor, tested it on
four platforms and published a `.deb`, an AppImage and an Arch package. A
release nobody can find is not a release, and the first file anyone opens was
telling them not to look.

It now says what is built, how to install it on each of the three Linux routes,
and how to build it. It also says the thing the acceptance documents say and the
old text never had to: **nobody has walked the interface.** Every criterion so
far is an assertion about the core, and the twenty-five flows stay unticked
until a person has done them.

`docs/roadmap.md` goes from `PROPOSED` to `ACTIVE` for 0.1 only, with the three
sub-milestones and the versions they shipped in. Everything from 0.2 on is still
a specification.

## 0.11.7 - the index stopped starving itself on a workspace that keeps changing

A bug the background index introduced, found by asking what happens on a folder
that is being written to while it fills.

ADR-032 drops the quick-open list on every operation that changes the tree, and
on every reconciliation that saw an event. That was right when building the list
was a 30 ms walk inside the call. It is wrong once the walk is background work
that takes **15.8 s on `~/x`**: any folder with continuous activity in it — a
build, an `npm install`, a `git checkout` — invalidates faster than the walk can
finish, so each `Ctrl+P` restarted it from zero and quick open returned an empty
list for as long as the activity lasted.

`quick_open` now rebuilds when there is no index, or when the list is stale
**and the previous walk has finished**. The staleness is remembered rather than
dropped; the next call after the walk ends starts a fresh one, and the palette
says `building` for the whole of it.

Measured by putting the old rule back:
`QuickOpen { matches: [], indexed: 0, building: true }` after **2 919 changes**
and thirty seconds. With D-11 the same test settles in 1.4 s with the whole
workspace indexed, while the changes are still arriving.

## 0.11.6 - the watch-limit sentence is asserted; the behaviour behind it is not

The one claim in this milestone that nothing exercised. `notify` reports an
exhausted watch table as an ordinary I/O error, so errno 28 is the only thing
separating "this kernel has run out of watches" — which a `sysctl` fixes — from
"this path does not exist", which it does not. Getting that wrong costs the user
the single instruction that would have helped, so it is now asserted: errno 28
classifies as `WatchLimit` and its message carries
`fs.inotify.max_user_watches`; errno 2 does not, and must not offer a command
that would not help.

**What is still not verified is the behaviour on a full table** — keep the
watches already installed, count the remainder, do not demote the workspace.
Reaching that state means lowering `max_user_watches`, which is 1 048 576 here
against the 49 937 `~/x` needs, and the inotify sysctls are not writable from an
unprivileged user namespace on this kernel. The ENOSPC suite does exactly that
trick for a full disk; it does not work for this. Recorded as not verified in
`ACCEPTANCE-0.1b.md` §6 rather than left to look tested.

## 0.11.5 - the folder that froze it, measured on the folder that froze it

`fixtures/deep` is a reconstruction. `~/x` is the original, and
`deep.rs::where_the_time_goes_on_a_real_folder` now measures it directly, behind
`NOTES_DEEP_ROOT`. Nothing in it writes to the folder: `open_workspace` reads,
and the case probe is read-only by construction (scope §2.3).

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

Over two minutes to **0.59 ms**. The two walks that used to cost it are 7.3 s
and 15.8 s of background work, with the window usable throughout.

The `unreadable: 1` is `.../www/web1/ead` — the directory whose
`Permission denied (os error 13)` was in the banner. It is a number now; it used
to stop the workspace being watched at all, and to make `quick_open` return
nothing for the whole folder.

`over_limit: 0` because this machine's `max_user_watches` is 1 048 576 and the
folder needs 49 937. A default Linux ships 8 192 or 65 536, where the same
folder leaves tens of thousands over the limit — the state the banner exists to
name, and the one thing in this milestone that nothing has exercised.

56 622 notes indexed under `~/x`, most of them inside `node_modules/`. That is
D-08's argument as a number: hiding the folder by name would hide all of them.

## 0.11.4 - the walk is actually cancellable, and the numbers moved to the banner

Two gaps between what ADR-034 says and what 0.11.0 shipped.

**"Cancellable" was true of the quick-open index and not of the watcher.**
`PathIndex` stops when it is dropped; the watcher's walk ran to completion
whatever happened to the `Watch`, so closing a workspace or opening another left
a thread installing inotify watches on a folder nobody had open — minutes of it
on a large tree. `add_watches_below` now checks the stop channel once per
directory, which is the granularity it already works at.

Asserted by what it does rather than by a thread's death, which is not
observable: `Watch::counters()` hands out the live counters, the test drops the
`Watch` three directories into an 8 000-directory walk and reads the count it
stopped at. Removing the check again fails it — *"the walk stopped where it was
rather than finishing: 8001 directories"*.

**The over-limit and unreadable counts moved from the status bar to a banner.**
The rule the milestone set is that a full watch table degrades only the excess
and **says the number**; a count in the corner of a status bar is not something
a user can act on. The banner names how many directories did not fit and the
`sysctl` that raises the limit, and separately how many folders could not be
read. What stays in the status bar is the walk's progress, which is transient
and gone the moment it ends — the one reading that would be noise as a banner.

## 0.11.3 - two tests that asserted about the runner instead of the code

The 0.11.0 criteria were green here and red on two of the four CI platforms, and
in both cases the test was wrong rather than the code.

**Arch, as root.** The container job runs the suite as root, and root reads a
mode-000 directory anyway — `CAP_DAC_OVERRIDE`. A test about *skipping an
unreadable directory* has nothing to exercise there, so it asserted that a
directory it could read had been skipped. It now probes first —
`permissions_are_enforced_here()` creates a mode-000 directory and checks
whether reading it actually fails — and skips with a reason when it does not.
A uid check would have been the same test written to guess; this asks.

**macOS.** `an_unreadable_directory_does_not_demote_the_workspace` asserted the
per-directory counters, which 0.11.1 made Linux-only on purpose: FSEvents
watches the subtree from one handle and never reads the tree, so it cannot meet
an unreadable directory. The half that is universal — the workspace is still
watched, `degraded` is `None` — stays universal; the counting is guarded.

The two edits that missed in 0.11.1 missed for one reason: they were written
against the file as it read before `cargo fmt` split the assertions across
lines, and the replacement was made without checking that it had matched.

## 0.11.2 - the Linux release: .deb, AppImage, tarball and an AUR package that was actually built

`ARCHITECTURE.md` §15 has described this since 0.1a and none of it existed.
Now it does, and all of it was run before it was committed.

**`build.yml`** builds the artifacts and attaches them to the Release
`release.yml` already publishes. Two workflows on purpose: publishing a Release
must not wait on, or be failed by, a compiler — a broken build should leave a
Release with notes rather than no Release. The trigger is `workflow_run` rather
than `on: release`, because `release.yml` creates the Release with the built-in
`GITHUB_TOKEN` and GitHub fires no workflow events for what a `GITHUB_TOKEN`
did; an `on: release` job here would simply never have run.

**Linux**: `.deb` and AppImage from the Tauri bundler, on `ubuntu-22.04` rather
than `ubuntu-latest` — a `.deb` links against the glibc it was built on, so the
oldest supported runner is the widest audience.

**Arch** (ADR-023): `packaging/aur/notes-bin/PKGBUILD.in` plus
`gen-pkgbuild.sh`, and a job that runs a real `makepkg` in an `archlinux:latest`
container against the tarball the previous job produced — then installs the
package and checks `ldd` resolves. Not a lint of a PKGBUILD: a package that
builds nowhere but the maintainer's machine is not a release target. The
tarball is unpacked from the `.deb` rather than assembled, so the `.desktop`
entry and the icon set are the ones the bundler produced and not a second copy
that drifts.

**macOS and Windows are written and disabled** (ADR-024), each behind `if:
false` with the list of what is missing: an Apple Developer membership and a
Developer ID certificate for notarisation; an OV code-signing certificate for
SmartScreen. Neither is engineering. A job that does not exist is a job nobody
can cost.

**ADR-035 — the version is stamped, not maintained twice.** `tauri.conf.json`
said `0.1.0` while `version.md` said `0.11.1`; a package attached to Release
`0.11.1` calling itself `0.1.0` cannot be matched to the code that produced it.
`tools/stamp-version.sh` writes `version.md`'s version at build time, the
committed value is `0.0.0`, and CI and `tools/check.sh` both fail on anything
else. The `PKGBUILD` is generated the same way, checksum included.

Two things the first build got wrong and this one does not: the binary was
installed as `usr/bin/notes-app` — the crate name, an artefact of the workspace
layout rather than the name of the program — fixed with `mainBinaryName`; and
`Depends:` listed `libwebkit2gtk-4.1-0` and `libgtk-3-0` twice, because Tauri
already derives them.

Verified end to end on this machine before committing: the `.deb` builds and
carries the right paths, the AppImage builds, the tarball unpacks from the
`.deb`, and `makepkg` in a real `archlinux:latest` container built, installed
and resolved a `notes-bin` package from a binary compiled on Debian 13.

## 0.11.1 - native_id on Windows, and the watcher stops being one shape for three platforms

**D-24 closes.** `native_id` returned `None` on Windows from 0.1a because the
standard library's `volume_serial_number` and `file_index` are behind the
unstable `windows_by_handle` feature — the Windows job did not fail a test, it
failed to build. It now reads the same two numbers through
`GetFileInformationByHandle` (`windows-sys`, one target-gated dependency, one
call): `access_mode(0)` so a file another process holds open still answers,
`FILE_FLAG_BACKUP_SEMANTICS` so a directory can be opened at all, and
`FILE_FLAG_OPEN_REPARSE_POINT` so a symlink reports its own identity rather
than its target's — matching the `symlink_metadata` the rest of `Stat` is built
from. `Caps::LOCAL.native_id` is `cfg!(any(unix, windows))`, which is what
`ARCHITECTURE.md` §11's matrix has promised for NTFS all along.

Because the id costs an opened handle on Windows, it is filled in by `stat` and
**not** by `list`: `Entry` carries none, and identity correlation asks one path
at a time. Seven tests in `crates/notes-fs/tests/identity.rs` run on all three
platforms in CI — the capability agrees with the value, a rename keeps the id,
identical bytes do not share one, a directory has one, and an atomic replace
produces a new one and says so in the `Stat` it returns.

**And a correction to 0.11.0.** That commit moved the watcher to one watch per
directory on every platform. That is right on Linux, where an inotify descriptor
covers exactly one directory and `notify`'s recursive mode is a walk it does for
you. It is wrong everywhere else: FSEvents watches a subtree from one handle and
`ReadDirectoryChangesW` takes a `bWatchSubtree` flag, so the walk would have
replaced an O(1) call with 20 000 kernel objects on the deep fixture and
hundreds of thousands on the folder that started all this — the same mistake as
the freeze, introduced by the fix for it.

`PER_DIRECTORY` is now `cfg!(target_os = "linux")`. The two hazards the walk
handles are Linux's too: a recursive add failing whole on an unreadable
directory is inotify enumerating, and `max_user_watches` is an inotify sysctl.
`WATCH_SKIP` applies only where there is a watch table to protect (D-10).

## 0.11.0 - the tree in under a second, and two walks moved off the critical path

Opening `~/x` — around 160 repositories with `node_modules/`, `target/` and
`.git/` — froze the Welcome screen for over two minutes, with a banner naming
one unreadable subdirectory. That is not a notes workload, and it does not have
to be: the application may not freeze on any folder.

**Measured first.** `tools/gen-deep.sh` builds the same shape — 20 962
directories, with a mode-000 directory and a symlink loop in it — and
`crates/notes-core/tests/deep.rs` times each step of opening it:

```
open_workspace:        0.70 ms
list root:             0.43 ms   (160 entries)
start_watch:         502.72 ms
quick_open first:    549.88 ms
to a usable tree:      1.13 ms
```

The tree costs a millisecond on 21 000 directories, because it is lazy. The
freeze was `start_watch` and `quick_open`, each walking the whole tree inside a
`#[tauri::command]` holding `Mutex<WorkspaceService>` — so `tree_list` did not
run slowly, it did not run at all until they finished. The fixture also found a
second bug that had nothing to do with time: with the mode-000 directory in
place, `quick_open` returned `Err(PermissionDenied)` for the entire workspace,
and `notify`'s recursive add did the same to the watcher, demoting the whole
folder to polling because of one directory.

**The rule, now an acceptance criterion (ADR-034).** `workspace_open` returns and
the tree appears in under one second at any size. Everything that needs the whole
tree runs on its own thread, is cancellable, and reports progress to the status
bar: the watcher's per-directory walk (`notes-fs::watch`) and quick open's path
list (the new `notes-core::index`). Both use an explicit stack and
`symlink_metadata`, so a symlink loop cannot be entered. Both count and skip a
directory they cannot read. A full watch table degrades **only the excess** —
the watches already installed keep working, the remainder is counted, and the
banner names the number and the `sysctl`.

`quick_open` now returns `QuickOpen { matches, indexed, building, unreadable }`
and answers from a partial index while it fills; the palette says *"still
indexing — N notes so far"* rather than "nothing matches". The status bar shows
the watcher's coverage while it walks.

**`node_modules/` and `target/` do not go into the default ignores** (D-08). They
hold real Markdown, and hiding a folder by name is the application deciding which
of the user's files are real. They go into the watcher's skip list instead, which
is a different list answering a different question — a watch is a finite kernel
resource, visibility is not. Changes inside a skipped directory still arrive
through the 5 s scan.

Three new automated criteria, in CI rather than behind `--ignored`, over a corpus
each test builds: the tree under a second on 2 160 directories; `start_watch` and
the first `quick_open` each returning in under a fifth of the walk they replace,
over 7 200 directories. The assertion is a ratio against the walk measured in the
same test rather than a millisecond budget, because at this size a synchronous
walk costs ~30 ms and any absolute budget worth writing would let it through.
Both were verified by putting the regression back: the inline walk fails them.

`fixtures/large` could never have caught this. It is 10 000 notes, flat, and
lists in 37 ms. The axis that broke was directories.

## 0.10.3 - milestone 0.0 closes on this machine, and the queue drops three rows

The owner ran it without the environment variable and then **switched the failure
back on**: with `NOTES_NO_DMABUF_WORKAROUND=1` the GBM error returns. That was
the control the Debian section was missing — until it was run, "the workaround
fixed it" was inference, and now the failure has been turned off and on again.
The section is closed.

Three rows leave `.continue/`. Dependabot #1 was merged at `0.9.6`, and
`PROGRESS-0.1b.md` describes where a milestone stopped that shipped at `0.9.0` —
both were items the `QUEUE-RULE` says should have gone with the commit that
carried the work. The repodocs skeleton defect leaves for a different reason: it
is **not this repository's item**. It is a defect in repodocs' skeleton, fixed
here at `0.1.0` and still shipping from there to every new repository, so it is
recorded in "where things went" pointing at the repository that can fix it rather
than sitting in a queue that cannot.

## 0.10.2 - the record catches up with the cause

Three documents said something the run at `0.10.1` disproved.

**ADR-033** amends ADR-022. The Wayland half of its condition was wrong, and the
log is quoted in full as the evidence rather than summarised. It records what the
change costs — every Linux machine with the proprietary driver now turns the
DMA-BUF renderer off, X11 included, which is a real performance cost on hardware
where the bug may never have shown — and why that is the right side of the
trade: applying it needlessly is slower compositing, not applying it is no
window. It also says what it does **not** claim: whether the original Wayland
black-window reports share this mechanism is not established, and ADR-022's
account of those is left standing.

**D-20 is marked resolved and wrong.** Its reading — a GTK file chooser taking
its parent down — fitted the evidence and was not the cause, and the reason is
named: every run behind it had `WEBKIT_DISABLE_DMABUF_RENDERER` already exported
in the owner's shell, so the workaround never ran and its absence could not be
observed. A masked symptom produces a plausible mechanism. The GTK hypothesis and
the portal workaround are left as written rather than edited away, because a
decision log that deletes its wrong turns stops being evidence of how the
conclusion was reached.

**`SPIKE-0.0.md` had a fabricated line.** It reported `nvidia false` for this
machine and concluded criterion 1 could not be exercised here. The machine has a
GTX 1060 with all four proprietary modules loaded and `/proc/driver/nvidia/version`
present; the detection said `true` all along, and what was false was the
document. It had been written from expectation rather than from a run, and that
is the failure mode the whole spike document exists to prevent.

Debian 13 / X11 / NVIDIA is now a checklist section of its own with both boxes
answered — the failure reproduced with the old rule, and fixed by the new one —
plus one box left open on purpose: turning the workaround `off` should bring the
failure back, and until someone sees that, "the workaround fixed it" is inference
rather than observation. The Arch boxes stop asserting `session wayland`, since
the session is reported and no longer required.

`ACCEPTANCE-0.1b.md` and `ACCEPTANCE-0.1c.md` are untouched, as instructed.

## 0.10.1 - the dmabuf rule required Wayland, and the failure never did

The window that disappeared on *Open Folder* was found, and it was not the file
chooser. Run without the environment variable that had been masking it, on
Debian 13 / X11 / NVIDIA:

```
[notes] dmabuf: not needed — session is not wayland
src/nv_gbm.c:288: GBM-DRV error (nv_gbm_create_device_native): …failed (ret=-1)
KMS: DRM_IOCTL_MODE_CREATE_DUMB failed: Permission denied
Failed to create GBM buffer of size 1100x720: Permission denied
[notes] window main: close requested
[notes] window main: destroyed
```

**The workaround declined to apply, and one line later the reason it exists
happened.** WebKitGTK has used the DMA-BUF renderer on **X11 since 2.42**; the
fault is in NVIDIA's GBM, not in a compositor, so requiring Wayland was checking
the wrong thing. `decide` no longer takes the display server at all: on Linux,
the proprietary NVIDIA driver is the whole condition.

`nouveau` does **not** count, and the function says how it is told apart rather
than matching a name: `/proc/driver/nvidia/version` is created by the
proprietary kernel module and by nothing else, `/sys/module/nvidia/` is that
module's own sysfs directory while nouveau's is `nouveau`, and `nvidia-smi` is a
weaker hint that nouveau never ships. Nouveau's GBM works, and turning the
renderer off there would cost compositing performance for nothing. It is reported
in the diagnostics beside the proprietary flag, so the panel shows which one is
loaded.

The six tests become nine, and the one that mattered flipped: *"skips X11 even
with NVIDIA"* — an assertion of the bug — is now **"applies on X11 with
NVIDIA"**. `session_kind()` keeps a test of its own because the diagnostics still
report it; it just no longer decides anything.

Verified on the machine that produced the failure, without the variable:
`dmabuf: applied — proprietary nvidia driver detected`, no GBM error, and the
window renders.

**Two of this repository's own claims were wrong and are corrected by it.** The
instrumentation added at `0.9.5` is what named the event — `close requested` then
`destroyed` — and D-20's reading of the evidence, that a GTK file chooser was
taking its parent down, was a plausible mechanism built on a false premise: the
run that produced it had the variable set, so the workaround never ran and the
comparison was never made. And `docs/SPIKE-0.0.md` recorded *"nvidia false"* for
this machine, which has a GTX 1060 with the proprietary modules loaded — that
line was written from expectation rather than from a run.

## 0.10.0 - milestone 0.1c ships, and the MVP desktop is complete

`0.1a + 0.1b + 0.1c` is the desktop MVP of `SCOPE_final.md` §17.

**Both criteria are met, and both were measured rather than asserted.** The first
result over 10 000 notes and 197 MiB arrives in **11.4 ms** against a ceiling of
500 ms, and cancelling returns in **650 ns**. The restart criterion is automated
on both sides of the IPC, because it spans both: five tests in `notes-core` prove
tabs, the active tab and the cursor survive a restart through a *different*
service over the same data directory, and twenty in the store prove it puts them
back.

Three decisions are promoted to ADRs:

**ADR-030** — tabs are a list *beside* the editor, not a second document model.
The editor holds one loaded document and all of 0.1a and 0.1b is written against
that: the write protocol, the stale-save guard, the draft rules, the conflict
state. Making it hold a map to gain a tab strip would put every one of those back
in play for a navigation feature, when two notes are never visible at once. The
cost is stated: switching tabs re-reads from disk, and the moment two documents
must be visible the decision is to be revisited rather than worked around.

**ADR-031** — global search is a scan the core owns and the frontend **polls**,
the same way it polls reconciliation. `ARCHITECTURE.md` §7.2 had sketched events;
two delivery mechanisms for two streams of the same kind is one more than this
application needs. And the scanner does not retire when FTS5 arrives: §10
requires literal, words and regex to keep their names, and the index only takes
over *words*.

**ADR-032** — quick open matches a cached path list that the tree invalidates.
Walking 10 000 notes is fine once and ruinous per keystroke; a stale list offers
a note that is not there, so every create, rename, move, duplicate, delete and
reconciliation tick drops it. Coarse on purpose.

**`notes-index`, SQLite and the `registry.db` move are not here**, and that was a
stop rather than a preference. The instruction that opened the milestone asked
for them; §17 puts them at 0.2, §10 says the 0.1c search is a scan and that FTS5
takes over word search at 0.2, and **ADR-015 is `ACTIVE`** saying the registry
moves at 0.2. Building them would have contradicted an ACTIVE ADR and made this
milestone's own search criterion untestable as written. `DECISIONS-0.1c.md` D-01
records it with the alternative: an ADR superseding ADR-015 and an edit to the
scope, in that order.

One gap the acceptance document found before a user could: **C8 says clicking a
search hit opens the note *at that line*, and the panel only opened the note.**
Writing the row first is what surfaced it. `openAt` now sets the target before
the note opens — so a fresh view mounts on it — and bumps a counter for the case
where the note is already on screen and the view will not rebuild.

`ACCEPTANCE-0.1c.md` has existed since the milestone's first commit with its
thirteen interface rows, **and not one of them is ticked.** A screenshot showed
tabs, the active tab, the workspace and Split all restored from a seeded session;
that is written down as evidence and explicitly not as a tick, because a caret
position is not visible in a screenshot and nobody has clicked anything.

## 0.9.11 - tabs, quick open, workspace search, the command palette and settings

The interface half of milestone 0.1c.

**Tabs are a separate store, and that is the decision worth stating.** The editor
holds exactly one loaded document and all of 0.1a and 0.1b is written against
that; making every consumer tab-aware to gain a tab strip would put the write
protocol back in play for a navigation feature. So the new store owns the *list*
and the editor keeps owning the *document*. A tab carries only what has to
survive a restart — path, identity, cursor, scroll — and the buffer stays where
it was.

Leaving a tab is not a new rule either: a dirty note is **flushed** and a note in
conflict writes its **draft** instead, which is what `ARCHITECTURE.md` §5 already
says happens when a buffer stops being looked at.

The cursor is the part of the restart criterion that is easy to lose, because it
can only be applied *after* the editor has mounted the document — a position in a
document that does not exist yet means nothing. So the editor reports the caret
on every selection change and asks the tab store for one when it builds a view,
and the store suppresses reports while a restore is in flight so a freshly
mounted editor does not overwrite the position being restored.

**Quick open and the command palette are one surface**, because filter-arrow-
`Enter` over different rows is one interaction. **Global search is not**, and is a
panel rather than a modal: its results are something you work through, not
something you pick from. It polls the core the way reconciliation does, and says
two things out loud that scope §10 requires — that it reads **what is on disk**,
with an explicit warning when the open note has unsaved changes, and that a
truncated list is the first N rather than all of them.

Settings apply as they are changed, with no Save button, for the same reason a
note has none: a panel that can be closed with unsaved changes is a way to lose
them. Font size, line numbers, wrapping and tab size are CodeMirror *extensions*,
so each one rebuilds the view — they are in the effect's dependency list rather
than applied to a live one, which is the honest way to say it.

Thirty-eight new strings, in both catalogues, and CI still fails if they diverge.

## 0.9.10 - milestone 0.1c starts with search in the core, and its acceptance document

**First result in `fixtures/large` in 11.4 ms, cancel in 650 ns** — two orders of
magnitude under the criterion, measured over 10 000 notes and 197 MiB.

The shape is what makes it hold rather than the language: the walk is parallel,
hits are pushed **as they are found** instead of collected and returned at the
end, and every worker reads the cancel flag before each file — so cancelling is
bounded by one file, not by the workspace. Dropping a `Search` cancels it, which
is why typing a second query cannot leave the first one scanning 197 MiB for
nobody.

Quick open is deliberately a **different thing** and not a cheap query over the
same scanner: it matches paths from a list held in memory and never opens a
file. The list is built once and dropped whenever the tree changes shape — every
create, rename, move, duplicate, delete, and any reconciliation that saw a file
appear or vanish. Rebuilding it for ten thousand notes costs tens of
milliseconds, which is fine once and ruinous per keystroke. Its scoring is small
and explainable rather than clever: a subsequence, a bonus for consecutive
characters and for landing at the start of a segment, and the file name ranked
ahead of the directory, because `Ctrl+P` is how someone reaches for a file they
can name.

Sixteen tests cover what the timing does not: a literal query is **not** read as
a pattern (`(a.b)` finds `(a.b)`), regex mode is separate and named, case
sensitivity is opt-in, an invalid pattern is refused instead of scanning for
nothing, only notes are searched, `.git/` is never a hit, and — scope §10 —
**search reads the disk, so a buffer typed and not saved is not reported as
found.** A test asserts exactly that, because it is the fact the interface has to
tell the user rather than let them infer.

`docs/ACCEPTANCE-0.1c.md` exists **from this first commit**, with its "verified
in the running app" section already written and **thirteen rows, none ticked**.
0.1b shipped five green criteria over six dead flows because every criterion was
an assertion about the core; a section that only appears once the work is
finished is a section that agrees with whatever was built.

`notes-index`, SQLite, FTS5 and the `registry.db` move are **not** in this
milestone. §17 puts them at 0.2, §10 says the 0.1c search is a scan and that FTS5
takes over word search at 0.2, and ADR-015 is `ACTIVE` saying the registry moves
at 0.2. The acceptance document says so where a reader will look for it.

## 0.9.9 - the contracts job ran a suite twice and threw away the reason it failed

CI went red on `contracts` while `rust (ubuntu-latest)` — which runs the same
tests properly, with system dependencies and a cache — went green. The failure
was `notes-core --test reconcile`, and the log said nothing beyond *"test
failed"*, because the step ends in `>/dev/null`. **A step that discards its
output has thrown away exactly the thing that is worth having at the only moment
it matters**, and I wrote that line.

The output is kept now. And the step stops running the integration suites at all:
its purpose is to regenerate the TypeScript and diff it, `#[ts(export)]` emits
its writer as a **lib** test, so `--lib` still produces all fifty-one types.
Running the integration tests there duplicated a job that already exists and
bought nothing but a second chance to be flaky — which is what it spent.

The `reconcile` suite passed eight consecutive runs locally, so what is fixed
here is the duplication and the missing diagnostic, not the flake. If it is real
it will now surface in the job built to run it, with output attached. The likely
mechanism is written down rather than guessed at in silence: those tests assert
against a self-write expectation with a **two-second wall-clock TTL**
(`reconcile.rs`), and a cold, loaded runner is where a wall clock in a test first
disagrees with the machine that wrote it.

## 0.9.8 - what the acceptance document could not see: the interface

Every 0.1b criterion is an assertion about `notes-core`, and all five were met
while **six flows of the same milestone were dead** behind a dialog the WebView
does not have. A criterion satisfied in the core says nothing about the
interface. `ACCEPTANCE-0.1b.md` now has a section that says so and keeps the two
apart.

**Six things are marked verified**, because they were watched on screen on
Debian 13 / X11: the Welcome screen paints, the last workspace restores with no
dialog at all, the tree lists and marks notes from non-notes, a note opens into
CodeMirror with highlighting, **Split renders the preview beside the source** —
`notes-markdown` through the IPC, doing its job — and the status bar reports
`✓ saved`.

**Twelve are marked not verified, and none is ticked.** The six dialog flows, the
three view modes, the conflict compare screen, the three resolutions and in-file
search. The reason is the machine, not a judgement about the code: this window
manager will not raise the application window — `xdotool windowactivate` returns
`_NET_ACTIVE_WINDOW failed`, and `windowraise` and `wmctrl -a` do nothing — and
WebKit ignores synthetic input delivered to an unfocused window. **The window can
be photographed and cannot be driven.** Each step is written out so a person can
walk it, and an unticked box means a flow nobody has seen work.

`src/app/dialog.test.ts` closes the part a machine can: eleven tests over the
contract those flows depend on. That a request resolves at all; that cancelling
gives `null` for text and `false` for a confirm; that **the empty string survives
as an answer** instead of collapsing into a cancellation, which is what *move to
the workspace root* is; that the validator refuses before the core is asked; and
that a second request cancels the first rather than stacking, so no caller is
left awaiting a promise nobody settles.

Said plainly in the document, because it is the honest limit: a dialog that
resolves correctly and never renders passes every one of those tests. The
machine-checkable half is checked; the other half needs eyes.

## 0.9.7 - the other five checkouts the Dependabot PR could not have seen

`#1` was opened at 18:42 on 07/09 and `ci.yml` was written at `0.7.0`, two hours
later. So merging it bumped `release.yml` and left the five checkouts in the file
that did not exist yet — the repository ended with two versions of one action,
which is worse than one old version because nothing reports it.

## 0.9.6 - the three things milestone 0.1a left in the queue

Housekeeping, and one of the three is a rule this repository wrote about itself
and then broke.

**The scaffold row leaves the queue.** *"Scaffold milestone 0.1 — the Cargo
workspace, `apps/notes-app/`, the first crates"* has described something that
exists since `0.7.4`. The `QUEUE-RULE` says a document leaves when the thing it
describes exists, and that removing it is **the last step of the commit that
carries the work** — never a step of its own. It became a step of its own because
`.continue/` was closed to that milestone's work, which is the right instruction
and this is its cost, paid late.

**`.continue/ARCHITECTURE.md` says on its first line that it is superseded** and
names `docs/ARCHITECTURE.md`. It opened with *"PROPOSTA, aguardando revisão"* —
a document that had been answered months of commits ago still asking to be read
as current. Kept rather than deleted, because it is where the questions were
asked and its §5 is the list the live document answered; a status line is the
difference between a record and a trap.

**Dependabot #1 is merged**: `actions/checkout` 5 → 7, open since the repository
was created.

## 0.9.5 - the window that vanishes: instrumented, and one wrong claim withdrawn

The owner clicked *Open Folder…* and the window disappeared. The process exited
**`0`** with an empty `stderr` — so **not a crash**: no panic, no signal. Tauri
ends its loop when the last window is gone, which means the window was destroyed
and the application shut down normally. That is what a parent following its child
dialog down looks like from outside.

The dependency tree agrees with that reading and is stated as evidence rather
than as a conclusion: `ashpd` is absent, so `rfd` is on the **GTK3 backend**, not
the portal; it pulls `raw-window-handle`, so the chooser is parented
`transient-for` to the Tauri window; and the XDG portal is installed and running
on this machine but unused. A GTK3 chooser parented to the `GtkWindow` that hosts
the WebView, in one main loop.

**It did not reproduce, and an earlier claim that it had is withdrawn here.** The
window manager refuses to raise the window — `xdotool windowactivate` returns
`_NET_ACTIVE_WINDOW failed`, `windowraise` and `wmctrl -a` do nothing — so
synthetic clicks were landing on whatever was in front. Two runs that opened the
chooser programmatically both survived. A mechanism consistent with the evidence
is not a proven one, and swapping the dialog backend to fix a failure that cannot
be triggered on demand leaves nothing to verify against.

So the deliverable is the instrumentation: the window lifecycle is logged, and
`CloseRequested` and `Destroyed` answer different questions that are
indistinguishable from outside the process — something *asked* the window to
close, or it was destroyed outright. The next occurrence names which. The
workaround is written out in `docs/DECISIONS-0.1b.md` D-20 with its costs, so it
is not rediscovered and not applied blind.

One finding invalidates a different test. The owner's shell already exports
`WEBKIT_DISABLE_DMABUF_RENDERER`, and `linux.rs` correctly refuses to override a
value the user set — every run logged *"left alone — already set"*. **Milestone
0.0's first acceptance criterion was not exercised by any of these runs**, and
`docs/SPIKE-0.0.md` now says to unset the variable before answering that box.

## 0.9.4 - the six flows behind a dialog the WebView does not have

`window.prompt` in five places and `window.confirm` in one: new note, new
folder, the workspace name, rename, move and delete. **Those are a browser's
blocking script dialogs, and the WebView this ships in is not a browser** —
WebKitGTK, WKWebView and WebView2 each answer somewhere between "does nothing"
and "blocks the WebView's own loop". Behind one of them a flow is dead without a
sound.

Six flows of the milestone that just shipped were behind one, and **no test could
have caught it**: every 0.1b criterion is an assertion about `notes-core`, and
these live only in the interface. `0.9.0` said it in one line — *nobody has
launched the window* — and it took launching it to find them.

They are replaced by the application's own modal: one surface, one at a time,
promise-based, Escape cancels and Enter confirms because those are the keys the
dialogs it replaces already taught. Focus moves in on open and **returns to
whatever had it on close**, since a modal that strands keyboard navigation is a
regression in an application that is keyboard-first. The text variant validates
before resolving, so an empty name is refused in the dialog rather than by a
round trip to the core.

The native file picker stays native. Choosing a folder is the operating system's
job, and that is the one dialog `@tauri-apps/plugin-dialog` should own.

**`tools/no-blocking-dialogs.sh` fails the build if they come back**, in
`npm run lint`, in `npm run build`, in `tools/check.sh` and in CI. It has **no
exclusions** — which is why the file that documents the ban does not spell the
tokens, rather than exempting itself. The rule was checked by putting one back:
it bit.

## 0.9.3 - the git hooks are regenerated from repodocs

Both hooks of the standard are rewritten from repodocs, and `tools/release.sh`
with them when it came from there. `commit-msg` checks the shape of the subject
(`X.Y.Z - description`), refuses a Conventional Commits prefix and a vague
message, **and checks that the subject's `X.Y.Z` is the version this commit
carries in `version.md`**. `pre-push` compares the local `version.md` against
the remote default branch for a repeated or a backwards version — **only when
the push actually updates that branch**, so a branch deletion, a tag and a topic
branch pass through.

The hook does **not** check the language and could not: what it measures is the
shape and the number.

Escape hatch, declared in both: `NOTES_NO_HOOK=1`. In a fresh clone, enable them with
`git config core.hooksPath tools/git-hooks`.

## 0.3.2 - the queue rule arrives as a regenerated block, and stops being local

This repository decided two things on the day its queue was emptied and
restored: an item leaves `.continue/` only when the thing has been built
([ADR-009](docs/decisions.md)), and the queue is written in Portuguese
([ADR-010](docs/decisions.md)). Both were written as **local exceptions**, placed
deliberately outside the marked echo blocks so a fleet pass would not erase them.

The fleet adopted both the same day, as ADR-021 and ADR-022 in
[samirhvbr/repodocs](https://github.com/samirhvbr/repodocs). What was an
exception is the norm, so keeping a local copy of it would be the thing the
standard forbids: one rule with two sources, and no way to tell which is stale.

The rule now arrives in the new **`QUEUE-RULE`** block — the single exit
condition, the definition of *produce*, the bound on the half-a-page rule that
authorised the deletion in the first place, and the sentence that is the actual
instruction: **never empty this folder as tidying**. `LANGUAGE-RULE` and
`COMMIT-RULE` are regenerated in the same pass; the language block now names
three carve-outs, the third being this queue.

Why the block matters more than the correction it carries: on 07/09/2026, of the
52 repositories in the fleet, **2** carried any version of the queue rule and
**35** never mention `.continue/` in their agent instructions. It had never been
an echo block — it lived in the skeleton's `CLAUDE.md`, which is copied once at
creation and never regenerated. This repository was created from that skeleton
hours before the fix, which is precisely why the fix had to become something that
travels.

The two local ADRs stay as the record of **where** the decision was made, each
carrying a note that the fleet adopted it. The block is the source if they ever
disagree. Two details this repository holds that the fleet rule does not spell
out survive in prose: `.continue/README.md` is the folder's index and stays
English, and writing the `docs/` page in English is part of checking that the
thing was actually built.

## 0.2.0 - record the product scope and roadmap in docs/

The scope arrived as a 1 338-line draft in `.continue/`, written in Portuguese.
Two rules in this repository say it cannot stay there: a queue item that needs
half a page belongs in `docs/` with a pointer left behind, and everything in the
repository is written in English (US). This commit lands the first half of that
conversion — [docs/product.md](docs/product.md) and
[docs/roadmap.md](docs/roadmap.md).

`product.md` is the definition: local-first, a user-chosen folder as the
workspace, `.md` files on the filesystem as the source of truth, one dark theme,
CodeMirror 6, Source/Preview/Split with Live Preview explicitly deferred, and the
list of what the first version does not do. The promise it exists to protect is
that the files belong to the user rather than to the application — everything
else in the document is downstream of it.

`roadmap.md` is the order: seven product milestones from a desktop editor to an
MCP server. Its ordering constraint is that **each stage is useful on its own** —
someone who stops receiving updates after the first one still has a working
Markdown editor. It also states in its own header that its stage numbers are
product milestones and not repository versions, because `0.3` there and `0.3.0`
in `version.md` are otherwise going to be read as the same thing.

This is a `Y` bump rather than a `Z`: the repository went from having no product
definition to having one, and every later decision is measured against it.

## 0.2.0 - record the architecture in docs/

[docs/architecture.md](docs/architecture.md), the second half of the scope
conversion. It opens with the layering rule — Markdown is the source of truth,
SQLite is index and cache, the server is sync, REST is integrations, MCP is
agents — because that is the rule every later proposal gets checked against, and
the two forbidden shapes (SQLite as the only copy of a note; a proprietary
format with a Markdown export bolted on afterwards) are written down as
forbidden rather than left to be inferred.

Two things in the draft contradicted each other and are resolved here. The
`apps/` + `crates/` + `server/` layout and the `src/` + `src-tauri/` layout are
not two proposals: the second is what lives *inside* `apps/notes-app/`. The page
states both levels together, and adds the rule that makes the split worth
anything — **the Rust logic lives in `crates/`, and `src-tauri/` stays a thin
shell with no business logic**, which is what lets `server/` reuse the core at
milestone 0.5 instead of extracting it under pressure.

The filesystem abstraction is documented as existing from milestone 0.1, when it
will have exactly one adapter behind it. That looks like premature generality, so
the page carries the reason inline: "a folder the user picked" is a desktop
concept that iOS does not have, and finding that out after the UI has been
written against local paths is a UI rewrite.

The sync section states the constraint that shapes the data model years before
sync is built — `modified_at` alone cannot synchronise anything, because clocks
disagree, filesystems round timestamps differently, and a restored backup
rewrites them all.

## 0.2.0 - record the founding decisions as ADRs

Eight ADRs in [docs/decisions.md](docs/decisions.md), replacing the skeleton's
template. They exist so that the expensive parts of the scope are not
re-litigated by the next session — each one carries the reason and, more
importantly, the cost.

ADR-001 is the load-bearing one: Markdown files on the filesystem are the source
of truth, with no proprietary format at any point. It is recorded with what it
gives up — sync gets harder, indexing must be incremental, writes must be atomic
because the file is the only copy — because a decision that lists only benefits
has not been thought through.

The rest: Tauri 2 over Electron and Flutter, with the platform-webview tax
stated; Rust logic in `crates/` with a thin `src-tauri/`, so `server/` can reuse
the core at 0.5 without an extraction under pressure; `.notes/` restricted to
data that can be rebuilt, with the "delete it — did the user lose anything they
wrote?" test that keeps it from silently becoming the proprietary store ADR-001
forbids; sync deferred but its identity model protected, because an app built on
path + `modified_at` cannot be given sync later, only rewritten; Git dropped as
a dependency; no network port opened by default, since an editor that quietly
listens on a laptop joining untrusted networks is not a default worth shipping;
and desktop before mobile, with the filesystem seam carried from 0.1 so that 0.4
is an adapter rather than a rewrite.

ADR-006 is the one that records a reversal: the project was first sketched as a
Markdown editor with a public Git repository attached, and local-first replaced
it. Written down as a decision rather than dropped, so the idea does not come
back as a suggestion.

Also fixes a section reference in `roadmap.md` that pointed at
`architecture.md#3` when the filesystem abstraction is §4 — stale in the same
pass that created it.

## 0.3.0 - an item leaves the queue only when it has been built

`0.2.1` put the deleted drafts back. This writes down the rule that would have
stopped them being deleted, because an override nobody wrote down is not an
override — it is a mistake waiting to be repeated by whoever reads the rules and
obeys them.

**[ADR-009](docs/decisions.md#adr-009--an-item-leaves-continue-only-when-it-has-been-built):
an item leaves `.continue/` when the thing it describes has been BUILT** — not
when it has been documented, decided, translated or written up. A queue note
reading "a black screen with a yellow ball in the middle" stays in the queue
until that screen exists and works. **Size is never a reason to move an item
out**, which is the second half of the override: the fleet rule sending a
half-page queue item to `docs/` is exactly the rule that was followed into the
`0.2.0` mistake, and a 1 300-line specification stays in the queue while its code
does not exist. And nothing leaves the queue before it has been committed — the
operational half, which would have made `0.2.0` cost a `git revert` instead of a
reconstruction from memory.

The ambiguity that caused it is one word, and the ADR names it: the fleet
convention says a document moves to the record when it describes "something that
already exists", and *exists* was read as the definition existing rather than the
thing existing. Under the first reading, describing something well is what makes
it real. The failure is worst on a new project and that is not incidental — on
day one everything is words and nothing is code, so a rule that retires an item
once its text is tidy retires the whole queue. Which it did: four open items to
zero, with no application code written.

Golden rules 1 and 2 in `CLAUDE.md`/`AGENTS.md`, the "how it works" list in
`.continue/README.md` and the "where a new document goes" table in
`docs/README.md` all said the old thing and now say this one, each pointing at
ADR-009. Three files repeating a rule is worse than one when they disagree, and
they disagreed with the owner's intent in the same direction, which is how the
mistake looked correct at every checkpoint.

This is a `Y` because an ADR that overrides a fleet convention now counts as one.
That trigger did not exist before this commit and is added by it — a repository
quietly diverging from the fleet is exactly the change that has to be visible in
the version history, and `Z` would have buried it.

## 0.3.0 - write the queue in Portuguese and translate on the way out

[ADR-010](docs/decisions.md#adr-010--continue-is-written-in-portuguese-everything-else-is-english):
`.continue/` is written in Portuguese, and translation to English happens at the
moment the material leaves the queue — which, per ADR-009, is the moment the
thing has been built. Everything else is unchanged and stays English (US):
`docs/`, commit messages, pull requests, issues, code comments, changelog
entries, release notes.

The queue is where the owner thinks before anything exists, and a second language
is a tax on precisely the part of the work least able to carry one. It was also
part of the `0.2.0` argument for emptying the queue — "it is in Portuguese" read
as a defect to fix rather than as the queue working correctly.

**The exception is written outside the `LANGUAGE-RULE` markers, and that placement
is the point of the commit.** That block is a marked echo regenerated from
repodocs; an exception written between the markers is erased by the next fleet
pass with nobody noticing, leaving a repository whose stated rule contradicts its
practice. The precedent is `BLUE3-INTRANET`, whose language exception sits
outside the block for the same reason. The new section says so in its own first
line, so that a later reader tidying the file does not move it inside.

`.continue/README.md` stays in English and now says why: it is the folder's
index, not queue material.

## 0.9.2 - the acceptance document says the matrix is green, because now it is

`ACCEPTANCE-0.1b.md` was written while CI was still red and said so: *"no CI run
exists for this milestone yet"*. The `0.9.1` fix made the matrix green on all
four platforms, which made that line wrong an hour after it was written. It now
says what happened, including that the Linux leg had been red since `0.7.5` —
scope §19's *"sem verde nos quatro, marco desktop não fecha"* is satisfied by
that run, not by a document claiming it.

## 0.9.1 - the ENOSPC test needed the one privilege the runner has

CI had been red on the Linux leg since `0.7.5`, and on that leg alone: Windows,
macOS, Arch, the contracts job, the frontend and the crash loop were green
throughout. The failing step was the full-disk test, with
`unshare: write failed /proc/self/uid_map: Operation not permitted`.

**It is the case the script already anticipated, arriving from the machine
nobody expected it from.** Ubuntu 24.04 ships
`kernel.apparmor_restrict_unprivileged_userns=1`, so a GitHub runner cannot
create the user namespace the test mounts its `tmpfs` in — and it is also the
one machine in this project with passwordless `sudo`. `tools/enospc.sh` now
tries both, in the order that needs the fewest privileges: the namespace first,
because that is what a developer runs and it leaves nothing mounted anywhere,
then `sudo -n mount -t tmpfs`. `sudo -n` never prompts, so a machine with a
password falls through rather than stopping the gate to ask for one.

**And a skip is now a failure where it matters.** `NOTES_REQUIRE_ENOSPC=1` is
set on the CI leg: a runner that lost both mechanisms would otherwise skip in
silence, which is precisely the failure mode D-01 was written to refuse two
versions ago — a check that quietly opts out is not a check.

**The lesson is mine and it is worth writing down.** Six commits went out
without the CI result being read, on the assumption that a green local gate
meant a green matrix. It did not, for a reason the local gate structurally
cannot see: the developer machine allows the thing the runner forbids. The
matrix exists for exactly that, and it is only useful if somebody looks at it.

## 0.9.0 - milestone 0.1b ships: search in the file, the acceptance document, and five ADRs

The last scope item and the record. `@codemirror/search` gives `Ctrl+F` and
`Ctrl+H` **on the buffer in front of the user** — which is why it searches what
is being typed rather than what is saved. Global search is 0.1c and is a
different thing entirely: it scans the workspace in the core, streams results
and is cancellable.

[docs/ACCEPTANCE-0.1b.md](docs/ACCEPTANCE-0.1b.md) puts each of scope §17's five
criteria against a named test or a documented manual step, and says plainly
where a criterion is met **in the core** and unobserved in the window. All five
are met; criterion 1 is qualified, because nobody has watched a tab update.

**262 Rust tests and 8 `vitest` cases.** The `fixtures/xss/` census renders every
payload under all four combinations of `raw_html` and `remote_images`, so adding
one is enough and forgetting to write a test for it cannot make it pass.

**What the preview IR costs, measured rather than argued.** Turning `Rendered`
into JSON is 6–9% of render-plus-serialise at any size a person writes and 18%
at the 5 MiB edge case: not where the time goes, and nothing was engineered
around it. What the profile *did* say is that the cost tracks element count
rather than bytes — 1 MiB of dense HTML costs about what 5 MiB of prose does —
and that is written down so the next person measures the right thing.

Five ADRs, for the decisions that outlived the milestone that made them:
**ADR-025** golden corpus, and why blessing is not accepting; **ADR-026**
reconciliation driven from what vanished, and a full scan that announces no
creations, amending ADR-014; **ADR-027** not being able to watch is a state of
the workspace rather than a failure; **ADR-028** a resolution keeps the version
it did not choose; **ADR-029** `mailto:` and every scheme but `http(s)` render
as text.

`docs/ARCHITECTURE.md` is `ACTIVE` for §§7–10 — they describe code that exists
now — and §17.1 gained four more rows where the implementation and the
Portuguese scope had to be reconciled out loud.

**What is not done, in one line: nobody has launched the window.** The
interface compiles, typechecks, bundles, and has tests over the one piece of it
that is logic rather than markup. Everything else about it is unobserved, and
`ACCEPTANCE-0.1b.md`'s *Not verified* section lists it item by item rather than
leaving it to be discovered.

## 0.8.5 - the watcher, reconciliation, and identity that survives an external rename

`ARCHITECTURE.md` §8 and §9 in code, and the last three 0.1b criteria that can
be asserted without a window.

**`stat`, then hash. Everything else is a hint.** A watcher event, a window
regaining focus, a tab switch and the 5 s poll all arrive at the same function
as *these paths may have moved, go and look*. Nothing believes an event; size
and mtime alone never conclude anything (scope §12), and reconciliation never
writes.

**The self-write filter is armed before the write, not after.** Otherwise there
is a window exactly as long as the write in which the application's own autosave
comes back as an external change. It is consumed on its first match and expires
after two seconds, so **someone else writing the same bytes right afterwards is
still seen** — there is a test named after that, because it is the half that is
easy to get wrong.

**Identity correlation is driven from what vanished.** §9 phrases it as
*"appeared := disk paths not in registry"*, which here is nearly every file —
the registry is lazy. Driving it from the vanished side computes the same answer
and costs nothing on every tick but one. Rule 1 is a unique native id, rule 2 a
unique non-empty hash — **a zero-byte file is never correlated**, because every
empty file has the same digest — and rule 3 is a new identity, because
re-identifying a note is cheaper than attaching one to the wrong history.

**Two design defects the tests found before the push.** A full scan reported
every note nobody had opened as `Created`, which on a real workspace means
announcing a thousand creations each time the window regains focus, and which
blew the hash budget with events that were not changes; `Created` is now a
hinted-path signal only (D-13). And the editor could not accept a reload at all:
the CodeMirror view is keyed on the note id, so replacing `doc.text` did nothing.
It now takes the new text in **one transaction** with the selection clamped and
kept — rebuilding the view would throw away the undo history and put the caret
at the top of a note the user was reading half-way down — and the transaction is
annotated so the update listener does not mark the buffer dirty and autosave
text the user never typed.

**Not being able to watch is a state of the workspace, not a failure.**
`watch()` returns a `Watch` with a `degraded` reason rather than an `Err`: a
network mount, a SAF tree and a kernel out of inotify watches all mean *poll
instead and say why*, and the inotify case says it with the `sysctl` that raises
the limit. The interface shows the reason and keeps working.

The hash budget is 50 files per tick with the rest queued, and a test asserts
the queue drains and that every change is reported **exactly once** — a budget
that silently dropped work would be worse than no budget.

`notify` 8.x, not the 9 release candidate, and the debouncer is ours (D-12).

## 0.8.4 - rename, move, duplicate and delete, and the identity that survives them

The four entry operations of 0.1b, and the criterion they exist to satisfy:
**a rename performed by the application never resets a tab.**
`ARCHITECTURE.md` §9 says a rename the app performs never enters identity
correlation — it updates the registry directly — and `Registry::repath` is that
sentence in code. Renaming a folder carries every note beneath it, because the
notes inside a folder someone renamed did not change and giving them new ids
would lose their history for a reason invisible to the person who did it. The
prefix test is on a path boundary, so `pasta2/` is not dragged along by a rename
of `pasta/` — a naive `starts_with` corrupts the registry silently, which is why
there is a test named after it.

**Duplicate never overwrites**, per scope §17: `create_new` throughout, a copy
gets an identity of its own because a new file is a new note, and the name is
`nome (copy).md` → `nome (copy 2).md`, in ASCII and the same in every language
(D-10). **Move refuses a collision and names what is in the way**, which is what
lets the interface ask rather than guess, and a folder cannot be moved inside
itself.

**Delete has a trash now**, and says which of the two things happened. `trash`
is a dependency from this commit; `caps.trash` decides whether to try, and a
failure — no bin on a removable stick, no session bus in a container — degrades
to a permanent delete with a *different sentence in the interface*, never a
silent one (scope §7.7, D-11). The notes leave the registry; **their drafts do
not**, because a note deleted while it held unsaved edits is precisely the case
where the draft is the only copy of them.

The tree grew a context menu for the four, and the frontend a `notice` channel
for a thing that went right — an error banner is the wrong shape for "moved to
the trash, so it can be put back".

**The Windows cross-check earned its place again.** `tools/check.sh` failed on a
`let mut f` that is only mutated inside a `#[cfg(unix)]` block: fine on Linux,
`-D warnings` on Windows, and invisible to every other step of the gate. That is
the third time this class of defect would otherwise have been found by CI, and
the first time it was found before the push.

## 0.8.3 - Source · Preview · Split, and the conflict screen 0.1a shipped without

The interface catches up with the core. `Ctrl+E` cycles Source → Preview →
Split, the mode is remembered per workspace in `session.json`, and the preview
renders through `markdown_render`.

**`innerHTML` is assigned in exactly one component, and the comment above it
says why.** The string came from `notes-markdown` behind `ammonia`; nothing else
in this application may assign it, and that component must never render a string
it did not get from that command. A click inside the preview never navigates: a
relative link opens the note in-app, an anchor scrolls, an `http(s)` link goes to
the operating system's browser through `shell_open`, which **checks the scheme
again in Rust** — the capability is what the WebView may ask for, and the check
is what the process will do. Blocked remote images are named in a banner with a
button that turns them on for this workspace, because a silent gap is worse than
a visible one.

**The comparison screen is the piece 0.1a left out.** The core suspended
autosave and wrote the draft; the interface said only that something had
happened. Scope §12's four resolutions are now all reachable — *comparar* as a
screen (`ARCHITECTURE.md` §17.1: it changes nothing on disk and reads two
strings the frontend already holds), and the other three as one call to
`conflict_resolve`. It reads the disk version with `note_reload`, which touches
no buffer, and shows the two side by side with the differing lines aligned.

**The diff is sixty lines of this repository's own**, for the reason
`notes-markdown` writes its own slugs: a dependency that changes how a diff
aligns changes what a user sees at the one moment they are deciding which
version of their work to keep. Common prefix and suffix are trimmed first, so a
one-line change in a 6 000-line note is cheap; past four million cells the
alignment is skipped and the differing middle is shown as one block, **loudly**,
because a window that stops responding at that moment is worse than a coarse
answer. Eight `vitest` cases hold it, and the one that matters asserts no line
from either version is ever lost. `npm test` joins the local gate and CI.

A mixed-EOL note now offers `note_convert_eol` in its read-only banner rather
than only explaining why it cannot be edited.

Two things were deliberately **not** done on the way past, and both are in
`docs/DECISIONS-0.1b.md`: the preview serves raster images only, because
"probably safe because of a browser rule" is not the same as safe by decision
(D-08); and `shell().open` stays deprecated rather than migrating to
`tauri-plugin-opener`, because that means a new dependency and a capability
edit, and scope §19 sends both to the owner (D-09, with the whole change written
out for whoever makes it).

## 0.8.2 - the three ways out of a conflict, each keeping the version it did not choose

Scope §12 lists four resolutions — *comparar · manter o meu · usar o do disco ·
salvar como `nome (local).md`* — and `ARCHITECTURE.md` §17.1 had already settled
that **compare is not one of them**: it changes nothing on disk and reads two
strings the frontend is already holding, so it is a screen rather than a
command. The other three are `conflict_resolve` now.

**The rule they share is the reason the module exists.** Resolving a conflict is
the one moment a user can lose a morning by answering a dialog quickly, so the
version they did not choose is written to `conflicts/` *before* anything else
happens: `KeepLocal` snapshots the disk and then overwrites it, `UseDisk`
snapshots the buffer and then throws it away, `SaveAsCopy` writes the buffer to
`nota (local).md` and leaves the note exactly as the other program wrote it —
numbered `nota (local 2).md` when that name is taken, because `create_new` never
overwrites and a second conflict has to have somewhere to go.

`KeepLocal` passes no `base_rev` to the write, deliberately: the user has just
been shown both versions and said which one wins, and re-checking the revision
there would refuse the very thing they answered.

**The removal case both ways.** A note deleted externally with a dirty buffer:
`KeepLocal` recreates it — the only circumstance in which this application
recreates a path it did not create, and only because the user asked — and
`UseDisk` accepts the deletion, keeps the buffer in `conflicts/` anyway, and
returns `NotFound` so the tab can close.

`note_convert_eol` arrives with them, and it is the one command in this
application that rewrites a file the user did not edit. It exists for one
situation: a mixed-EOL note opens read-only, and without a conversion the
application would be refusing to edit a file while offering no way forward. The
old bytes go to `conflicts/` first. It found a real trap on the way —
`TextProfile::detect` normalises `\r\n` only when the *whole* file is CRLF, so a
mixed file reaches the caller with its endings intact and the flattening has to
happen in the conversion itself.

`conflicts/` follows §4.3: `<NoteId>/<iso-ts>-<local|disk>.md` with a sidecar,
colons stripped from the timestamp because they are legal on ext4 and illegal on
NTFS. Resolved snapshots are pruned after `files.conflict_retention_days` (30,
`serde(default)` so an older `settings.json` still loads at schema 1), **`0`
means keep them** rather than delete them all, and the 200 MB warning says so
and deletes nothing — making room by throwing away the only copy of something a
user wrote is the failure the directory exists to prevent. An *unresolved*
conflict is a draft, and nothing prunes those.

`note_reload` and `note_close` land with them: reload re-reads from disk and
lets the caller decide when a buffer may be replaced, and close lifts the
suspension while **leaving the draft alone** — a draft outlives its tab.

## 0.8.1 - the preview IR crosses the IPC, and the measurement that says it may

`markdown_render`, `markdown_outline` and `markdown_trust_set` are commands
now, and `notes-asset://` is a registered scheme. That completes
`docs/ARCHITECTURE.md` §10 in code: **sanitized HTML crosses for the preview, a
slim `Document` crosses for the outline, and no AST crosses at all.**

**The asset scheme is a second entry point into the workspace, and it resolves
nothing itself.** The WebView has no filesystem capability, so a note that shows
a picture cannot reach for the file; the preview writes
`notes-asset://<workspace-id>/<relative/path.png>` and the handler in
`src-tauri/src/asset.rs` hands the path to `notes-core`, which applies the same
root jail as every command — the string check, then `notes-fs` re-resolving each
segment and refusing a symlink. `tests/preview.rs` proves that with a symlink out
of the root and asserts the file it pointed at is untouched. Only raster image
types come back; a `.txt` and a `.md` are both `Unsupported`, so the preview
cannot be used to read one note into another. Responses carry
`default-src 'none'; sandbox` and `nosniff`, and a failure has an empty body —
a message would say whether a path exists outside the root, and that is not a
question the preview is entitled to ask.

**Raw HTML and remote images are per workspace**, in the registry rather than in
the global settings: trusting the notes in one folder says nothing about
another, and the setting survives a restart because that is the only reason to
persist it at all.

**The serialisation cost was measured, not guessed** — `cargo test -p notes-core
--test cost -- --ignored --nocapture`. Turning `Rendered` into JSON is **6–8%**
of render-plus-serialise for anything of a size a person writes, and 18% for the
5 MiB edge case; it is not where the time goes, and nothing was optimised for
it. What the profile did show is that `ammonia`'s builder was being assembled
per render: 0.385 ms → 0.293 ms for a 337-byte note once it is built once. That
is a small number and it is stated small, because the point of measuring first
is being able to say which numbers are real.

## 0.8.0 - notes-markdown reads the two fixture corpora it was written against

`fixtures/xss/` was committed at 0.1a with a README calling each file *"an
assertion, not a sample"*, and nothing read it. This commit is the thing that
reads it, and the corpus it needed beside it.

**The fixtures came first, and that mattered.** `fixtures/markdown/` holds
seventeen inputs, each with the exact HTML and the exact `Document` it must
produce, compared byte for byte; `fixtures/markdown/README.md` states the
contract one row per file *before* any of it existed. The goldens are generated
with `NOTES_BLESS=1` and then **read against that table** — blessing is not
accepting (docs/DECISIONS-0.1b.md D-04). That reading caught four defects the
suite would otherwise have frozen as decisions: `outra.md#uma-secao` lost its
fragment; `<alguem@example.com>` was classified as a relative path and rendered
as a note link to a file with an `@` in its name; a refused image dropped its
alt text; and a bare `https://…` in prose was not linkified, which scope §8.1
lists among the GFM features. All four are fixed and pinned.

**The XSS corpus is now a census.** Every `.md` in `fixtures/xss/` is rendered
under all four combinations of `raw_html` and `remote_images` and checked
structurally — tags and attributes read back out of the sanitized output, never
substrings. `safe-in-code.md` is why: it must render `javascript:alert(1)` **as
text**, so a suite that greps for `javascript:` asserts the opposite of the
requirement. Adding a payload to the folder is therefore enough; forgetting to
write a test for it cannot make it pass. Each file also keeps a named test of
its own, asserting it was refused for the right reason and that the rest of the
note still rendered.

**Two layers, on purpose.** The rewrite pass in `url.rs` decides what every
destination may become — schemes, root escapes, the raster-only `data:`
allowlist that excludes `image/svg+xml`, remote images blocked and named rather
than silently missing. `ammonia` then applies a closed allowlist that knows
nothing about notes, forces every `<input>` to be a disabled checkbox, and
permits exactly three `style` values, on table cells only. A mistake in one has
to coincide with a hole in the other to reach a user.

**`mailto:` renders as text**, and so does an email autolink. Scope §8.4 says
*"outros esquemas recusados"*, and `shell:allow-open` is restricted to `http`
and `https` — a `mailto:` anchor would be a link that does nothing when clicked.
Widening that capability is the owner's act, not the renderer's (D-06).

**One 0.1a defect surfaced on the way and is fixed here.** `RelPath::root()`
serialises to `""` and `TryFrom<String>` refused `""`, so the type could not
deserialise a value it produces. `tree_list` takes a `RelPath`, and the
frontend's `ROOT` is that string: every listing of the workspace root was
rejected by argument deserialisation before the command body ran — the sidebar's
first call on every launch. `parse` still refuses an empty name; only the wire
form accepts it (D-05). Three tests hold the line.

`docs/ARCHITECTURE.md` §10 is rewritten to describe what was built rather than
what was proposed. The generated-types check now covers `notes-markdown` and asks
two questions instead of one — `git diff` for a changed file and
`git ls-files --others` for an untracked one — because a type introduced by a
new crate arrives untracked, which is how eight new `.ts` files stayed invisible
to a green gate.

## 0.7.5 - the debt 0.1a left: the Windows check runs by default and the full disk is automated

Three things 0.1a left behind, cleared before any 0.1b feature so that the
milestone starts from a gate that is actually closed.

**The full-disk criterion is automated, and it is the one that mattered.**
[ACCEPTANCE-0.1a.md](docs/ACCEPTANCE-0.1a.md) §5 read *partly met*: `IoKind`
classified errno 28 in a unit test, but nothing exercised the path from a
filesystem that is really out of room to a visible error and a recoverable
buffer — the two steps in that gap being `write_atomic` returning `Err` at the
right moment and `settle` writing the draft instead of propagating. The document
called automating it "a decision about CI privileges", because the manual recipe
wanted `sudo mount -o loop`. It does not need one: an **unprivileged user
namespace** can mount a `tmpfs`, and a size-capped `tmpfs` over its limit returns
ENOSPC exactly as a full disk does. `tools/enospc.sh` builds that namespace and
runs `notes-core`'s `tests/enospc.rs` inside it, in the local gate and on the
Linux leg of CI, with no privileges at all and no mount left behind anywhere.
The test asserts the whole path: `WriteFailed { kind: DiskFull }` rather than an
`Err`, the note byte-identical afterwards, no `.tmp` left in the user's folder,
the draft holding the buffer verbatim, and reopening the note offering it back.
Criterion 5 is now **met**; the reasoning and the loopback alternative it
displaced are [DECISIONS-0.1b.md](docs/DECISIONS-0.1b.md) D-03.

**The Windows cross-check runs by default.** `tools/check.sh` gained it at
`0.7.3` and then skipped it whenever `x86_64-pc-windows-gnu` was not installed —
so the one check that would have caught both Windows compile failures was
missing on exactly the machines that had never added the target. It now installs
the target once and runs. `NOTES_NO_WINDOWS_CHECK=1` opts out deliberately; a
machine with no `rustup` gets a loud warning rather than a failed gate, because
refusing to run the test suite over a cross-compilation concern trades a real
check for a hypothetical one (D-01).

**And the queue index points at a file that exists.** `.continue/README.md`
linked `ARCHITECTURE.md` at the repository root, where it has never lived. That
is a pointer, not queue material — the README says of itself that it is the
folder's index — so repairing it is not the tidying the queue rule forbids
(D-02).

`docs/DECISIONS-0.1b.md` opens with these three, in the same shape the 0.1a log
uses: what was decided, which gap it closed, and what to do instead if the owner
disagrees.

One stale transcript went with them: `ACCEPTANCE-0.1a.md` §3 still quoted 22
edge-case files saved unchanged, from before D-20 and D-23 removed the names no
target filesystem could hold. The corpus is 21 files — 17 saved unchanged, 4
read-only — and 227 in total across both corpora.

## 0.7.4 - the CI matrix is green on all four platforms

Ubuntu, macOS, Windows and Arch, plus the contracts job, the frontend and the
1000-round crash loop. `docs/ACCEPTANCE-0.1a.md` said the matrix had not run;
now it has, and what it found is written down there as a table.

**Not one of the four rounds was a failing test.** Every problem stopped the
build or the checkout before a test could execute — an unclonable repository on
Windows, a corpus APFS cannot materialise, two compile failures behind `cfg`
walls Linux cannot see. That is the argument for the matrix in one line, and it
is why "it passes here" was never the same claim as "it passes".

What remains asserted rather than observed is narrower now: the suite runs on
ext4, APFS and NTFS, so §11's rows for SMB, NFS, exFAT and FUSE are the ones
still unproven.

## 0.7.3 - check the Windows target locally instead of discovering it in CI

The third CI round failed on Windows for the third time in a row, and for a
class of reason Linux cannot see: a helper used only under `#[cfg(unix)]` is
**dead code** on Windows, and `-D warnings` makes that a build failure. Not a
test failing — the crate does not compile, so nothing runs. Three symbols were in
that state (`IoKind`, `drafts_dir`, and two symlink tests that kept a fixture
they no longer used), each behind a `#[cfg(unix)]` block inside an otherwise
portable function.

They are fixed by making the whole test Unix-only where that is what it is,
rather than by threading `cfg` through a function body — which is also more
honest: `no_command_accepts_a_path_outside_the_root` was two tests, a string
half that needs no disk and a symlink half that does, and splitting them says so.

**`tools/check.sh` now runs the gate including
`cargo clippy --target x86_64-pc-windows-gnu`.** It costs one `rustup target
add`, type-checks without linking, and would have caught all three of these plus
the unstable-API failure at `0.7.2` — about thirty minutes of CI, found in
seconds. The step skips with a message when the target is absent rather than
failing.

123 tests; native and Windows targets both clean.

## 0.7.2 - the second CI run found two more, and both were the product

The first pass fixed the harness. This one is code and corpus.

**Windows did not fail a test — it failed to compile.**
`MetadataExt::volume_serial_number` and `file_index` sit behind the unstable
`windows_by_handle` feature, so `native_id` could never have built on stable.
It now returns `None` there and `Caps::LOCAL.native_id` is `cfg!(unix)`, which is
the degradation `ARCHITECTURE.md` §11 already specifies: correlation falls back
to the content hash and yields a new `NoteId` in more ambiguous cases — the safe
direction, and it costs nothing at 0.1a because nothing correlates yet. Doing it
properly needs `GetFileInformationByHandle` and belongs with its first consumer
at 0.1b.

**macOS found the general form of the trailing-dot defect.** Two more sets of
names cannot be materialised on APFS: `Duplicate.md` and `duplicate.md` are *one
file* on a case-insensitive filesystem, so git checks one out over the other and
the survivor reports as modified on a clean clone; and APFS normalises to NFD, so
the NFC name in the index and the NFD name on disk disagree, leaving one missing
and one untracked. Both pairs are gone from the committed corpus and are created
at runtime by tests that **ask the filesystem what it does** rather than assume —
the case test asserts a collision only where the root folds case.

The rule generalises, and is written down: a committed fixture must be
materialisable on every platform in the matrix. What tests a thing a filesystem
cannot represent is built at runtime.

That is three defects in two runs that only a real matrix could find, and two of
them made the repository unusable on a platform before a single test executed.
122 tests; `fmt`, `clippy -D warnings`, the workspace suite and the generated
types are all clean here.

## 0.7.1 - the CI matrix ran for the first time and found four real problems

Three were the test harness. **One made the repository unclonable on Windows.**

`actions/checkout` did not fail a test — it aborted:
`error: invalid path 'fixtures/edge-cases/trailing-dot.md.'`. A file whose name
ends in a dot cannot exist on NTFS, so git refuses the entire checkout. Every
Windows contributor's first command would have failed, and **no test could have
caught it, because no test ran.** The file is gone from the committed corpus; the
rule it covered is a unit test, and the "an existing odd name is listed, never
renamed" half is created at runtime by a test that skips on Windows.

That defect exposed a gap: scope §7.6 requires a **new** name to follow a
portable rule and nothing implemented it. `portable_name` now refuses
`\ / : * ? " < > |`, control characters, a trailing dot or space, and the Windows
device names — checked against what the user typed **before** `.md` is appended,
because otherwise `trailing-dot.` becomes `trailing-dot..md`: legal, and not what
they asked for. A name already on disk is still never touched.

The other three, each recorded with its alternative:

- **Arch** runs its container as root, and root ignores permission bits, so the
  denial the write-failure test needs could not be arranged and it observed a
  successful write. It now skips as root and says so. Asserting anyway would have
  made it pass for the wrong reason everywhere else and mean nothing there.
- **macOS** resolves `/var` to `/private/var`, so a temp directory has two names
  and the registry stores the resolved one; the test was comparing the name it
  handed in.
- **contracts** ran `cargo test --workspace`, which builds the Tauri application
  and needs GTK, WebKit and glib — on a job whose entire point is that it needs
  none of them. It now builds only the two crates that export types.

Ubuntu, the frontend and the 1000-round crash loop were green on the first run.

## 0.7.0 - milestone 0.1a ships: ARCHITECTURE.md is ACTIVE and its decisions are ADRs

A workspace is a folder, its `.md` files are notes, and editing one is safe
against everything else on the machine that might touch it at the same time.

**Seven of the eight acceptance criteria are met, one is partly met, and
`docs/ACCEPTANCE-0.1a.md` says which is which** — each against a named test or a
documented manual step, with the measurements rather than assurances:

- a 10 000-note, 197 MiB workspace **opens in 226 µs and its whole tree lists in
  37.5 ms**, two orders of magnitude under the one-second criterion, with the
  registry still empty afterwards — proof that listing assigned no identity and
  therefore hashed nothing;
- **1000 kills mid-save, 0 failures**, no truncated or empty note;
- **228 files opened and saved unchanged with `git status` clean**, in the
  criterion's literal form, plus a hermetic copy-based version that cannot dirty
  the repository;
- the external-append case, the path-escape cases and "opening a folder creates
  nothing" are all automated in the core, as the criteria require.

**The one that is only partly met is said so plainly.** Permission-denied is
automated and proven to leave a recoverable draft; **no test fills a
filesystem**, so the path from a real ENOSPC to a visible error is documented as
a manual step and listed as unverified. Automating it needs loopback privileges
in CI, which is a decision about CI rather than about this milestone.

`ARCHITECTURE.md` becomes `ACTIVE`, and the twelve decisions it introduced become
**ADR-013 … ADR-024**. Three are worth naming here. ADR-014 amends ADR-005 once
rather than twice, closing both readings of its Decision together: identity never
enters a note file, and the content hash is correlation rather than identity —
which is what keeps the promise that the app never writes what the user did not
type alive through 0.6, the milestone at which most note applications break it.
ADR-020 records that one command per operation was chosen over a single
`dispatch` on a capability argument, not a stylistic one: permitting `dispatch`
permits `delete`, and there is no way to grant half of it. ADR-021 records why
autosave and the base-rev guard could not ship apart — the moment autosave
exists, the app is writing to files that VS Code or an agent may be writing too,
and without the guard it overwrites them.

`docs/DECISIONS-0.1a.md` holds the nineteen calls the specification did not make,
each with the alternative if the owner disagrees. Two changed the design rather
than filling a hole: the case-sensitivity probe reads instead of writing, because
the mechanism specified would have created a file inside a folder that was merely
opened; and the temporary file has a deterministic name, because the crash loop
proved that random ones accumulate in the user's folder forever.

**Not verified, and not claimed: the window has never been launched.** Everything
above comes from the core and the corpus. The CI matrix — Ubuntu, macOS, Windows
and an Arch container against rolling `webkit2gtk-4.1` — has not run yet either,
so `ARCHITECTURE.md` §11's capability matrix remains a specification rather than
an observation. Milestone 0.0 stays open in `.continue/`, on hardware this
machine does not have.

A `Y` bump: a completed roadmap milestone.

## 0.6.1 - the Tauri shell, the typed IPC boundary, and the 0.1a interface

Nineteen commands, one per operation, each of them parse → call the core →
return. `src-tauri` holds no policy: a single `dispatch` command was rejected in
`ARCHITECTURE.md` §18.8 because Tauri's capabilities are per command, so
permitting `dispatch` would permit everything.

**The generated TypeScript found a real defect.** `mtime_ns` is around
1.7 × 10¹⁸ and `Number.MAX_SAFE_INTEGER` is 9.0 × 10¹⁵, so a nanosecond
timestamp sent as a JSON number is rounded by JavaScript — and it does not
merely display wrong. `BaseRev` travels back to the core on every save, so a
rounded timestamp would make the cheap check disagree with the disk on every
write and quietly send each one down the hashing path. It now crosses as a
string, with a test that asserts the exact round-trip above the safe integer.
No test that stayed inside Rust could have caught it.

The capability file grants `core:default`, `dialog:allow-open`, clipboard read
and write, and `shell:allow-open` restricted to `http(s)`. **No `fs:` permission
exists in it**, and CI greps for one — the check `ARCHITECTURE.md` §12 asked for
in as many words. A second job deletes `ipc/generated`, regenerates it and fails
on any diff, because Rust and TypeScript disagreeing about the wire while both
compile is the failure the generator exists to prevent. A third fails when the
two i18n catalogues do not carry the same keys, since a missing key is a blank
label in exactly one language.

The interface is the 0.1a list and nothing beyond it: welcome with recents,
lazy tree, CodeMirror 6, autosave with the base-rev guard, `Ctrl+S` as a flush
rather than the only path to disk, a draft banner, a conflict banner, and the
status bar carrying the seven states of scope §9 — each with a word and a glyph
as well as a colour, and `saved` set only from a `SaveResult`.

The store holds the stale-save guard: a save paints the tab clean only when the
`buffer_version` it returns still equals the current one, so an old save landing
after new keystrokes cannot mark the buffer saved. While a note is in conflict
the debounce writes to the **draft** instead of the note, which is the rule §5
states and which needs the command D-11 added.

The 1000-round crash loop passed here: no truncated or empty note, and temporary
files never exceeded one.

`cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo test
--workspace` and `npm run build` are all clean, and the CI matrix now runs them
on Ubuntu, macOS, Windows and an Arch container against rolling `webkit2gtk-4.1`.

## 0.6.0 - notes-core: the write protocol, drafts, the registry and the lock

117 tests, none of which needs Tauri or a window. Four of the eight 0.1a
acceptance criteria are now automated tests rather than intentions.

**The write protocol** is `ARCHITECTURE.md` §5 with `base_rev` explicit on the
wire. The order matters and is asserted: identical content is a no-op that never
moves mtime, so an unchanged save leaves `git status` clean; a disk whose bytes
already equal the buffer is *convergence*, not a conflict; a change in mtime with
an unchanged hash is a touch, and only a changed hash is a conflict. Size and
mtime never authorise an overwrite on their own.

**A failed write is a result, not an error**, and that was a real bug found by
writing the acceptance test first: propagating `Err` out of `save_note` skipped
the draft, so "disco cheio / permissão negada → buffer recuperável ao reabrir"
would have been false while the code looked right. The test denies write
permission on the directory and asserts the draft holds the buffer verbatim.

**`tools/crash-save-loop.sh` found a defect on its first run.** The note never
truncated across sixty kills — but every `SIGKILL` between the write and the
rename left a temporary file behind, and with a random suffix **they accumulate
in the user's folder forever**. No process cleans up after being killed, so the
fix is not cleanup: the temporary name is now deterministic, one per note, and
the next save overwrites it. The loop asserts that bound rather than asserting
zero, because zero is not achievable and a test that demands it would be
disabled within a week.

Seven more decisions in `docs/DECISIONS-0.1a.md`, each with its alternative. The
load-bearing ones: the registry is populated when a note is **opened** and never
by listing, because a `hash` per record plus population-on-listing would mean
reading every file in a workspace the criterion says must list in under a second;
`write_draft` exists as a command at all, because §4.2 wants a draft after 30 s
of dirty buffer and on exit while §5 gives the buffer to the frontend, so all
three rules were unimplementable; and `workspaces.json` gains `last_workspace`,
because picking the maximum `last_opened` is a tie-break invented at read time
that is wrong the moment two workspaces open in the same second.

State loading reads the `schema` before the body, so a file written by a newer
build is detected even when its shape no longer parses — that workspace opens
read-only and **nothing is overwritten**, with a test that asserts the bytes
survive.

A `Y` bump: a new crate.

## 0.5.0 - notes-model and notes-fs, with the root jail and the atomic write

Two crates, 76 tests, no Tauri anywhere near them.

**`notes-model`** is types and nothing else — the rule that makes the write
protocol testable against a fake filesystem later. `RelPath` refuses every escape
shape as a string and **never normalises**, because a normalised path is a string
that does not open the file the user has on any filesystem storing NFD;
comparison is `CompareKey`'s job, and it is a separate type so the two can never
be confused. `ContentHash` serialises as `b3:<hex>` — prefixed by the algorithm,
so changing hash one day is a migration rather than an ambiguity — and carries
the digest of the empty input as a constant, which `notes-fs` asserts against the
real hasher so the constant cannot rot.

`TextProfile` is where the byte policy lives, and where front-matter preservation
actually comes from: the editor only ever sees `\n` with no BOM, and `encode`
puts the file's own shape back, so YAML survives 0.1a because nothing rewrites
the buffer — not because a parser restores it. Mixed endings and invalid UTF-8
return a read-only reason instead of a lossy decode.

**`notes-fs`** is the seam. The root jail is two halves that fail differently:
`RelPath` refuses what can be seen in the string, and `LocalFs::resolve`
`symlink_metadata`s each segment as it appends it, because a symlink is a
perfectly well-formed relative path that resolves somewhere else. Both halves run
on **every** call — a root validated at open time says nothing about the path
being used now.

The atomic write is temp, fsync, mode copy, re-stat, rename, `fsync` on the
directory — the last one because without it the contents survive a power cut and
the name may not. `expect` re-stats immediately before the rename and returns
`Diverged` with **nothing written**; a test asserts the external content is still
there afterwards. A rewrite with identical bytes moves mtime and not the hash,
and the test for that is the one that keeps size-and-mtime from ever authorising
an overwrite on its own.

The case-sensitivity probe reads instead of writing (D-01): it flips the case of
one character of an existing name and compares `dev`+`ino`. Inconclusive resolves
to *insensitive*, and the asymmetry is the point — a missed fold refuses a
legitimate name, the opposite lets a create pass its collision check and
overwrite a note.

Five decisions the specification left open are in `docs/DECISIONS-0.1a.md` with
their alternatives: a typed `IoKind` so a full disk is distinguishable from a
denied permission by *code* rather than by a string the contract says not to read;
`watch()` answering `Unsupported` until 0.1b rather than pulling `notify` early;
a fixed table of byte shapes instead of a property-testing dependency; NFC and
case-folding implemented in-crate rather than widening the four-dependency list
`ARCHITECTURE.md` §2 fixes for `notes-model`; and `delete` reporting `Permanent`,
which scope §7.7 allows as long as the user is told, and which no 0.1a command
can reach.

A `Y` bump: adding a crate is one, per `docs/versioning.md`.

## 0.4.1 - build the fixture corpora, because no fixture means no test

Milestone 0.1a's acceptance criteria are almost all statements about a corpus:
list `fixtures/basic` and `fixtures/large` in under a second, kill the process
during a thousand saves against `large`, open and re-save every file in `basic`
and `edge-cases` and see a clean `git status`. None of those corpora existed.

`fixtures/basic/` — 200 notes over a nine-directory tree, plus the files that
must **not** appear in it: a `.txt`, a `.png`, a dot-file, and three ignored
directories. `fixtures/edge-cases/` — 26 files, one per hazard the byte policy
has to survive: LF, CRLF, CR-only, missing final newline, BOM with each ending,
mixed EOL, empty, whitespace-only, invalid UTF-8, a lone surrogate, valid and
malformed front matter, front matter that is not on the first line, tabs, a name
with a space, a trailing dot, a case collision, NFC and NFD names, and 5 MB.
`fixtures/xss/` — 18 files, each an assertion rather than a sample, with two that
must **survive**: the payloads inside a code fence have to render as text, and a
renderer that strips them there is rewriting what the user wrote.

Both committed corpora come from `tools/gen-fixtures.py`, which is deterministic
by construction — a blake2b stream keyed on the file's own path, never
`random` — so regenerating on a clean checkout leaves `git status` empty and a
review can see where each byte came from. `tools/gen-large.sh` generates the
performance corpus at 10 000 notes and 197 MiB and is never committed.

**`.gitattributes` marks the corpus `-text`, and without it the byte-preservation
criterion would be theatre.** The files under test deliberately carry CRLF,
CR-only and mixed endings; git's default `text=auto` would normalise them on
commit and re-expand on checkout, handing the Windows runner different bytes from
the ones committed — so the test would pass or fail on git's behaviour rather
than the application's, exactly where it is most likely to break.

Two things the corpus cannot contain, recorded in `docs/DECISIONS-0.1a.md` rather
than discovered later: a nested `.git/` directory, which git will not track, so
that entry of the ignore list is covered by a unit test over a temp directory;
and, on Windows, the trailing-dot filename, which the generator skips with a
warning instead of failing.

`tools/crash-save-loop` is not here: it drives the write path, and the crate that
owns the write path arrives in the next commit.

## 0.4.0 - move the architecture into docs/ and resolve the scope contradictions

Milestone 0.1a starts here. Nothing prescriptive is left at the repository root:
`ARCHITECTURE.md` moves to `docs/ARCHITECTURE.md`, and the scope-v1 page it
replaced becomes `docs/architecture-v1.md` — which also removes the hazard of two
files whose names differ only in case, on a filesystem where §11 of the same
document says case may not distinguish them.

The eight contradictions the review found are resolved in the document itself:

**The scope wins on the write contract.** `note_save(note_id, text,
buffer_version, base_rev)` — the `BaseRev` is explicit on the wire rather than
held core-side. That is scope §9 as written, and it collapses the app and
`notes-mcp` onto one write path: the agent already had to send the base it read,
and a core-held `open_rev` would have given the app a second, weaker rule for the
same guard.

**The document wins on three**, all recorded in a new §17.1 rather than by
editing the queue: `notes-markdown` is 0.1b because its first consumer is the
0.1b preview and front matter survives 0.1a through the byte policy, not a
parser; conflict resolution has three variants because "compare" changes nothing
on disk and is therefore UI, not a command; and a draft is written on four
occasions rather than two, a superset that cannot weaken the guarantee.

**The default ignore list is a constant in `notes-core`**, not configuration. It
could not live in `.notes/config.json`: that file is off by default, and a
default that only exists once the user opts in is not a default. `.notes/`
extends the list and can never replace it — no configuration file can unhide
`.git/`.

**The dmabuf workaround is unconditional at 0.0** and gated by a setting only
from 0.1a, because `settings.json` is itself 0.1a: gating 0.0 on it would gate it
on a file that does not exist. From 0.1a a missing settings file degrades to
`auto`, never to `off` — not applying it yields a black window, applying it
needlessly yields slightly slower compositing.

**Case sensitivity is probed rather than assumed**, and the probe reads instead
of writing — see `docs/DECISIONS-0.1a.md` D-01. The mechanism the owner specified
would have created a temporary file inside a folder that was merely opened,
which scope §2.3 forbids and a 0.1a acceptance criterion tests for. The intent is
kept: nothing is assumed from the operating system, and the flag self-corrects in
both directions.

**npm, not pnpm** — the lockfile has been committed since `0.3.7`.

`§18` is corrected for the ADR pass that closes 0.1a: items 2 and 9 fold into one
ADR so that ADR-005 is amended once rather than twice in the same commit; item 3
drops its `index.db` half, which is already ADR-012; and item 11 splits, because
the WebKitGTK workaround and Arch-as-a-release-target are two subjects.

`fixtures/large/` joins `.gitignore` under ADR-011's test. `.continue/` is
untouched, as instructed — which leaves one stale link in its README pointing at
the old root path.

## 0.3.11 - record what the 0.0 spike established, and what it did not

`docs/SPIKE-0.0.md`, in two halves, because the second is the one that matters.

**Verified here**, on Debian 13 / X11 / no NVIDIA: `cargo build`, `cargo clippy
--all-targets` and `npm run build` with zero warnings, and 12 tests passing with
no Tauri and no window. The document says what those tests actually cover rather
than reporting a count.

**Not verified here, and not claimed.** The window was never launched on this
machine, so "renders correctly" is unverified even for Debian/X11 — the document
says so and gives the command. Wayland, NVIDIA, macOS, Windows, iOS and Android
do not exist here at all. The checklist for them is written to be *seen* rather
than reasoned about: the diagnostics panel prints the word `APPLIED`, so
criterion 1 is read off a screen, not inferred from the fact that the code looks
right.

**Milestone 0.0 stays open and its queue item stays in `.continue/`.** The tests
prove the decision, not the rendering, and the whole reason a spike exists is the
part that only hardware can answer. Closing it here would be the failure the
document exists to prevent: a milestone marked done because the machine that
could not test it had nothing left to run.

The queue also records that my `.continue/ARCHITECTURE.md` is superseded by the
owner's `ARCHITECTURE.md`. It is kept rather than deleted — it is where the
questions were asked, and three of the four were answered by the document that
replaced it.

## 0.3.10 - track ARCHITECTURE.md and retire the v1-derived page it replaces

`ARCHITECTURE.md` at the repository root, written by the owner and aligned to
`.continue/SCOPE_final.md` v2.0. It closes every item SCOPE §20 delegates —
layout, crates, core types, app-data schemas, the command contract, `CoreError`,
the inter-process lock, the markdown IR, `Caps`, distribution — and its §18 lists
the decisions to record as ADRs. It is `PROPOSED` and becomes `ACTIVE` in the
commit that ships 0.1a, which is when those ADRs get written and numbered from
the last one here.

It arrived untracked. Committing it is the same rule that `0.2.0` broke in the
other direction: a document the project is about to be built from, existing only
in one working tree, is one accident from being the loss this repository has
already paid for once.

**Two architecture documents was the actual risk**, and this closes it.
`docs/architecture.md` — derived from the **v1** draft — is marked `SUPERSEDED`
with a line telling the reader not to build against it, and it names the file
that replaces it. It contradicts v2 on identity and on the app-data layout, and a
stale document is worse than a missing one precisely because it has the authority
of being written down. It is kept rather than deleted: it is the record of what
was understood before v2, and its original status line is preserved underneath.

## 0.3.9 - track Cargo.lock, which the spike build produced and 0.3.7 missed

The workspace builds a binary application, so the lockfile is part of the source:
without it, a clone resolves whatever versions are current that day, and "it
builds here" stops being a statement about this repository. `0.3.7` reported the
build as passing and left the file that makes the result reproducible untracked.

## 0.3.8 - write the ADR the .gitignore was already pointing at

`0.3.3` added `target/`, `node_modules/`, `dist/` and `.vite/` to `.gitignore`
with a comment saying the exception is recorded as ADR-011. **ADR-011 did not
exist.** The rule in that file is that any exception beyond secrets needs an ADR
rather than a silent line, and a line that cites an ADR nobody wrote is a silent
line with a citation on it — worse than an uncommented one, because it reads as
settled.

ADR-011 states the test for admitting anything to that list: it is produced by a
command in this repository, from inputs in this repository, and reproducing it is
running that command. `src-tauri/gen/schemas/` passes and joins the list — it is
rewritten by `tauri-build` on every build and read only by an editor resolving a
`$schema` reference; `0.3.7` committed it by accident. `icon-source.png` fails
the test and stays versioned: it is what `tauri icon` consumes, and without it
the icons cannot be regenerated.

The ADR is numbered 011 and lands after 012, which was written first. The number
is an identifier, not a timeline, and `.gitignore` had already named this one.

## 0.3.7 - complete the 0.0 spike so it builds, tests and lints clean

The Rust half the previous commit said was missing: the Tauri crate, the
capability set, the window and CSP configuration, the icons, the five commands,
the stylesheet, and the platform module. `cargo build`, `cargo clippy
--all-targets` and `npm run build` all pass with zero warnings, and `cargo test`
runs **12 tests with no Tauri and no window**.

**The dmabuf decision was split into a pure function, and that is the point of
the commit.** 0.0's first acceptance criterion is that the Wayland + NVIDIA
workaround is applied automatically — on hardware this was not written on. A
function that reads the environment can only be checked by having the
environment. `decide_dmabuf(linux, opt_out, already_set, session, nvidia)` can be
checked by anyone: it applies on Wayland + NVIDIA, stays out of the way on
Wayland alone, on X11 with NVIDIA and off Linux, loses to
`NOTES_NO_DMABUF_WORKAROUND=1`, and never overrides a value the user set. Six
tests. **They prove the decision, not the rendering** — the window still has to
be looked at, which is why 0.0 stays open.

The other six cover the two things a spike can still get wrong in a way that
matters later: `..`, `sub/../../` and absolute paths are refused against the
resolved path rather than by string rules that each miss a case; and the atomic
write round-trips bytes exactly for empty, plain, CRLF, BOM-led and
accented/emoji payloads, leaving no temp file behind.

Three rules are honoured now rather than retrofitted, because breaking them would
make the spike measure the wrong thing: the webview gets `core:default` and
`dialog:allow-open` and **no filesystem capability**; every path is re-resolved
and re-checked against the root at the moment of use, not only at open; and
opening a folder writes nothing into it, with the chosen path persisted in app
data.

The identifier is `br.com.samirhv.notes.spike`, suffixed deliberately. The
production identifier is still open, it fixes the app-data path on three
operating systems, and changing it later strands the state of everyone who
installed — a spike must neither squat on it nor pollute its directory.

`apps/notes-app/README.md` stops saying the app cannot run and starts saying what
it is not: no `BaseRev`, so a write can still overwrite a concurrent external
change; no identity registry, no draft recovery, no watcher, no byte policy. That
list is milestone 0.1a, and naming it here is what keeps the spike from being
mistaken for a first draft of it.

## 0.3.6 - amend ADR-004: index.db lives in app data, not in the workspace

ADR-012, written as an **amendment** rather than a reversal, because ADR-004's
rule was right and only its example was wrong. `.notes/` stays what ADR-004 made
it — optional, deletable, holding nothing whose loss costs a note — and its test
is untouched. One file moves out.

The reason is not that the index is rebuildable; it is that users keep their
folders inside Dropbox, iCloud Drive, OneDrive, Nextcloud and Syncthing, and
those tools copy files whenever they change with no knowledge of transactions.
**An active SQLite database copied mid-transaction is not stale, it is corrupt**,
and on the provider's side that corruption is what other devices download. Being
rebuildable is exactly why nobody would notice: the app reindexes, the provider
copies again, and the loop repeats with no error anyone can act on. A `-wal` file
copied apart from its database is the same failure wearing another name.

Recorded with its cost: "delete `.notes/` to force a reindex" stops being the
recovery path, so an explicit reindex command has to exist; and the app now keeps
per-workspace state the user cannot see from their file manager, which has to be
discoverable rather than folklore.

## 0.3.5 - propose ARCHITECTURE.md, closing the SCOPE §20 items 0.1a needs

`.continue/ARCHITECTURE.md`, v0.1, a proposal awaiting review. It closes the
three items SCOPE §20 delegates to it that milestone 0.1a cannot start without:
the persistent-state schemas (§20.1), the Tauri command contract and the core
error model (§20.2), and the `Caps` mapping per backend (§20.6). The other four
are left alone because they do not block 0.1a.

It goes in the queue, in Portuguese, because it describes something that does not
exist — the `QUEUE-RULE` block, not a judgement call.

Three things it settles that the SCOPE could not have known it left open:

**A global file is missing from the §6.1 layout.** Keying a `WorkspaceId` by the
canonical root path and persisting the last workspace are both data that cannot
live inside `workspaces/<WorkspaceId>/` — you need the index before you have the
id. `workspaces.json` is proposed alongside it.

**`registry.json` is needed at 0.1a, and not for the reason it looks like.**
Nothing consumes `NoteId` until 0.1b, so the registry looks deferrable. It is
not, because a suspended draft has to know which note it belongs to: keyed by
path, an external rename while the draft is suspended orphans it — and an
external rename during suspension is precisely the situation that produces
drafts. The 0.1a acceptance criterion "buffer recoverable on reopen" would fail
in the case that matters most.

**Capability detection cannot probe.** The reliable way to know whether `rename`
is atomic on a given root is to write a temp file and try. SCOPE §2.3 forbids
that — opening a folder must not modify it — and 0.1a has the literal acceptance
criterion "opening a folder creates no file in it". So caps are derived read-only
from the filesystem type, with an unknown type falling back to the conservative
profile. A FUSE mount that does support atomic rename will be treated as though
it does not; that is the cheaper mistake.

The document also argues one thing against the instruction that asked for it:
`notes-markdown` has no consumer at 0.1a. Front matter is preserved byte for byte
there, which is the `notes-fs` byte policy rather than parsing, and CodeMirror's
highlighting is explicitly not the semantic authority. Its first real consumer is
the 0.1b preview.

Four questions are held open at its §5 — the bundle identifier above all, since
it fixes the app-data path on three operating systems and changing it later
strands the state of everyone who already installed. No ADR is written yet:
writing `ACCEPTED` decisions for a proposal nobody has reviewed would be the
paperwork imitating the decision.

## 0.3.4 - stop restating the queue rule now that a block carries it

`0.3.2` took the queue rule to repodocs and it came back as the regenerated
`QUEUE-RULE` block. Four places in this repository still restated it as a local
override, which is one rule with two sources — the exact thing that commit
removed — and three of them now said something false: that the fleet rule does
not apply here, when the fleet had adopted this one.

Golden rules 1 and 2 collapse into one that points at the block and says **do not
restate it here**; the list renumbers to nine. The freed slot goes to the rule
that is genuinely local and is in no block: **an `ACTIVE` document in `docs/`
wins a contradiction, a `PROPOSED` one does not** — the queue is the authority on
intent while both exist. `.continue/README.md` and `docs/README.md` lose their
copies the same way and keep only what is theirs: that the queue's README is the
one file in the folder that is not queue material, and so stays in English while
the items around it do not.

The ADR bodies are untouched. Their status lines already record the fleet
adoption, and `0.3.2` put it there; rewriting a decision's Context and
Consequences to match what happened afterwards would turn the log into a
description of the present rather than a record of what was decided and why.

## 0.3.3 - commit the 0.0 spike scaffold, unfinished and parked

The Cargo workspace, and the frontend half of the 0.0 spike application:
`apps/notes-app/` with Vite, React, TypeScript and a CodeMirror 6 host, plus a
diagnostics panel that exists because the spike's product is evidence rather
than software.

**It is committed incomplete, on purpose, and says so in three places** — the
status line of `apps/notes-app/README.md`, a table of what is written against
what is missing, and this entry. The Rust half does not exist: `src/api.ts`
declares five Tauri commands and none of them is implemented, so the application
cannot run. `npm install` and `cargo build` have never been executed against it.
Committing it beats leaving it in a working tree nobody else can see, which is
the failure this repository has already paid for once at `0.2.0`; pretending it
works would be a different and worse failure.

The design is recorded even where the code is not: the Wayland + NVIDIA
`WEBKIT_DISABLE_DMABUF_RENDERER` detection is 0.0's first acceptance criterion,
it belongs in the missing `src-tauri/src/lib.rs` before the webview is created,
and the README says exactly that so the next session does not rediscover it.

Two rules are honoured in the frontend from the start rather than retrofitted:
no filesystem capability is granted to the webview, so every read and write in
`api.ts` is a call into the core (SCOPE §2.5); and the editor adds nothing to
input handling, because the mobile acceptance criterion is measuring the
platform's IME, not ours.

`.gitignore` gains `target/`, `node_modules/`, `dist/` and `.vite/`. That file
requires an ADR for any exception beyond secrets, so ADR-011 owes it one — the
line is written with a pointer, and the ADR follows in the architecture pass
rather than being waved through as obvious.

0.0 cannot be closed from this machine: its acceptance needs Arch/Wayland/NVIDIA,
an iPhone and an Android device.

## 0.3.1 - take SCOPE_final.md into the queue as the version to build

`.continue/SCOPE_final.md` — the owner's v2.0 specification, in Portuguese, as
[ADR-010](docs/decisions.md#adr-010--continue-is-written-in-portuguese-everything-else-is-english)
provides. It supersedes the two v1 drafts beside it and is the document the
application gets built from. Committed on arrival, because the rule that nothing
leaves the queue uncommitted is worth as little as its counterpart on the way in
— the v1 drafts were lost at `0.2.0` precisely for want of this commit.

It closes decisions the v1 left open, and several of them contradict ADRs that
are currently `ACTIVE`: identity lives in the app's own registry and **no `id`
is ever written into a `.md`**, not even when sync is switched on; the content
hash stops being identity and becomes a correlation signal with explicit
ambiguity rules; data outside the notes splits into three categories where only
the derived one is disposable, which moves `index.db` out of the workspace by
default — an active SQLite database copied mid-transaction by Dropbox or iCloud
is a corrupt database; a concurrency guard ships with autosave at 0.1a rather
than with sync; and a timeboxed 0.0 spike now precedes 0.1a.

Those reversals are not applied in this commit. An ADR is reversed by an ADR,
and this one only records the arrival of the document that argues for it.

## 0.3.0 - mark the unbuilt specifications as PROPOSED

`product.md`, `architecture.md` and `roadmap.md` were written at `0.2.0` and
marked `ACTIVE`, which claimed they described something that exists. They
describe an application with no code. They are now `PROPOSED`, each carrying the
same header: nothing here has been built, the specification still lives in
`.continue/`, **the queue is the authority on intent while both exist**, and a
section becomes `ACTIVE` when its code exists and works.

That resolves the duplication ADR-009 creates rather than pretending it is not
there. The same subject is in the queue in Portuguese and in `docs/` in English,
and the pair only stays honest if the direction of authority is written on the
face of the document — otherwise the next reader picks whichever they opened
first. `.continue/README.md` states the same rule from its side: an `ACTIVE`
document in `docs/` wins a contradiction, a `PROPOSED` one does not.

`decisions.md` stays `ACTIVE`, deliberately. A decision exists the moment it is
taken — the ADRs are the artefact, not a description of a future one — and it is
the record that stops a settled direction being re-litigated during exactly the
long stretch of a project where nothing has been built and everything is still
arguable.

Golden rule 4 already said an undeclared status is read as `ACTIVE` and that
this is "exactly the failure mode". Three documents were sitting in it.

## 0.2.1 - restore the scope drafts to the queue

`0.2.0` deleted `.continue/scope.md` and
`.continue/scope.md — Aplicativo Markdown Local-First.md` on the reading that
writing them up in `docs/` had finished them. That reading is wrong for this
repository: an item leaves the queue when it has been **built**, not when it has
been documented. Nothing in those two files exists as code, so they belong in the
queue, and they are back in it.

They had never been committed, so they were not recoverable from the history —
they are reconstructed here from the session that deleted them, and are content-
complete rather than byte-identical to the originals.

The queue's "where things went" table is corrected too: it claimed the drafts had
migrated to `docs/`, which was the same mistake stated as fact.

The rule this violated is not yet written down anywhere — that is the next
commit, and it is why this one only repairs.

## 0.2.0 - adopt the scope in the agent instructions and empty the queue

The last block of the conversion: making the documents the repository actually
reads agree with the four that were just written.

`CLAUDE.md` and `AGENTS.md` had `_to be filled in._` in all three identity slots.
They now carry the stack, the repository layout and — the part worth having — a
list of things not to do without an ADR reversing the one named: no note stored
anywhere but as a `.md` file, no metadata written into a user's note that the
user did not ask for, no file identified by path plus `modified_at`, no
listening port in the desktop app, no touching `.git/` in a workspace. An
instruction file that only describes the project is a file an agent skims; one
that names the five ways to break it is one that changes behaviour.

The `X`/`Y`/`Z` bump triggers stopped being the skeleton's examples and became
this project's, in both the twins and `docs/versioning.md`. A `Y` here is a
completed roadmap milestone, a new crate, a change to the `FileSystemAdapter`
surface, an index-schema change forcing a reindex, or an ADR reversing an
earlier one. Both places also state that milestone numbers are not versions:
`0.3` in the roadmap and `0.3.0` in `version.md` will otherwise be read as the
same thing, and they are not kept in step.

`README.md` describes what the project is rather than what it was going to be,
and says plainly that there is no application code here yet — the pre-flight in
`docs/runbook.md` §6 asks for exactly that, and this repository is public.

`.continue/` is empty of drafts. The two scope files are recorded in its "where
things went" table with links to what replaced them. **They were never committed,
so they are not in the history** — their content lives in `docs/`, translated and
split, and nowhere else. The three questions that were open in the queue are
closed and named against the ADRs that answered them, so they are reversed by a
new ADR rather than re-opened as a queue item.

## 0.1.1 - rename the project to notes

The project was called `franknote` until this commit. The name is dropped
because the `frank-` slot is already taken by a known project in the same space
(`frankmd`), and a name that collides costs more attention than it earns before
a single line of product exists.

`notes` is provisional and deliberately plain: it holds the slot until the
product has a shape worth naming, and renaming again is cheap for as long as
there is nothing but documentation here.

The rename went through GitHub's own rename, so the old URL redirects and any
link already pointing at `franknote` keeps working. The hook escape variable
followed the name — `FRANKNOTE_NO_HOOK` is now `NOTES_NO_HOOK`, declared at the
top of both hooks and in `docs/versioning.md`.

**The `0.1.0` entry below keeps its original wording.** At that commit the
project was `franknote`, and this file is not rewritten — a changelog that
retro-names its own history stops being a record of what happened.

## 0.1.0 - initial documentation structure

<!-- Replace this entry. The heading IS the commit subject, so write it in
     English, in the format `X.Y.Z - description`. The body is prose: what
     changed, why, and what you measured — not a bullet list. -->

First commit of franknote, a desktop app for writing Markdown, standalone and optionally linked to a public Git repository.

The documentation skeleton comes from the fleet standard at
[samirhvbr/repodocs](https://github.com/samirhvbr/repodocs) — the norm itself
lives there and is **not** copied into this repository, so there is one place to
change it.
