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
