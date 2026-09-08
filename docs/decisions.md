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
[architecture.md §1](architecture-v1.md#1-the-layering-rule).

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

**Status:** `ACCEPTED` · 07/09/2026 · **amended** by
[ADR-014](#adr-014--identity-lives-in-the-registry-never-in-the-note-and-the-hash-is-correlation)
on two points of its Decision: identity is never written into a note file, and
the content hash is a correlation signal rather than an identity field. The rest
stands

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
[architecture.md §4](architecture-v1.md#4-the-filesystem-abstraction) so it is not
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

## ADR-011 — Build output is the one `.gitignore` exception beyond secrets

**Status:** `ACCEPTED` · 07/09/2026

**Context.** `.gitignore` in this repository carries a rule with teeth: everything
is versioned, the only exception is a secret, and **any further exception needs
an ADR rather than a silent line**. The rule exists because ignoring a directory
that holds an open question or a verdict has already cost this fleet real work,
and because a line added quietly is a line nobody can argue with later. The first
commit of application code needs `target/`, `node_modules/`, `dist/` and
`.vite/`, and the Tauri build generates `src-tauri/gen/schemas/` on every build.

**Decision.** Build output is ignored, and it is the only category admitted
besides secrets. The test for admitting anything here: it is produced by a
command in this repository, from inputs in this repository, and reproducing it is
running that command. `src-tauri/gen/schemas/` qualifies — `tauri-build` writes it
on every build, and the only thing that reads it is an editor resolving a
`$schema` reference. `icon-source.png` does **not** qualify and stays versioned:
it is the input `tauri icon` consumes, and losing it means the icons cannot be
regenerated.

**Consequences.** A fresh clone does not build without `npm install` and
`cargo build`, which is ordinary and is written in the app's README. The rule
that protects the queue keeps its force, because this exception is argued rather
than assumed — the `.gitignore` line names this ADR, so the next person adding
one can see what the bar was. The risk this accepts is the familiar one: a
generated directory that quietly starts holding something hand-edited stops being
build output while still being ignored. The mitigation is the test above, applied
when the line is added and not after.

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

---

## ADR-013 — The crate set, and when each one is created

**Status:** `ACCEPTED` · 07/09/2026 · extends
[ADR-003](#adr-003--the-rust-logic-lives-in-crates-and-the-tauri-shell-stays-thin)

**Context.** ADR-003 put the Rust logic in `crates/` and left the set open.
Building it out invites the opposite failure: crates created early, empty, "so
the structure is there", which are then refactored before they have a consumer
to constrain them.

**Decision.** `notes-model` (types, no I/O), `notes-fs` (the `FileSystem` trait,
`LocalFs`, the root jail, the atomic write), `notes-core` (`WorkspaceService`)
at 0.1a; `notes-markdown` at 0.1b, `notes-index` at 0.2, `notes-mcp` at 0.3,
`notes-sync` at 0.6. **No crate exists before the milestone that uses it.**
`notes-model` depends on nothing that does I/O — the rule that makes the write
protocol testable against a fake filesystem.

**Consequences.** `notes-markdown` is absent at 0.1a and front matter survives
anyway, through the byte policy rather than a parser — which is the evidence the
rule was right. The cost is that a crate boundary is decided when its first
consumer appears rather than in advance, so a wrong cut is found later; against
that, a boundary drawn with a consumer in hand is drawn from evidence.

---

## ADR-014 — Identity lives in the registry, never in the note, and the hash is correlation

**Status:** `ACCEPTED` · 07/09/2026 · amends
[ADR-005](#adr-005--sync-is-out-of-the-mvp-but-the-file-identity-model-is-not-foreclosed)

**Context.** ADR-005's Decision reads "a file eventually carries `file_id` …
`content_hash` …", which can be read as the file carrying them — and lists the
hash among the identity fields. Both readings have to be closed before sync is
built, and closing them separately would amend ADR-005 twice for one subject.

**Decision.** A `NoteId` lives in the app's registry and, later, on the server.
**Nothing is ever written into a `.md` file** — not a front-matter `id:`, not at
0.1a, not when sync is enabled at 0.6. And **the hash is not identity**: an
external rename reconnects a `NoteId` only on a unique native-id match or a
unique non-empty-hash match. Zero-byte files are never correlated by hash, and
any ambiguity yields a new id. Re-identifying is cheaper than attaching a note to
the wrong history.

**Consequences.** Copying a workspace folder produces a second workspace with new
ids, and reconnecting to a server is an explicit flow rather than something that
happens by itself. A user who wants portable identity across machines does not
get it from the file, and if that is ever wanted it is a separate opt-in feature
with the user told their files will change. In exchange the promise that the app
never writes what the user did not type survives contact with sync, which is the
milestone at which most note applications break it.

---

## ADR-015 — The registry is operational state, and moves to its own database at 0.2

**Status:** `ACCEPTED` · 07/09/2026

**Context.** The identity registry and the search index are both derived-looking
files that live outside the workspace, and treating them alike is the mistake:
the index is rebuildable from the notes, the registry is not. From 0.3 a second
process (`notes-mcp`) updates the registry, and a JSON read-modify-write between
two processes has no story better than a lock held for the whole file.

**Decision.** The registry is **operational** state: it has retention and
migration rules and no cleanup touches it. JSON at 0.1, moving to `registry.db`
— a SQLite file **separate from `index.db`** — at 0.2, so "delete the index" can
never touch identity. The `index.db` location is already
[ADR-012](#adr-012--indexdb-lives-in-app-data-not-in-the-workspace) and is not
re-decided here.

**Consequences.** Two database files instead of one, with two schemas and two
migration paths. The JSON at 0.1 is rewritten whole on every change, which is
O(n) in the number of notes ever opened — acceptable while nothing consumes a
`NoteId`, and the reason the move at 0.2 is stated as mandatory rather than
conditional.

---

## ADR-016 — One data directory, resolved by the core

**Status:** `ACCEPTED` · 07/09/2026

**Context.** Tauri offers `app_data_dir()`, and using it is the obvious choice
for an application built on Tauri. `notes-mcp` is not: from 0.3 it runs as a
stdio process with no Tauri and no window, and it reads and writes the same
registry.

**Decision.** `notes-core` resolves the directory itself —
`dirs::data_dir()/notes`, overridable by `NOTES_DATA_DIR`. Tauri's
`app_data_dir()` is not used.

**Consequences.** The bundle identifier no longer determines where state lives,
so changing it does not strand anyone. `NOTES_DATA_DIR` is what makes the whole
service testable without touching a developer's real notes, and it is what
"portable install" will mean later. The cost is one more thing that must agree
across processes, stated in one function rather than assumed twice.

---

## ADR-017 — Cross-process coordination is an advisory lock, per workspace

**Status:** `ACCEPTED` · 07/09/2026

**Context.** From 0.3 the app and `notes-mcp` write to the same files. Using the
same crate does not share a lock; the lock has to be in the filesystem. A pid
file is the usual reach, and it leaves a stale lock behind whenever a process
dies badly — which is exactly when it matters.

**Decision.** `write.lock` in the workspace's app-data directory, taken with an
OS advisory lock (`flock` / `LockFileEx`), one per workspace, guarding the
*stat → compare → replace* sequence and the registry update and nothing else.
Timeout 5 s, then `LockTimeout`, handled as a write failure — a draft is written
and the user is told. **There is no stale-lock problem by construction**: the
kernel releases an advisory lock when its holder dies. It exists from 0.1a, when
there is one process, so the protocol is exercised before a second arrives.

**Consequences.** One lock per workspace rather than per note serialises two
concurrent saves to different notes; hold time is milliseconds and simplicity
wins. **The lock coordinates our processes only** — a third-party editor does not
take it, and its writes are caught by the base-rev check instead. The scope does
not promise mutual exclusion with the rest of the system, and this ADR does not
either.

---

## ADR-018 — Preview crosses the IPC as sanitised HTML; outline and links as a slim document

**Status:** `ACCEPTED` · 07/09/2026 · applies from 0.1b

**Context.** The preview needs rendered Markdown in the WebView. Sending an AST
and rendering in JavaScript would put a Markdown parser in the frontend, and
sanitisation with it — inside the process that a malicious note is trying to
reach.

**Decision.** `notes-markdown` renders to HTML and `ammonia` sanitises it in
Rust; that HTML crosses the IPC. A slim `Document` — headings, links, tasks,
spans — crosses for outline and link work. The full AST does not, and the
frontend contains no Markdown parser.

**Consequences.** Sanitisation happens at one boundary, in one language, and can
be tested against `fixtures/xss/` without a browser. Interactive preview features
that would want the AST client-side have to ask the core instead, which is a
round trip. Front matter preservation is not this crate's job at all — the byte
policy keeps it intact because nothing rewrites the buffer.

---

## ADR-019 — Symlinks and junctions are not traversed

**Status:** `ACCEPTED` · 07/09/2026

**Context.** A symlink is a well-formed relative path that resolves somewhere
else, which makes it the one way a validated `RelPath` can leave the workspace.
Following them also makes the tree potentially infinite and identity ambiguous —
two paths, one file.

**Decision.** They appear in the tree marked as what they are and **do not
open**. Every path is resolved segment by segment and a symlink anywhere along
the way is refused, on every call rather than at open time. Following them is
opt-in, later, with its own ADR.

**Consequences.** A user who organises a workspace with symlinks finds them
inert, and the tree shows why rather than hiding them. The check costs a
`symlink_metadata` per segment per operation, which is a stat and is not
measurable against the read that follows.

---

## ADR-020 — One Tauri command per operation, with types generated by `ts-rs`

**Status:** `ACCEPTED` · 07/09/2026

**Context.** A single typed `dispatch(Request) -> Response` would put every
cross-cutting concern in one place and would let `notes-mcp` reuse the envelope.
Tauri's capability system is per command.

**Decision.** One command per operation. Types cross as `serde` JSON and `ts-rs`
generates the TypeScript for every one of them into
`apps/notes-app/src/ipc/generated`, which is committed; **CI regenerates it and
fails on any diff.** The frontend never hand-writes an IPC type.

**Consequences.** The argument that settles it is the capability: permitting
`dispatch` permits `delete`, and there is no way to grant half of it — a single
command would be a switch for the filesystem. The cost is many command names to
register and list. Generating the types caught a defect a Rust-only test could
not have: a nanosecond `mtime_ns` sent as a JSON number is silently rounded by
JavaScript and comes back wrong in the next `BaseRev`.

---

## ADR-021 — Autosave and the base-rev guard ship together

**Status:** `ACCEPTED` · 07/09/2026

**Context.** Autosave is a 0.1a feature and conflict detection reads like a sync
problem, so shipping autosave first and the guard later is the natural
sequencing. It is also the sequencing that loses data: the moment autosave
exists, the app is writing to files that VS Code, a script or an AI agent may be
writing at the same time, and without the guard it overwrites them.

**Decision.** They ship in the same milestone. Every write compares a `BaseRev`
before replacing; a divergence suspends autosave for that note, snapshots the
buffer to a draft, and writes nothing. Drafts and conflict copies are operational
data with retention rules and are **never deleted as cache**.

**Consequences.** 0.1a carries machinery that looks like sync infrastructure long
before sync — and it is the rehearsal for it. Storage that cannot replace
atomically has to say so rather than pretend. The user-facing cost is a note that
stops autosaving until they resolve it, which is the correct behaviour and has to
be visible: it is why the status bar has seven states rather than two.

---

## ADR-022 — The WebKitGTK dmabuf workaround is applied automatically on Wayland with NVIDIA

**Status:** `ACCEPTED` · 07/09/2026

**Context.** WebKitGTK on Wayland with the NVIDIA driver has a long history of a
black or flickering window. The mitigation —
`WEBKIT_DISABLE_DMABUF_RENDERER=1` — must be set before the WebView is created,
and telling users to export a variable means the first experience of the
application is a black window.

**Decision.** Detect Wayland and an NVIDIA driver at startup, before
`tauri::Builder`, and set the variable. Unconditional at 0.0, since `settings.json`
belongs to 0.1a; gated from 0.1a by `settings.linux.webkit_dmabuf_workaround`
(`auto` / `off` / `force`), where **a missing or unreadable settings file
degrades to `auto`, never to `off`**. A value already in the environment is never
overridden.

**Consequences.** Slightly slower compositing for users who did not need it, in
exchange for a window that renders. The degrade direction is chosen from the
asymmetry of the failures: not applying it yields a black window, applying it
needlessly costs a little performance. The decision is a pure function of its
inputs so the case the developer's machine cannot produce is covered by a test.

---

## ADR-023 — Arch Linux is a release target, with its own CI job

**Status:** `ACCEPTED` · 07/09/2026

**Context.** Arch is rolling. `webkit2gtk-4.1` moves without warning, and a build
that passes on Debian stable says nothing about it. The owner develops on Arch.

**Decision.** Arch is a release target, distributed through the AUR
(`notes-bin` from the release tarball, `notes-git` optional), and CI runs a job
in an `archlinux:latest` container against the current `webkit2gtk-4.1`.

**Consequences.** Upstream breakage surfaces in CI before it reaches a user, and
a red Arch job on a green Debian one is information rather than noise. The cost
is a CI job that can fail for reasons outside the repository, which is the point
and must not be treated as flakiness to be muted.

---

## ADR-024 — No unsigned macOS or Windows artefact is published

**Status:** `ACCEPTED` · 07/09/2026

**Context.** An unsigned Windows build trips SmartScreen and an unsigned macOS
build is refused by Gatekeeper. Both produce a first run that looks like the
application is malware, and the workaround taught to get past them is the same
one an actual attacker needs the user to learn.

**Decision.** macOS artefacts are signed with a Developer ID and notarised;
Windows artefacts are signed with an OV certificate. **No unsigned artefact is
published** for either. Linux artefacts (`.deb`, AppImage, AUR) need no signature
and are published without one.

**Consequences.** An Apple Developer account and a code-signing certificate are
prerequisites of the first macOS and Windows releases — cost and lead time, not
engineering. Linux ships before them, which matches where the project is
developed. Secrets live in CI secrets and never in the repository.

---

## ADR-025 — The preview corpus is a golden corpus, and blessing is not accepting

**Status:** `ACCEPTED` · 07/09/2026

**Context.** `fixtures/xss/` shipped at 0.1a with a README calling each file "an
assertion, not a sample", and nothing read it for a whole milestone. A corpus
nobody executes is a comment. The question at 0.1b was what "executing" it
should mean, and there are two answers with different failure modes: exact
output files, which can freeze a bug as a decision if regenerated carelessly,
and property assertions, which cannot say whether the renderer produces *the
right* HTML.

**Decision.** Both, for the two things they are each right for.
`fixtures/markdown/` holds an input, its exact expected HTML and its exact
expected `Document`, compared **byte for byte with no normalisation**;
`NOTES_BLESS=1` regenerates them, and **the diff is read against a written
contract before it is committed** (`fixtures/markdown/README.md`, one row per
file). `fixtures/xss/` asserts *properties* — structurally, on tags and
attributes read back out of the sanitized output — because a sanitizer is
specified by what cannot survive it, and because `safe-in-code.md` must render
`javascript:alert(1)` as text, which a substring ban would forbid.

**Consequences.** The reading is not ceremony: the first one caught four
defects, each of which the suite would otherwise have frozen — a dropped
`#section` fragment, an email autolink rendered as a link to a file with an `@`
in its name, a refused image losing its alt text, and bare URLs never linkified.
Byte-exactness also turned an intermittent `ammonia` attribute-ordering
behaviour into a red build rather than an occasional shrug
([DECISIONS-0.1b.md](DECISIONS-0.1b.md) D-07). The cost is that a
`pulldown-cmark` upgrade produces a diff that has to be read, which is the same
property stated as a cost.

---

## ADR-026 — Reconciliation is driven from what vanished, and a full scan announces no creations

**Status:** `ACCEPTED` · 07/09/2026 · **amends [ADR-014](#adr-014--identity-lives-in-the-registry-never-in-the-note-and-the-hash-is-correlation)**

**Context.** `ARCHITECTURE.md` §9 states identity correlation as
*"appeared := disk paths not in registry"*. That phrasing assumes a registry
that knows every file. This one does not: it is populated when a note is
**opened**, never by listing — [ADR-015](#adr-015--the-registry-is-operational-state-and-moves-to-its-own-database-at-02)
and `docs/DECISIONS-0.1a.md` D-09, which exist so that listing a 10 000-note
workspace does not hash 197 MiB. Under a lazy registry, "appeared" is
nearly every file in the workspace, on every scan.

**Decision.** Correlation is computed **from the vanished side**: for each
record whose path is gone, look for a unique match among the paths on disk that
no record claims. The answer is identical — a unique native id, then a unique
non-empty hash, then a new identity — and the work is zero on every tick where
nothing vanished. Separately, `ChangeKind::Created` is emitted **only for a
hinted path**, one the watcher has just reported; a full scan reports
modifications, removals and correlated moves and says nothing about a file that
is merely absent from the registry.

**Consequences.** A window regaining focus no longer announces every note the
user has never opened as newly created — which the first run of the
reconciliation tests did, a thousand events at a time, and which also blew the
hash budget with events that were not changes. Nothing is lost: a scan re-lists
the tree, which is what the sidebar needs, and a file that appeared as half of a
rename is found by correlation, which walks for exactly that. The cost is that
"a file appeared while the application was closed" is a listing rather than an
event, which nothing currently needs.

---

## ADR-027 — Not being able to watch is a state of the workspace, not a failure

**Status:** `ACCEPTED` · 07/09/2026

**Context.** `ARCHITECTURE.md` §11 already says several backends have no
watcher — SMB, NFS, exFAT, a SAF tree at 0.4 — and §8 says Linux can run out of
inotify watches. The obvious signature, `watch() -> Result<()>`, makes all of
those errors, and an error at open time is a workspace that will not open.

**Decision.** `FileSystem::watch()` returns a `Watch` carrying an optional
`degraded` reason rather than a `Result`. A workspace that cannot be watched
opens normally, is reconciled by a 5 s poll and a scan on focus, and **the
interface says why** — for the inotify case, with the `sysctl` that raises the
limit.

**Consequences.** Every backend in §11's matrix is usable, with a stated
limitation instead of a refusal, which is the same shape as `Caps` everywhere
else in this application. The application also always runs its poll and its
focus scan, watcher or no watcher, so a watch that is silently lost degrades to
5 s rather than to nothing.

---

## ADR-028 — A resolution keeps the version it did not choose

**Status:** `ACCEPTED` · 07/09/2026

**Context.** Scope §12 lists four resolutions for a conflict — compare, keep
mine, use the disk's, save as a copy — and three of them destroy one of two
versions of something the user wrote. It is the one moment in this application
where answering a dialog quickly can cost a morning.

**Decision.** Every resolution writes the version it is discarding to
`conflicts/` **before** it acts: `KeepLocal` snapshots the disk, `UseDisk`
snapshots the buffer, and `SaveAsCopy` writes the buffer to a file of its own so
both survive on disk. `note_convert_eol` — the one command that rewrites a file
the user did not edit — does the same with the old bytes. Unresolved conflicts
are drafts and are never pruned; resolved snapshots are pruned after a retention
setting whose `0` means *keep*, and the 200 MB warning **deletes nothing**.

**Consequences.** A resolution is always recoverable, which is what lets the
interface offer the three buttons without a second confirmation. The cost is
disk in app data, bounded by retention and reported rather than reclaimed —
making room by throwing away the only copy of something a user wrote is the
failure the directory exists to prevent.

---

## ADR-029 — `mailto:` and every scheme but `http(s)` render as text

**Status:** `ACCEPTED` · 07/09/2026

**Context.** Scope §8.4: *"Links externos `http(s)` abrem no navegador do SO por
clique. Outros esquemas recusados."* `mailto:` is the one that looks like an
exception worth making, and `shell:allow-open` in the capability file is
restricted to `http` and `https`.

**Decision.** A `mailto:` link, and an email autolink, render as text. Only
`http` and `https` become anchors, with `target=_blank rel="noopener
noreferrer"`, opened through a command that **checks the scheme again in Rust**
— the capability is what the WebView may ask for, and the check is what the
process will do.

**Consequences.** A rendered `mailto:` would have been a link that does nothing
when clicked, which is worse than text. Making it work means widening a
capability, and granting a permission is the owner's act, written into the
capability file with its reason — not applied by an agent on the way past
(CLAUDE.md golden rule 7). The change is two lines and is written out in
[DECISIONS-0.1b.md](DECISIONS-0.1b.md) D-06 for whoever makes it.
