# Decisions taken while building 0.1d

> **Status:** `ACTIVE` · Calls made under the standing rule — *choose the
> simplest option compatible with the scope, write the missing section into
> `ARCHITECTURE.md`, and carry on*. One row per decision: what was decided,
> which gap it closed, and **what the alternative is** if the owner disagrees.
>
> `.continue/0.1d-interface.md` §10 leaves three questions open by name and asks
> for them to be decided by the rule and recorded here. They are D-03, D-04 and
> D-05.
>
> §5 of that document also sets this milestone's boundary: *"O core não muda por
> causa da cara."* Where the interface met a gap in the core, it is written down
> here rather than fixed on the way past — D-01 is the first of those.

---

## D-01 — The registry debounce `ARCHITECTURE.md` §4.1 describes does not exist, and nothing here adds it

**Found, not decided.** §4.1 says the registry is *"written atomically (tmp +
rename), debounced to at most one write per 2 s, and on shutdown"*. The debounce
is not implemented: `open_note` calls `state::store` on every observe, as do
save, reconcile and the preview's asset path. Every mutation is written
immediately.

**Why it came up.** The workspace selector switches with `close` then `open`
(`.continue/0.1d-interface.md` §4.2), and the question was whether the third
path — `open_workspace` called while another workspace is open, which replaces
`self.open` outright — could drop an unwritten registry change. With the
debounce as documented, it could: identity is **operational** state
([ADR-015](decisions.md)), and a lost `NoteId` is a tab that cannot find its
note and, at 0.6, a file re-uploaded as new. Without it, there is nothing
unwritten to drop.

**What was done, and what was undone.** A store-before-adopt was written into
`open_workspace` and then removed, because `tests/switch.rs` passed identically
with and without it — a test that cannot fail is not evidence, and a fix with no
demonstrated defect is a claim. The tests stay: they pin the property
(*identity survives a switch*) rather than a mechanism, and nothing had ever
exercised the switch path at all, because until this milestone the only route to
it was a Welcome screen that disappears after the first open.

**What this leaves.** Two documents that disagree. The day the debounce is
implemented — and §4.1 is right that it should be, since `~/x` writes the whole
registry on every note opened — `open_workspace` becomes a loss path and
`tests/switch.rs` starts failing, which is the outcome to want.

**Alternative if you disagree.** Implement the debounce now and add the
store-before-adopt with it. That is a core change in a milestone whose §5 says
the core does not change for the interface, and it is a performance change made
without a measurement.

---

## D-02 — A DOM test environment enters the repository, for focus

**Decided.** `jsdom`, `@testing-library/react`, `@testing-library/user-event`
and `@testing-library/jest-dom` join the **dev** dependencies. `vitest`'s
environment stays `node` by default and a file opts in with
`// @vitest-environment jsdom` on its first line.

**Gap closed.** `.continue/0.1d-interface.md` §7 asks, in its own words, for
*"teste de componente com foco rastreado"* on the menus. Focus is not a
property a reducer has. A pure state machine can prove which row is *selected*;
only a document can prove that closing the menu put focus back on the button
that opened it — and that is the failure being guarded against, because a menu
that keeps focus leaves a keyboard user on `<body>` with no way back into the
interface.

**Why it is defensible under scope §19**, which tells an agent to stop before a
central dependency: these are four **development** dependencies that ship in
nothing. `vite build` does not see them, the `.deb` does not contain them, and
removing them costs one test file. The stack table in §4 is about what the
product is built from; this is about what the tests are run in.

The default stays `node` because the store tests are pure logic and a DOM around
them would be slower and would let a test accidentally depend on one.

**Alternative if you disagree.** Extract the menu's key handling into a reducer
and test that, with the focus calls untested. It adds no dependency and it does
not test the thing that breaks.

---

## D-03 — The editor column is a fixed maximum, not a setting

**Decided.** ~700 px, in CSS, not in `settings.json`.

**Gap closed.** `.continue/0.1d-interface.md` §10, first question.

**Why.** The rule is *the simplest option compatible with the scope*, and the
scope's settings list is closed: §17 puts *"configurações mínimas (font, line
numbers, wrap, tab size)"* in 0.1c and names the four. A fifth arrives with a
schema field, a migration, a control, two catalogue keys and a row in the
settings panel — for a value whose good range is narrow and whose bad values are
all worse. A measure that is too wide is the specific thing this column exists
to prevent.

It is a CSS custom property (`--column`), so changing it is one line and no
migration, which is the reversibility that makes deciding now cheap.

**Alternative if you disagree.** Add it to `editor` in `settings.json` with the
other four. It is a schema bump and it is the kind of setting people ask for.

---

## D-04 — Split is horizontal only in this milestone

**Decided.** One vertical divider, two panes side by side. No vertical split, no
grid, no more than two panes.

**Gap closed.** `.continue/0.1d-interface.md` §10, second question.

**Why.** Split already exists as a *view mode* — source, preview, split
(scope §9, `ADR-030`) — and 0.1d's job is to give it a real divider and a second
header, not to turn it into a window manager. Two panes side by side is what the
milestone's own drawing shows. Anything beyond that needs a layout tree, which
needs to be persisted in `session.json`, which is a schema change in a milestone
that is meant not to touch the core.

**Alternative if you disagree.** A layout tree from the start, so the second
split never has to migrate the first. It is the right shape for the eventual
feature and it is several times this milestone's frontend work.

---

## D-05 — Welcome stays a screen, and does not become an empty shell

**Decided.** With no workspace open the window shows the Welcome screen, not the
rail-and-empty-sidebar shell.

**Gap closed.** `.continue/0.1d-interface.md` §10, third question.

**Why.** Every control in the shell operates on a workspace: the explorer lists
one, the tab bar holds notes from one, the note header acts on one, the status
bar reports one. An empty shell is a window of controls that all do nothing,
which reads as an application that has failed to load rather than one waiting
for a folder. Welcome asks the only question there is to ask.

It also keeps §3's second principle — *nada mente na tela* — without a special
case: there is no disabled toolbar to explain and no empty tab bar to justify.

**Alternative if you disagree.** The shell with an empty sidebar, which is what
Obsidian does and which makes opening a folder feel like a smaller step. It
costs an empty state for every panel in the shell rather than one screen.
