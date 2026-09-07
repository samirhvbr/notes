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
