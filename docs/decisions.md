# Decisions (ADR log)

> **Status:** `ACTIVE` · The single, chronological record of decisions taken in
> this repository. Format: Architecture Decision Record.

An ADR records a decision **and the reason it was taken**, so that the next
session does not re-litigate it. A how-to does not argue direction — it links
the ADR.

Numbering is sequential and never reused. A superseded ADR is not deleted: its
status changes to `SUPERSEDED` and it names the ADR that replaced it.

---

## ADR-001 — Markdown files on the filesystem are the source of truth

**Status:** `ACCEPTED` · 07/09/2026

**Context.** A note-taking app has to store notes somewhere, and the convenient
answer is a database: indexing is trivial, sync is tractable, and the schema is
whatever the app needs this week. The cost is paid later and by the user — their
notes become readable only through the app that wrote them, and "export to
Markdown" is a lossy escape hatch bolted onto a format that was never Markdown.
The alternative, plain files in a folder the user chose, gives up all of that
convenience and buys one thing: the notes remain usable from a terminal, VS
Code, `git`, `rsync`, a backup tool, another editor, or an AI agent writing
straight to disk.

**Decision.** Markdown files on the filesystem are the source of truth. There is
no proprietary storage format at any point. SQLite exists only as an index and
cache, and everything it holds must be rebuildable from the files. Markdown
exists on disk from the first moment — never as an export path from something
else.

**Consequences.** The app must tolerate the filesystem changing underneath it,
which makes external-change detection a requirement rather than a feature. It
must write atomically, because a half-written note is a lost note when the file
is the only copy. Sync becomes harder than it would be over a database, and
milestone 0.6 will pay for that in full. Indexing must be incremental, since
there is no schema to query directly. In exchange, every other decision in this
project becomes cheap to reverse: the app can be rewritten, abandoned or
replaced, and the user's notes are untouched. This is the decision the rest of
the architecture hangs off — see
[architecture.md §1](architecture.md#1-the-layering-rule).

---

## ADR-002 — Tauri 2 with React, TypeScript, Rust and CodeMirror 6

**Status:** `ACCEPTED` · 07/09/2026

**Context.** The product targets five platforms — Linux, macOS, Windows, iOS and
Android. Electron covers three of them and ships a browser to do it: roughly
120 MB per install and a memory floor that is hard to defend for a text editor.
Flutter covers all five with one rendering model, but puts the editor on a
canvas where CodeMirror does not exist, and the whole editing surface would have
to be rebuilt. Native per platform is the best result and several times the
work. Tauri 2 covers all five, uses the system webview instead of shipping one,
and puts the non-UI half of the app in Rust — which matters here because the
non-UI half is filesystem work, incremental indexing and, later, a sync protocol.

**Decision.** Tauri 2 as the shell; React and TypeScript for the interface;
Rust for filesystem, indexing, search and native integration; CodeMirror 6 as
the editor; SQLite as the index.

**Consequences.** The webview differs by platform — WebKitGTK on Linux, WKWebView
on Apple, Android WebView — and that is the recurring tax this choice carries;
CSS and web APIs need testing per platform rather than once. Binaries are around
an order of magnitude smaller than the Electron equivalent, and the Rust core is
reusable by `server/` later without a rewrite. The team needs Rust as well as
TypeScript. CodeMirror 6 rules out a WYSIWYG editor without replacing the editor
layer wholesale — acceptable, because [product.md §15](product.md#15-not-in-the-first-version)
puts full WYSIWYG out of scope anyway.

---

## ADR-003 — The Rust logic lives in `crates/` and the Tauri shell stays thin

**Status:** `ACCEPTED` · 07/09/2026

**Context.** The default Tauri layout puts Rust code in `src-tauri/`, which is
the right place for a single application and the wrong place for this one:
[roadmap.md](roadmap.md) has a self-hosted server at milestone 0.5 that needs the
same domain model, the same index and the same sync protocol as the app. Logic
that grows inside `src-tauri/` gets entangled with Tauri's command layer, and
extracting it later happens under deadline pressure, which is when it happens
badly.

**Decision.** Domain logic lives in workspace crates — `notes-core`,
`notes-fs`, `notes-index`, `notes-sync` — and `src-tauri/` is a thin shell that
exposes Tauri commands and wires them to those crates, holding no business logic
of its own. The repository is a Cargo workspace with `apps/`, `crates/`,
`packages/` and `server/` from the start, even though only `apps/notes-app/`
exists at milestone 0.1.

**Consequences.** More ceremony on day one: a workspace, crate boundaries and
cross-crate types before there is a second consumer to justify them. Crate
boundaries have to be decided early, and a wrong cut costs a refactor. Against
that, `server/` can depend on the core at 0.5 without an extraction, and the
boundary keeps Tauri-specific types out of the domain model — which is also what
keeps the domain model testable without a running app.

---

## ADR-004 — `.notes/` holds only data that can be rebuilt, and must be deletable

**Status:** `ACCEPTED` · 07/09/2026 · **amended** by
[ADR-012](#adr-012--indexdb-lives-in-app-data-not-in-the-workspace) on where
`index.db` is kept. The rule below stands as written; only the location of that
one file changes

**Context.** The app needs somewhere to keep the index, workspace settings and
caches, and the obvious place is a reserved directory inside the workspace. The
failure mode of such a directory is well known: it starts as a cache, something
convenient gets stored there because it has no other home, and eventually
deleting it loses user data. At that point the directory is a proprietary store
living inside a folder that was supposed to be plain Markdown, and
[ADR-001](#adr-001--markdown-files-on-the-filesystem-are-the-source-of-truth) has
been broken without anyone deciding to break it.

**Decision.** `.notes/` holds only auxiliary data — `workspace.json`, `index.db`,
`cache/`. It must never contain the only copy of anything the user wrote.
Deleting it must cost a reindex and nothing else. The test for anything proposed
for `.notes/`: delete the directory — does the user lose something they wrote? If
yes, it does not go there.

**Consequences.** Things that would be easy to keep in `.notes/` need another
home or must be derivable from the files: note metadata belongs in YAML front
matter, and per-file sync state has to be reconstructible. A "reindex workspace"
path must exist and stay working, which means it needs to be exercised, not just
implemented. The gain is that the workspace stays a plain folder that survives
the app.

---

## ADR-005 — Sync is out of the MVP, but the file identity model is not foreclosed

**Status:** `ACCEPTED` · 07/09/2026

**Context.** Multi-device sync is at milestone 0.6, four milestones after the
first usable app. The risk is not that it is late; it is that decisions taken now
make it impossible later. The specific trap is identifying a file by its path and
its `modified_at`. Paths change under rename. Timestamps are not a version:
clocks disagree between devices, filesystems round them differently, a restored
backup rewrites them wholesale, and none of that is detectable after the fact.
An app built on path + mtime cannot be given sync later — it has to be rebuilt.

**Decision.** Do not build sync now. Do design the data model so it stays
possible: a file eventually carries `file_id` (stable across renames), `path`,
`revision` (monotonic per file), `content_hash`, `modified_at` (advisory, never
authoritative) and `device_id`. The protocol must account for tombstones,
conflicts, renames, offline modification and history. And the rule that outranks
all of it: **a silent overwrite is a defect, never a conflict resolution** —
both versions survive, whether as `project (conflict iphone).md` or through a
resolution interface.

**Consequences.** The index schema carries fields that nothing reads at
milestones 0.1 to 0.4, and they have to be maintained correctly anyway — an
identity that is only nearly right is worse than none, because sync will trust
it. Generating and preserving a stable `file_id` across renames done by *other*
programs is real work, and the honest answer for a file that vanishes and
reappears may be that it is a new file. The alternative is discovering the whole
problem at 0.6 with an app to rewrite.

---

## ADR-006 — Git is not a dependency, and not a feature in the first versions

**Status:** `ACCEPTED` · 07/09/2026

**Context.** The project was first sketched as a Markdown editor with a public
Git repository attached — the notes would live in a repo, and publishing would
be a native act. Once local-first was settled it stopped fitting: Git as a
dependency means requiring an installed binary or embedding libgit2, plus a
credential story, plus a conflict story — and it is unusable on iOS, where a
whole milestone of the product lives. It would also make the second-simplest
thing a user can do (open a folder that is not a repo) a special case of the
harder one.

**Decision.** The app does not depend on Git and does not integrate with it in
the first versions. Because notes are ordinary Markdown files in an ordinary
folder, a workspace *can* be a Git repository, and the app must not get in the
way — concretely, it tolerates `.git/` in the tree and never touches it. Native
integration is a candidate for later, listed as out of scope in
[product.md §15](product.md#15-not-in-the-first-version).

**Consequences.** Versioning and multi-device use are not solved by borrowing
Git's — they are the job of milestones 0.5 and 0.6, and this ADR is part of why
those exist. Users who want Git keep using Git, outside the app, which works
today and costs the project nothing. The `.git/` directory has to be excluded
from indexing and from the file tree, which is a small explicit rule rather than
an accident of hidden-file filtering.

---

## ADR-007 — The desktop app opens no network port by default

**Status:** `ACCEPTED` · 07/09/2026

**Context.** Milestones 0.5 and 0.7 add an HTTP API and an MCP interface for AI
agents. The convenient implementation is a small server inside the desktop app,
listening locally. That turns every install into a listening service, on
machines whose owners believe they installed a text editor; on a laptop that
joins untrusted networks, a local API with filesystem access is a serious
default to ship silently.

**Decision.** The desktop application opens no network port by default and
initiates no outbound connection for its basic operation. The API and MCP
surfaces belong to **Notes Server**, which the user chooses to run. Any external
integration is explicitly enabled by the user.

**Consequences.** An agent that wants to reach the notes has to go through a
server the user deliberately started — more setup for that user, and the setup
is the consent. A local-only agent mode, if it is ever wanted, needs its own ADR
rather than arriving as a default. In exchange, "local-first" stays literally
true: the app installed and never configured talks to nothing. The normative
statement lives in [security.md](security.md), which wins any conflict with this
log.

---

## ADR-008 — Desktop first; mobile at milestone 0.4, behind the same abstraction

**Status:** `ACCEPTED` · 07/09/2026

**Context.** The product targets desktop and mobile. Building both at once was
considered and rejected on a specific ground rather than on effort: the
workspace model — a folder the user chose, held open, watched for external
changes — is a desktop concept. iOS has no equivalent; the app gets a sandboxed
container or scoped access through the document picker. Android has scoped
storage and the Storage Access Framework. Designing the first version to satisfy
all three at once means the desktop app is worse today for a user who does not
exist yet.

**Decision.** Desktop (Linux, macOS, Windows) at milestone 0.1. Mobile at
milestone 0.4. The `FileSystemAdapter` seam exists from 0.1 with a single
adapter behind it, so 0.4 is an adapter and an interface, not a rewrite. The
mobile interface is adapted to mobile rather than a shrunk desktop.

**Consequences.** Mobile users wait. The abstraction is carried for three
milestones before a second implementation justifies it — accepted deliberately,
with the reasoning recorded in
[architecture.md §4](architecture.md#4-the-filesystem-abstraction) so it is not
"simplified away" by a later reader who sees one adapter behind an interface.
There is a real risk that the seam turns out to be cut in the wrong place when
the iOS adapter is finally written; that is cheaper than a UI written directly
against local paths.

---

## ADR-009 — An item leaves `.continue/` only when it has been built

**Status:** `ACCEPTED` · 07/09/2026 · **Adopted fleet-wide the same day** as
[repodocs ADR-021](https://github.com/samirhvbr/repodocs/blob/master/docs/decisions.md)
— it stopped being local. The rule now arrives here in the `QUEUE-RULE` block of
`CLAUDE.md`, which is regenerated; this ADR stays as the record of where the
decision was made, and the block is the source if the two ever differ

**Context.** The fleet convention says a document moves from queue to record
"the moment it describes something that already exists", and a companion rule
sends any queue item needing more than half a page to `docs/`. Both were
followed at `0.2.0`, and the result is the reason this ADR exists: a 1 338-line
specification for an application with zero lines of code was translated into
`docs/`, deleted from `.continue/`, and the queue then reported nothing open —
on a project where nothing at all had been built. The ambiguity is one word.
"Exists" was read as *the definition* existing; the owner means *the thing*
existing. Under the first reading, writing about a black screen with a yellow
ball makes it exist. Under the second, only the screen does.

The failure is specific to a new project and worst exactly there: on day one
everything is words and nothing is code, so a rule that retires an item once its
text is tidy retires the entire queue. A queue whose job is to list what is
still missing must not lose an entry because someone described the entry well.

**Decision.** In this repository an item leaves `.continue/` when the thing it
describes **has been built and works** — not when it has been documented,
decided, translated or written up. **Size is never a reason to move an item
out**: a specification of any length stays in the queue while its code does not
exist. A decision taken along the way still becomes an ADR in the same pass, and
that ADR does **not** retire the queue item. And nothing leaves `.continue/`
before it has been committed, so that a wrong call costs a `git revert` rather
than a reconstruction from memory.

**Consequences.** This overrides two fleet rules for this repository —
`conventions.md` §1 and the golden rule that sends a half-page item to `docs/` —
and an override that is not written down is not an override, which is what this
ADR is for. The queue will hold large files. `docs/` may hold a document
describing something that does not exist yet; those carry `PROPOSED` rather than
`ACTIVE`, and the queue, not the document, is the authority on intent while both
exist. The cost is real: the same subject can live in the queue and in `docs/` at
once, in two languages, and they can drift. The mitigation is direction — intent
changes in the queue, and `docs/` is updated when the thing is built. If this
rule is right for the whole fleet rather than only here, it belongs in repodocs
`conventions.md`, and this ADR is the argument to take there.

---

## ADR-010 — `.continue/` is written in Portuguese; everything else is English

**Status:** `ACCEPTED` · 07/09/2026 · **Adopted fleet-wide the same day** as
[repodocs ADR-022](https://github.com/samirhvbr/repodocs/blob/master/docs/decisions.md),
which makes it the third carve-out of the English rule rather than this
repository's exception to it. The `LANGUAGE-RULE` block above now carries it

**Context.** The language rule — repodocs `conventions.md` §8, stamped into
`CLAUDE.md` and `AGENTS.md` as the `LANGUAGE-RULE` echo — puts everything in the
repository in English (US), with two carve-outs: end-user-facing strings, and
the Blue3 internal repositories. `.continue/` is in the repository, so it falls
under English. But the queue is where the owner thinks out loud before anything
exists, and a second language is a tax on precisely the part of the work with
the least tolerance for one. It also contributed to the `0.2.0` mistake:
"it is in Portuguese" was part of the case for emptying the queue.

**Decision.** `.continue/` is written in Portuguese. Translation to English (US)
happens **on the way out** — when the thing has been built and its document
lands in `docs/`, per
[ADR-009](#adr-009--an-item-leaves-continue-only-when-it-has-been-built).
Everything else is unchanged and stays English (US): `docs/`, commit messages,
pull request titles and bodies, issues, code comments, changelog entries,
release notes.

**Consequences.** The exception must be written **outside** the `LANGUAGE-RULE`
markers in `CLAUDE.md` and `AGENTS.md`. That block is a marked echo regenerated
from repodocs, so an exception written inside it is erased by the next fleet
pass with nobody noticing — the precedent is `BLUE3-INTRANET`, whose language
exception sits outside the block for exactly this reason. A contributor who does
not read Portuguese cannot read the queue; that is acceptable while the queue is
the owner's own, and it is the trigger to revisit this ADR rather than a cost to
absorb quietly. Translation work concentrates at the moment of production
instead of being spread thin, which is also the moment the material is best
understood — writing it in English is part of checking that it was actually
built.

---

## ADR-012 — `index.db` lives in app data, not in the workspace

**Status:** `ACCEPTED` · 07/09/2026 · amends
[ADR-004](#adr-004--notes-holds-only-data-that-can-be-rebuilt-and-must-be-deletable)

**Context.** ADR-004 established that `.notes/` inside the workspace holds only
data that can be rebuilt and must be safe to delete, and it listed `index.db`
among the files living there. The rule is right. The example is not, for a reason
that has nothing to do with whether the file is rebuildable.

A workspace is a folder the user chose, and users put those folders inside
Dropbox, iCloud Drive, OneDrive, Nextcloud and Syncthing. Those tools copy files
whenever they change, with no knowledge of transactions. **An active SQLite
database copied mid-transaction does not produce a stale database — it produces a
corrupt one**, and on the sync provider's side that corruption becomes the
version other devices download. The database being rebuildable is exactly why
nobody would notice: the app would reindex, the provider would copy again, and
the loop would repeat with no error the user could act on. A `-wal` file copied
without its main database, or after it, is the same failure with a different
name.

**Decision.** `index.db` and every other derived cache live in app data, per
workspace, outside the folder the user chose. `.notes/` inside the workspace
remains what ADR-004 made it: optional, deletable, and holding nothing whose loss
costs the user a note — portable configuration the user switches on, never the
index.

**Consequences.** The index no longer travels with the folder: copying a
workspace to another machine copies the notes and leaves the index behind, and
the second machine reindexes. That is the correct outcome and cheaper than
shipping a database written by a different build. Deleting `.notes/` no longer
removes the index, so "delete `.notes/` to force a reindex" is not the recovery
path — a reindex command is, and it has to exist and be reachable. The app now
keeps per-workspace state the user cannot see from their file manager, so where
it lives has to be discoverable rather than folklore. ADR-004's test is untouched
and still applies to everything proposed for `.notes/`: delete it, and if the
user loses something they wrote, it never belonged there.
