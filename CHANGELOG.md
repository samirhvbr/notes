# Changelog

Entries in the commit-message format (`version - short description in English`, see
[docs/versioning.md](docs/versioning.md)), newest first. **Each `##` heading is
literally the commit subject** — this file is the handoff artefact between
whoever does the work and whoever commits it.

Bodies are narrative: what changed, why, and what was measured. This file is
never rewritten.

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
