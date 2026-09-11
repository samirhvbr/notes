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

**Status:** `ACCEPTED` · 07/09/2026 · **amended** by
[ADR-033](#adr-033--the-dmabuf-workaround-keys-on-the-nvidia-driver-not-on-the-display-server):
the Wayland half of the condition was wrong. WebKitGTK uses the DMA-BUF renderer
on X11 too, the fault is in NVIDIA's GBM rather than in a compositor, and
requiring Wayland cost a window on X11. Everything else below stands

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

**Status:** `ACCEPTED` · 07/09/2026 · amended by
[ADR-034](#adr-034--the-tree-appears-in-under-a-second-at-any-size-whole-tree-work-is-background-work) — coverage has a middle, and it is reported with numbers

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

---

## ADR-030 — Tabs are a list beside the editor, not a second document model

**Status:** `ACCEPTED` · 08/09/2026

**Context.** Milestone 0.1c adds tabs with restoration. The editor store holds
exactly one loaded document, and everything in 0.1a and 0.1b is written against
that: the write protocol, `buffer_version` and the stale-save guard, the draft
rules, the conflict state, the reload-from-disk path. Making it hold a map of
documents to gain a tab strip would put every one of those back in play for what
is, at 0.1c, a navigation feature — two notes are never visible at the same time.

**Decision.** A separate store owns the **list**; the editor keeps owning the
**document**. A tab carries only what has to survive a restart — path, `NoteId`,
cursor, scroll, pinned — and the buffer stays where it was. Leaving a tab is not
a new rule: a dirty note is flushed and a note in conflict writes its draft,
which is what `ARCHITECTURE.md` §5 already says happens when a buffer stops being
looked at.

The cursor is handed to the editor **after** it mounts the document, because a
position in a document that does not exist yet means nothing; reports from a
freshly mounting editor are suppressed while a restore is in flight, or the
caret at 1:1 would overwrite the one being restored.

**Consequences.** Switching tabs re-reads the note from disk rather than swapping
an in-memory buffer, which is a round trip the user can feel on a very large
note — accepted, because it means there is exactly one place where a buffer
exists and therefore exactly one place where it can be lost. The moment two
documents must be **visible** at once — a split of two notes, not of one note's
source and preview — this decision is the one to revisit, and it should be
revisited rather than worked around.

---

## ADR-031 — Global search is a scan the core owns, polled like reconciliation

**Status:** `ACCEPTED` · 08/09/2026

**Context.** Scope §10 puts a workspace-wide search at 0.1c and the FTS5 index at
0.2, and requires the first result in under 500 ms on a 10 000-note workspace
with the search cancellable. `ARCHITECTURE.md` §7.2 sketched `search:result` and
`search:done` as core-to-frontend events; the reconciliation built at 0.1b uses
polling instead, and two delivery mechanisms for two streams of the same kind is
one more than the application needs.

**Decision.** The core walks the workspace with `ignore` and matches with
`regex`, in a background thread, pushing hits as they are found. The frontend
**polls** — `search_start`, `search_poll`, `search_cancel` — exactly as it polls
reconciliation. Every worker reads the cancel flag before each file, so
cancelling is bounded by one file rather than by the workspace.

**This scanner does not go away when FTS5 arrives.** §10 requires the three
semantics — literal, words, regex — to keep their names and not swap underneath
the user; the index takes over *words*, and literal and regex remain what the
scan is for.

**Consequences.** Measured at 11.4 ms to the first result and 650 ns to cancel
over 197 MiB, which is the criterion met by two orders of magnitude. Polling adds
a latency floor of one interval, which is invisible against a walk and is the
price of one delivery mechanism instead of two. The core carries a background
thread and a cancellation flag it did not have; a search left running by a
closed panel is prevented by cancelling on drop rather than by asking every
caller to remember.

---

## ADR-032 — Quick open matches a cached path list, and the tree invalidates it

**Status:** `ACCEPTED` · 08/09/2026 · amended by
[ADR-034](#adr-034--the-tree-appears-in-under-a-second-at-any-size-whole-tree-work-is-background-work) — the list is built in the background, and a partial one answers

**Context.** `Ctrl+P` has to answer on every keystroke. Walking a 10 000-note
workspace takes tens of milliseconds — fine once, ruinous per character — and
reading files to match a *name* would be work for nothing.

**Decision.** The service caches the workspace's note paths, built on first use,
and matches them in memory. It reads no file. **Every operation that changes the
shape of the tree drops the cache** — create, rename, move, duplicate, delete,
and any reconciliation tick, since a file that appeared or vanished outside the
application changes the tree as surely as one the application renamed.

Scoring is deliberately small and explainable rather than clever: a subsequence
match, a bonus for consecutive characters and for landing at the start of a path
segment, and the file name ranked ahead of the directory, because `Ctrl+P` is how
someone reaches for a file they can name.

**Consequences.** The first `Ctrl+P` after opening a workspace pays for the walk;
every later one is a filter over memory. Invalidating on every reconciliation
tick is coarse — a tick that changed nothing still drops the cache — and that is
chosen over tracking which paths moved, because a stale quick-open list offers a
note that is not there, which is worse than rebuilding a list. The scoring will
disagree with someone's expectation eventually; it is small enough to read and
change, which is the property that matters.

---

## ADR-033 — The dmabuf workaround keys on the NVIDIA driver, not on the display server

**Status:** `ACCEPTED` · 08/09/2026 · amends
[ADR-022](#adr-022--the-webkitgtk-dmabuf-workaround-is-applied-automatically-on-wayland-with-nvidia)

**Context.** ADR-022 said "detect Wayland and an NVIDIA driver". The Wayland half
was wrong, and it was wrong in the direction that costs a window.

WebKitGTK has used the DMA-BUF renderer on **X11 as well since 2.42**. The fault
the workaround exists for is in NVIDIA's GBM — creating the buffer — not in a
compositor, so it occurs on both display servers. Observed on Debian 13 / X11 /
NVIDIA, with the environment variable unset:

```text
[notes] dmabuf: not needed — session is not wayland
src/nv_gbm.c:288: GBM-DRV error (nv_gbm_create_device_native): …failed (ret=-1)
KMS: DRM_IOCTL_MODE_CREATE_DUMB failed: Permission denied
Failed to create GBM buffer of size 1100x720: Permission denied
[notes] window main: close requested
[notes] window main: destroyed
```

The workaround declined to apply, and the next four lines are the failure it
exists to prevent. **This is the window that disappeared on *Open Folder***
(`docs/DECISIONS-0.1b.md` D-20): the GBM buffer for a new surface cannot be
created, the window is destroyed, and Tauri ends its event loop with status `0` —
which is why it never looked like a crash. It had nothing to do with the file
chooser; the chooser was merely the first thing that asked for a surface.

The reason it took this long is worth recording, because it is a hazard and not
an accident: **the owner's shell already exported
`WEBKIT_DISABLE_DMABUF_RENDERER`.** `linux.rs` correctly refuses to override a
value the user set, so every run before this one logged *"left alone — already
set"*, the workaround never ran, and no comparison was ever made. A masked
symptom produced a plausible mechanism (a GTK chooser taking its parent down)
built on a false premise.

**Decision.** On Linux, the **proprietary NVIDIA driver alone** decides. The
display server is no longer an input to `decide` — not weighted differently,
removed, because it never bore on the failure.

`nouveau` does not count, and is distinguished by what each driver *creates*
rather than by a name: `/proc/driver/nvidia/version` is written by the
proprietary kernel module and by nothing else; `/sys/module/nvidia/` is that
module's own sysfs directory, while nouveau's is `nouveau`; `nvidia-smi` on
`PATH` is a weaker hint and is never shipped by nouveau. Nouveau's GBM works, so
disabling the renderer there would cost compositing performance for no reason.
Both are reported in the diagnostics panel, so a user can see which is loaded.

**Consequences.** Every Linux machine with the proprietary driver now turns the
DMA-BUF renderer off, X11 included — a real performance cost on hardware where
the bug may never have shown, accepted because the asymmetry is absolute: applying
it needlessly is slower compositing, and not applying it is no window at all. The
`off` setting remains for anyone who measures the difference and wants it back.

A false positive from `nvidia-smi` present without a loaded driver lands on the
cheap side of that same asymmetry.

**What this ADR does not claim.** It fixes the failure that was observed. Whether
the *original* Wayland black-window reports share this mechanism is not
established here, and ADR-022's account of them is left standing.

---

## ADR-034 — The tree appears in under a second at any size; whole-tree work is background work

**Status:** `ACCEPTED` · 08/09/2026 · amends
[ADR-027](#adr-027--not-being-able-to-watch-is-a-state-of-the-workspace-not-a-failure)
and [ADR-032](#adr-032--quick-open-matches-a-cached-path-list-and-the-tree-invalidates-it)

**Context.** The owner opened `~/x` — around 160 repositories with
`node_modules/`, `target/` and `.git/` — and the Welcome screen stayed on screen
for over two minutes before the tree appeared. That folder is not a notes
workload, and it does not have to be: **the application may not freeze on any
folder.**

The measurement is in `docs/DECISIONS-0.1c.md` D-09 and reproduces with
`tools/gen-deep.sh` plus `cargo test -p notes-core --test deep -- --ignored`. On
20 962 directories, opening the workspace and listing the root cost **1.13 ms**
together — the lazy tree was never the problem — while `start_watch` cost
**502 ms** and the first `quick_open` **550 ms**, each walking the entire tree
inside a `#[tauri::command]` holding `Mutex<WorkspaceService>`. Every other
command, `tree_list` included, waited behind them. On a folder ten times the
size, so did the user.

**Decision.** Three rules, and the first is an acceptance criterion:

1. **`workspace_open` returns and the tree appears in under one second, at any
   size.** `crates/notes-core/tests/deep.rs` asserts it against the deep fixture;
   `docs/ACCEPTANCE-0.1b.md` and `docs/ACCEPTANCE-0.1c.md` carry it with the
   number.
2. **Anything that needs the whole tree runs off the critical path** — on its own
   thread, cancellable, with its progress visible in the status bar. That is the
   watcher's per-directory walk and quick open's path list today, and it is the
   rule any future whole-tree work is held to.
3. **Partial is a state, and it is reported with numbers.** A directory that
   cannot be read is counted and skipped, never fatal. A full watch table
   degrades **only the excess**: the watches already installed keep working, the
   remainder is counted, and the banner says how many and which `sysctl` raises
   the limit.

Rule 3 is the amendment to ADR-027. That ADR made "watched" and "polled" the two
states of a workspace; the truth has a middle — mostly watched, with a named
number of directories that are not — and the interface now says so instead of
collapsing it to the worse of the two ends. Rule 2 is the amendment to ADR-032:
the cached path list is still a cached path list, but it is **built in the
background** and `quick_open` answers from a partial one with `building: true`
beside it, rather than blocking on the first call. ADR-032's *"every operation
that changes the tree drops the cache"* survives with one qualification that
only a background walk needs: a walk still running is not restarted, because on
a workspace with continuous activity it would never finish
(`DECISIONS-0.1c.md` D-11).

**Consequences.** Quick open can answer from an incomplete index for the first
few seconds on a very large workspace, and says so in the palette. The watcher
does not descend into `node_modules/`, `target/` and their kin
(`docs/DECISIONS-0.1c.md` D-08) — changes there arrive via the 5 s scan instead
of instantly. Both walks skip symlinked directories, which the deep fixture's
loop exists to check.

The freeze had one more property worth keeping in view: it was invisible in
every fixture the project had. `fixtures/large` measures 10 000 **notes** and
lists them in 37 ms. The axis that broke was **directories**, and nothing
measured it until `fixtures/deep` did. A performance criterion is only as good
as the shape it is measured on.

---

## ADR-035 — The bundle version is stamped from `version.md`, never maintained beside it

**Status:** `ACCEPTED` · 08/09/2026

**Context.** `version.md` is the version (ADR-011): `tools/release.sh` names the
git tag and the GitHub Release after it. `tauri.conf.json` needs the same number
to stamp a `.deb`, an AppImage and a `PKGBUILD`, and it held its own copy —
`0.1.0`, while `version.md` said `0.11.1`. A package attached to Release
`0.11.1` that calls itself `0.1.0` cannot be matched to the code that produced
it, which is the one thing a version number is for.

**Decision.** `tools/stamp-version.sh` writes `version.md`'s version into
`tauri.conf.json` at build time. **What is committed is `0.0.0`** — valid
semver, and unmistakably not a release. `tools/check.sh` and CI both fail if the
committed value is anything else, so a stamped tree cannot be committed by
accident and the number cannot quietly acquire a second maintainer.

The AUR package follows the same line: `packaging/aur/notes-bin/PKGBUILD` is
**generated** from `PKGBUILD.in` by `packaging/aur/gen-pkgbuild.sh`, which fills
in the version, the source and its `sha256sum`. The generated file is
gitignored. A checksum committed by hand is a checksum that is eventually wrong.

**Consequences.** The version lives in exactly one file, and every artefact that
carries a version gets it from there. The cost is that a local
`npm run tauri build` produces a `0.0.0` bundle unless `tools/stamp-version.sh`
is run first — which is stated in the runbook and is the correct default, since
a local build is not a release.

**Alternative if you disagree.** Keep the number in `tauri.conf.json` and bump
both. It is one more line in the commit ritual and it is the line that gets
forgotten; the evidence is that it already had been, by ten minor versions.

---

## ADR-036 — Release artifacts are built for minor bumps, and on request

**Status:** `ACCEPTED` · 08/09/2026 · refines
[ADR-023](#adr-023--arch-linux-is-a-release-target-with-its-own-ci-job) and
[ADR-035](#adr-035--the-bundle-version-is-stamped-from-versionmd-never-maintained-beside-it)

**Context.** ADR-011 makes every commit a version and every version a Release.
`build.yml` then built a `.deb`, a 105 MB AppImage, a tarball and an Arch
package for each of them — nine minutes a time. In one working session that was
twelve full builds, three of them running concurrently while the CI job that
actually gates the work sat queued behind an AppImage. The owner's call:
*"9 min e 105 MB por commit de doc não se justifica."*

**Decision.** Artifacts are built for a **minor bump** — a version whose patch
component is `0` — and for any version asked for explicitly through
`workflow_dispatch`. A patch Release carries no artifacts and **says so in its
own description**, with the marker `<!-- no-artifacts -->` so a re-run does not
append the note twice.

Serialisation stays as well: the concurrency group is `build` with
`cancel-in-progress`, so even a burst of minor bumps produces one build.

**Consequences.** The version a user installs is a version where something an
installer contains actually changed. A patch that somebody does want packaged
is one manual run away, and the Release itself carries that instruction rather
than leaving an empty downloads section to be read as a failed build. What is
lost is the invariant *"every Release is installable"*; what replaces it is
*"every Release says whether it is"*, which is the honest version and the one a
person can act on.

The rule is arithmetic on the version string, not a diff of what changed. A
patch that touches the editor gets no artifacts even though it changes the
binary, and a minor bump that only moves documents gets a full set. Deciding by
content would mean defining which paths count, keeping that list correct, and
explaining an empty Release whose commit *looks* like code — the version number
is a decision the author already made, and reading it is cheaper than
second-guessing it.

**Alternative if you disagree.** Build on every version, which is what shipped
at `0.11.2` and cost the CI queue; or decide by which paths a release range
touched, which trades nine minutes for a rule that has to be maintained and can
be wrong in both directions.

---

## ADR-037 — Milestone 0.1d exists: the interface is a milestone, not a finishing pass

**Status:** `ACCEPTED` · 08/09/2026 · owner's decision, recorded here ·
amends `.continue/SCOPE_final.md` §17

**Context.** 0.1a, 0.1b and 0.1c built a Markdown editor and its behaviour is
tested on four platforms. What none of them built is an *interface*: the window
is a tree, a text area and a status line, and the twenty-five interface flows in
`ACCEPTANCE-0.1b.md` and `ACCEPTANCE-0.1c.md` had been waiting for a person to
walk them.

The owner installed the `0.11.11` `.deb`, opened it, and did not walk them —
because the interface is about to change entirely and walking flows against a
layout that is being replaced measures nothing. In the same pass they hit the
concrete consequence of never having had an interface pass: **there is no way to
change workspace without going back to the Welcome screen.** Every command
exists in the core; nothing in the window reaches them.

**Decision.** §17 gains **0.1d — Interface**, and the desktop MVP becomes
`0.1a + 0.1b + 0.1c + 0.1d`. The layout reference is Obsidian's dark interface.
Icons are `lucide-react` (ISC).

**The copying rule, which is the part that matters legally and is stated in the
scope rather than left to judgement:** palette, spacing and structure are free —
they are ideas, and an interface layout is not a protected work. **No Obsidian
theme file, stylesheet or asset is copied. Everything is rebuilt.** A CSS file
is a work; the observation that a note editor reads better in a 700-pixel column
is not.

The twenty-five flows of 0.1b and 0.1c are **re-indexed into
`ACCEPTANCE-0.1d.md`** rather than ticked where they are: the steps move in the
interface, the behaviour does not, and a flow whose steps no longer describe the
window is not a flow anyone can walk.

**Consequences.** The MVP ships later, and it ships as something a person can
use rather than something a test can prove. The workspace selector alone closes
a defect that had no route to the user at all. The cost is a milestone that
produces almost no core code and a great deal of frontend, at a point where the
project's testing strength is in the core — which is why the automated half of
0.1d is contrast, keyboard navigation and the standing dialog check, and why
everything else is a flow with a person's name on it.

**Alternative if you disagree.** Ship the MVP on 0.1c and treat the interface as
polish inside 0.2. That is what "finishing pass" usually means and it is why so
many applications never get one: there is always an index to build.

---

## ADR-038 — Graph view leaves "out of scope" and becomes 0.3, after backlinks

**Status:** `ACCEPTED` · 08/09/2026 · owner's decision, recorded here ·
amends `.continue/SCOPE_final.md` §18

**Context.** §18 listed graph view among the features that do not enter "until
further order", beside canvas, plugins and a marketplace. The owner has given
that order.

**Decision.** Graph view moves out of §18 and into **0.3**, explicitly *after*
backlinks. **It is not part of 0.1d.** The icon rail 0.1d builds carries a graph
entry that is **disabled, with a tooltip naming the milestone** — which is the
honest way to show a thing that is coming: visible, inert, and dated.

**Why after backlinks and not before.** A graph is a rendering of a link
relation; backlinks are that relation. Building the view first means inventing a
data source for it, and then rebuilding it when 0.3 produces the real one. The
ordering is not a preference about interest, it is which of the two can exist
without the other.

**Consequences.** §18 stops being the list of things that will never happen and
becomes the list of things that have not been ordered yet, which is what it
always said it was — *"Recurso fora de escopo não entra como 'melhoria
incidental' de um agente. Entra por revisão deste documento."* This is that
revision, made by the owner.

A disabled control in the interface from 0.1d onward is a promise with a date on
it. If 0.3 moves, the tooltip is what has to be corrected — it is a string, and
it is a key like every other.

**Alternative if you disagree.** Leave graph view out of §18 *and* out of the
interface, and add it whenever it arrives. That avoids a disabled control
carrying a promise, at the cost of an icon rail that has to be relaid out later
and a user who cannot see the shape of what is coming.


## ADR-039 — SQLite stores facts; the core owns workspace I/O

**Status:** Accepted, implemented in 0.14.0 (milestone 0.2).

**Context.** ADR-003 deferred the index boundary until real code needed it;
ADR-015 requires operational identity to move to a separate SQLite database.
The user requested all of 0.1d and 0.2 together; owner acceptance still follows
the installed-release and following-release rule.

**Decision.** `notes-index` depends on `notes-model`, `notes-markdown` and
bundled `rusqlite`, never `notes-fs`. Core walks the confined filesystem and
supplies metadata/text. Index persistence owns FTS5 and derived parser facts;
registry persistence owns a separate operational DB. The initial registry
schema stores the existing serialized identity snapshot to preserve its full
shape during migration. Immediate transactions merge deltas against a baseline
and refuse conflicting changes; a stale snapshot cannot erase unrelated notes.
Legacy JSON and a backup are retained. Persistence remains immediate, correcting
the earlier unimplemented debounce described in architecture §4.1.

Recent history and reviewed rewrite originals are operational app-data files,
not derived index tables. Link/image destinations are parsed by the existing
Markdown authority and rewritten only at exact source spans after review.
Concurrency checks and original-byte journals make partial application
recoverable; no claim of a filesystem transaction is made. Notes not examined
are disclosed; an unavailable index offers an explicit move without rewriting.

**Alternatives.** Giving the index direct filesystem access duplicates the root
boundary. A third registry crate adds no useful isolation between the two DB
lifecycles. Normalized registry rows can replace the snapshot in a later backed-up
migration if measurements justify it; the current per-note merge semantics must
survive that change. Tags, graph and wiki-link semantics remain 0.3.

---

## ADR-040 — Mobile starts with the iOS container

**Status:** `ACCEPTED` · Original decision 09/09/2026; integration reviewed 10/09/2026

**Context.** The owner started mobile independently while desktop milestones
0.2 and 0.3 were being delivered. The original PR proposed completing mobile
before 0.3; 0.3 has since shipped, so that sequencing statement is historical.
The owner now requests this foundation be merged and work continue on 0.5.

**Decision.** Start mobile validation with iOS and the application's own
container, where LocalFs applies. External folders require security-scoped
bookmarks on iOS and SAF on Android in a later slice. Toolchain availability
motivated iOS first at the time; it is not a permanent claim about a machine's
installed SDKs. Distribution credentials and physical-device acceptance remain
separate from compilation for the simulator.

**Consequences.** This foundation gates the unsupported trash dependency and
reports permanent deletion on mobile. It does not supply the Tauri mobile
entry point, generated projects, layouts, external-folder adapters or device
proof. Those remain in the mobile queue.

---

## ADR-041 — Local knowledge and agents share the parser and guarded core

**Status:** `ACCEPTED` · 10/09/2026

**Number reservation at publication.** ADR-040 was reserved by mobile PR #2.
Its integration in 0.17.0 filled that reservation and assigned ADR-042 to its
other decision, preserving the already published ADR-039.

**Context.** The owner requested all of milestone 0.3 while another agent
implements mobile in PR #2. Knowledge relationships need one semantic source;
MCP must run without the app and must not silently overwrite app buffers.

**Decision.** The Rust Markdown crate uses pulldown-cmark's wiki syntax and
`serde_yaml_ng` 0.10 for bounded, read-only YAML interpretation. It never
serializes metadata into notes. Core resolves wiki homonyms explicitly and
derives backlinks/graph from index schema 2. Frontend shows the same facts,
with bounded graph rendering and partial-state labels. The `image` crate
with PNG/JPEG decoders validates clipboard bytes under allocation/dimension
limits; original bytes are exclusively created in root attachments.

`notes-mcp` is a stdio executable over core, without a listener or application
dependency. Configuration explicitly scopes paths and independent permissions;
review writes stay in proposals. Note contents never grant authority. Global
enrollment locking gives simultaneous processes the same workspace ID, while
the existing workspace lock serializes identity assignment and mutations.
Disk hashes are always compared even when metadata matches: a two-process
test demonstrated that matching size/mtime could otherwise destroy a newer
write before the registry reported its conflict. Append receipts persist in
operational app data to make retries across process restarts idempotent. Agent
reads do not alter GUI visit history. Configuration and operational limits
are [documented separately](KNOWLEDGE-0.3.md).

**Consequences.** Disposable schema 1 indexes rebuild; operational schema stays
unchanged. Saving reads the current bytes even on a metadata match. Coordination
requires a shared app-data directory and cannot exclude unrelated editors from
the filesystem. Linux releases publish a standalone MCP archive; unsigned
macOS/Windows publication remains disabled under ADR-024. Version 0.16.0 avoids
the mobile PR's reserved 0.15.0; merging the PR must choose a newer version.
Owner acceptance remains the installed-release walk and following-release
repeat; automated process/UI tests do not tick those boxes.

---

## ADR-042 — Mobile remains in the same repository and application

**Status:** `ACCEPTED` · Original decision 09/09/2026; numbering reconciled 10/09/2026

**Context.** The mobile PR used ADR-039 before the index decision occupied that
number on master. Its architectural choice remains valid; its number changes
here without renumbering an already published decision.

**Decision.** iOS and Android are targets of notes-app and consume the same
core. Mobile UI will be a layout in the existing React application. Generated
Apple/Android projects will be committed when created, since their signing,
entitlement and manifest edits are source. They do not exist in this PR.
A separate repository or second React application is not introduced, and
packages/ui is deferred until real sharing requires it.

**Consequences.** Filesystem interfaces and their consumers evolve in one
commit and one CI matrix. The current iOS CI check covers core dependencies,
including bundled SQLite and clipboard image decoders; compiling the full
Tauri shell, Android runtime integration and device flows remain future work.

## ADR-043 — A separate owner-operated REST server reuses core policy

**Status:** ACTIVE · 10/09/2026 · Implemented in 0.18.0.

**Context.** The owner requested milestone 0.5 after reviewing and merging the
mobile foundation. Server access needs revocable, scoped credentials without
turning the desktop app into a network service or inventing another note store.

**Decision.** Add `server/notes-server` using Axum 0.8.9 and Tokio, with an
operator CLI and a path-only `/v1` REST API documented in OpenAPI 3.1. Core owns
scope checks, note IO, identity coordination, conditional writes, append
receipts and saved-content search. Paged core results remain scope-filtered.
Complete BaseRev comparison prevents consuming a stale conditional revision;
a timestamp never chooses a sync winner. This is not the 0.6 sync protocol.

There is one owner, with distinct random bearer credentials per integration.
Only secret digests are persisted; comparison is constant-time. Operator CLI
creation/revocation is outside HTTP. A shared administration lock surrounds
request authorization and its core operation; revocation takes the exclusive
lock. A separate lifetime lock excludes backup from a live server. State schema
1 is explicit; unknown versions are refused. Offline restore stages the data,
rejects traversal/links, rebinds restored enrollment through the core with a
pre-restore copy, and publishes only a new directory. Notes stay byte-identical.

The process defaults to loopback. Remote use requires the fixed private
backend/known HTTPS proxy topology documented in SERVER-0.5.md. Compose exposes
only Caddy; the backend rejects an untrusted actual peer or a missing HTTPS
forwarded-protocol header. Eight-operation concurrency and fixed identity/peer
rate windows bound requests. Request bodies, result pages and audit retention
are bounded. Audit records authorship and an opaque route reference, never
credentials, note text, queries or full note IDs/paths. Filesystem trash failure
logging is generic so a server fallback cannot disclose a note path.

**Explicit CSRF amendment to security.md §4.3.** These machine endpoints use
non-cookie bearer authorization instead of a session CSRF token or per-request
signature. No cookies authenticate, no CORS is enabled, and every Origin-bearing
request is denied. This deliberate exemption applies only to the documented
`/v1` API; it grants no exemption to a future browser/session interface.

**Consequences.** The desktop still opens no listener. Linux releases include a
separate server archive; CI builds the non-root container and verifies actual
TLS using a test CA. The public certificate setup remains an owner deployment
step. Server state is readable by its operator: no E2EE, teams, hosted service,
desktop sync or remote MCP is introduced. Dependencies and container digests
are tracked by Dependabot and Cargo.lock. The owner acceptance walk and its
following-release repeat remain open in ACCEPTANCE-0.5.md.

## ADR-044 — Synchronization starts with causal plans and explicit pairing

**Status:** ACTIVE · Implemented domain boundary in 0.19.0; milestone 0.6 open.

**Context.** The owner requested the next queued items after the 0.5 server.
ADR-005 already requires identity, revisions, tombstones and no clock-based
winner. Applying remote writes without a causal model would turn synchronization
into overwrite-by-arrival and bypass the local save protocol.

**Decision.** Create `notes-sync` beside the filesystem adapter. Its first block
contains immutable revision DAGs, compare-and-set heads, monotonic device
receipts, explicit two-parent resolution and deterministic pairing/incremental
plans. It depends on the domain model, never on Tauri or an HTTP implementation.
Core supplies explicit inventories with original byte hashes and its existing
identity/reconciliation rules. A separate `notes-sync-plan` executable exposes
those inventories as a read-only preview for two mounted folders.

Schema-1 sync metadata is private operational data and uses locked, atomic,
digest-conditional replacement. Future/corrupt state is refused unchanged.
Limits stop growth explicitly; acknowledgments do not authorize pruning.
Missing inventory is never a deletion, and a tombstone retains ancestry.
Upload/download require an empty destination inventory; reconciling two
populated folders produces explicit links and conflicts before any application.
Filesystem-specific name checks remain mandatory at the eventual apply step.

**Consequences.** Planning creates no source files and changes no note bytes;
explicit inventory may establish identities in operational state. The desktop
continues to open no listener. No remote wire format, background transfer,
content retention or UI completion is claimed by this first block. Those
remaining items stay in `.continue/0.6-sync.md`; MCP remote is still 0.7.
See SYNC-0.6.md for the implemented contract and limits.

## ADR-045 — The first server sync transport stores revisions before application

**Status:** ACTIVE · Implemented in 0.19.1; milestone 0.6 remains open.

**Context.** After causal planning, replication needs durable original bytes and
an authenticated resumable exchange. A storage acknowledgment cannot honestly
claim that a revision was applied to a source file or a dirty device buffer.

**Decision.** Add a separate `/v1/workspaces/{workspace}/sync/revisions` inbox
under the existing bearer, HTTPS, rate, concurrency and audit controls. Reuse
operation permissions and review scopes. Authorize every historical note path;
a token cannot recover old content through a UUID after the note leaves its
scope. The existing machine-only CSRF exemption covers these `/v1` endpoints.

The first vault is a bounded atomic document containing causal metadata and
base64 original content together, with verified hashes, expected-head checks
and immutable UUID retry receipts. It acknowledges storage only. No source file
is read or written through the vault, and no application receipt is invented.
Parents must already be accepted; divergent branches remain on the device until
a later conflict/import flow. Append-log cursors enable metadata paging; filtered
entries still advance the cursor and may reveal aggregate workspace activity.
Device UUIDs are asserted metadata, not authentication principals.

Retain all accepted revisions and tombstones within the documented byte/count
bounds; refuse capacity without eviction. No automatic pruning or migration is
introduced. Per-workspace OS locks serialize writes, atomic replacement binds
content to heads, and offline backup/restore includes the vault but not its lock.
Corrupt or future-schema state is refused unchanged. Original workspace files
remain the source of truth; the inbox contains replication copies.

**Consequences.** This block can transfer and recover immutable bytes but cannot
synchronize a user's workspace by itself. Device outboxes, conflict import,
application with draft protection, attachments, background work and UI remain
queued. The bounded JSON design deliberately limits scale; its replacement
needs a documented migration and recovery path. See SYNC-0.6.md and OpenAPI.

## ADR-046 — Device transfer uses durable queues before source application

**Status:** ACTIVE · Implemented in 0.20.0; milestone 0.6 remains open.
Explicit source application is added separately by ADR-047 in 0.20.1.

**Context.** The server inbox can store immutable revisions, but a device must
survive offline edits and lost responses without regenerating UUIDs or treating
a storage receipt as proof of source application.

**Decision.** Add `notes-sync-client` as a separate desktop CLI/transport crate.
Core captures saved original bytes against its identity/hash inventory. The
shared publication contract and byte validation live in `notes-sync`; neither
core nor domain imports an HTTP client. Explicit initialization pins endpoint,
workspace UUID and source root. The first upload requires an empty server inbox;
the first client only supports whole-workspace non-review credentials.

Offline staging atomically persists immutable publications in order. Transfer
checkpoints each validated receipt and reuses the exact publication on an unknown
outcome. Conflicts retain the rejected publication and successors. Receive
validates complete pages/content before committing its cursor. Received bytes
can be exported to a new private state file, never applied to a source note.
Missing source files are reported, not inferred as deletions. Draft protection
and application acknowledgments are not claimed by this block.

Use pinned reqwest 0.13.4 with rustls and the ring provider, bounded responses,
timeouts, no redirects, no automatic retries and no inherited proxies. The
operator-selected origin is the only network authority; no received URL is
followed. Resolve, validate and pin addresses per process while retaining TLS
hostname verification. Credentials come from a private regular file and remain
outside persisted client state and command-line values.

**Narrow amendment to security.md §4.4.** Explicit initialization with
`--allow-private` can authorize that selected private server, including plain
HTTP only at a literal loopback IP for local use. Link-local/metadata,
unspecified and multicast addresses remain denied. This does not authorize URLs
from note content, HTTP responses or arbitrary future integrations. A selected
PEM trust anchor augments certificate trust without disabling verification.

**Consequences.** Atomic schema-1 client state contains bounded queued and
received content. It must be backed up while stopped and preserved on conflict;
unknown/corrupt schemas are refused unchanged. Retention/migration, scoped or
populated pairing, deletion capture, divergent import, source application,
background work and UI remain in the queue. CI tests real client processes over
TCP and the existing HTTPS proxy, in addition to fault-injected lost receipts.


## ADR-047 — Apply received notes only through guarded closed-workspace writes

**Status:** ACTIVE · Implemented in 0.20.1; milestone 0.6 remains open.

**Context.** Durable transfer is not proof that a note was safely applied. The
editor owns buffers outside the transport process, and a crash can separate a
source write from the receipt that records it.

**Decision.** Extend the CLI with explicit receive-only application of creations
and same-path updates. Keep HTTP outside core. Core acquires an exclusive
workspace activity lease against shared leases held by open core workspaces,
then the existing write lock. The actual app data directory is required and
pinned. Any pending draft blocks application; updates require the observed
BaseRev and local identity, and creations refuse collisions. Core preflight
precedes the durable client intent; every source write follows it. LocalFs
creation publishes a synced temporary file without replacing an existing file,
so incomplete new bytes cannot appear as the destination. Unix new files use
0600. Existing atomic updates preserve their mode.

A separate schema-1 application checkpoint maps remote revisions to local
identity/BaseRev and persists intent before writing. Exact intended bytes allow
recovery of a lost receipt without another write; different content refuses
recovery. Successful receipts advance individually. Transfer receipts and server
state do not become device application acknowledgments. Existing schema-1
transfer state remains readable without migration.

**Consequences.** Updated cooperating processes must share the app data path and
workspace path; older binaries and unrelated editors cannot be protected by the
lease. Native root identities unify activity leases where available. The normal
filesystem adapter's external-writer race limits still apply. Renames, deletes,
conflict resolution, active buffers, server acknowledgments and mobile lifecycle
remain separate work. Owner installed-release acceptance is not inferred from
CLI tests. Server backup skips activity lock files; device backups must retain application
checkpoints.

## ADR-048 — Application acknowledgments report durable device receipts

**Status:** ACTIVE · Implemented in 0.20.3; milestone 0.6 remains open.

**Context.** Storage acceptance and a successful local application are different
facts. The server needs explicit device progress without inferring it from
received bytes or requiring connectivity during filesystem writes.

**Decision.** A separate receive-client `acknowledge` command sends one immutable
workspace/device/revision assertion per durable application receipt, up to 20
per invocation. It advances an additive default-zero cursor only after validating
the server's exact echo. Lost responses retry the same receipt. Application
intents and cached content never authorize sending. HTTP stays outside core.

The server requires Read and entire-history scope visibility and binds each
device to its first acknowledging credential ID. Same-revision retries are
idempotent; ancestors of an acknowledged revision are refused. Atomic vault
persistence retains acknowledgments and credential ownership alongside history,
under existing request and storage limits. Existing vaults have no owners;
existing client checkpoints have zero acknowledged receipts. Older binaries
refuse the additive fields, preventing silent loss on downgrade.

**Consequences.** A receipt is a historical assertion from an authenticated
client, not independent proof of current disk bytes. No pruning, source mutation
or new credential authority follows from it. Credential rebinding and stale
backup reconciliation remain explicit future work; retries never reset identity
or roll server progress back. Active-editor application remains queued under
ADR-047's existing closed-workspace guard.


## ADR-049 — Admit exclusive sync hosts before opening buffers

**Status:** ACTIVE · Core foundation in 0.20.5; frontend integration follows in ADR-050.

**Context.** Applying with an editor open needs a coherent snapshot of buffers
that live outside core. Upgrading an existing shared OS lock risks a gap in
ownership, and the CLI cannot discover the app's unpersisted edits.

**Decision.** Add opt-in exclusive workspace admission before a host opens any
buffers. Reuse the existing activity lease for the session lifetime. Refuse
admission when that service already has a workspace; do not release an existing
shared lease to try an upgrade. Ordinary app/MCP sessions stay shared and the
CLI's closed-workspace requirement from ADR-047 stays in force.

Expose single-revision application on an exclusively owned service. The host
must freeze editing and supply every buffer's identity, BaseRev and saved/current
versions. Core rejects dirty, stale, duplicated or suspended buffer state and
pending drafts before durable intent. Reuse the existing guarded source write;
return the local receipt and allow clean reload through the existing note API.
Invalidate the path index after successful application.

**Consequences.** This implements the core seam, not an editor sync feature.
The frontend barrier, inactive-pane accounting, durable client adapter, error
recovery/reload and UI remain queued. Omitted frontend buffers cannot be detected
by core. No automatic flush, draft deletion, lease upgrade, network listener or
server acknowledgment is added. Other cooperating processes using the same app
data are excluded for the entire exclusive session, not just its writes.


## ADR-050 — The editor owns an input barrier around received application

**Status:** ACTIVE · Implemented in 0.20.6.

**Context.** Exclusive core ownership does not freeze the frontend's document.
A batch may apply some revisions before a later failure, and a missing response
does not prove no source write occurred.

**Decision.** Add explicit app controls for an already initialized receive queue.
The Tauri shell delegates queue/session admission and application to
`notes-sync-client`, which reuses its durable intent/receipt protocol and calls
core without closing the exclusively owned workspace. HTTP stays in the client
crate, but these app commands make no transport calls or open listening ports.

The frontend snapshots its one live buffer (inactive tabs have no buffers and
Split is a preview). Dirty/draft/conflict/writing state refuses application.
Admission requires no pending IPC response continuations, no active composition,
and no outstanding modal. A synchronous barrier gates ordinary IPC, user input,
CodeMirror edits, autosave and reconciliation. The barrier owner alone invokes
application/recovery commands. Input resumes only after verified clean reloads
are installed into the exact frozen document. A missing response or failed
reload retains the barrier with an explicit recovery control.

The client advances buffer BaseRev snapshots between revisions and reloads after
partial failures. Successful earlier receipts stay durable. The shell owns the
selected Store for the session; source binding is checked by the client/core.
A normal app restart does not silently opt into exclusive mode.

**Consequences.** Received creations/updates can be applied with the app open.
Transport, acknowledgment, pairing initialization and credentials remain CLI
operations. Multi-buffer editing must extend the inventory before enabling this
workflow. No automatic flush, draft deletion, overwrite-on-conflict, network
listener, or server acknowledgment follows from the UI action.


## ADR-051 — Import divergent history only with its explicit resolution

**Status:** ACTIVE · Implemented in 0.20.7; extends ADR-045's inbox protocol.

**Context.** A rejected linear publication remains queued, but its peer cannot
resolve with two parents until the divergent history is available. Importing a
branch as a head would choose a winner before the operator resolves it.

**Decision.** Add an optional bounded `branches` field to immutable publications.
The branches retain original revisions and bytes; only the enclosing two-parent
resolution changes the head, under the existing expected-head compare-and-set.
Parents precede children, every imported revision belongs to that note and leads
to the divergent parent, and graph/hash/capacity checks include retained history.
Authorize every imported mutation and historical path. Import plus resolution is
one vault transaction; stale, forbidden or invalid submissions change nothing.

The client provides independent fetch, conflict inspection, private export and
explicit file-based resolution. It durably replaces selected pending envelopes
only when their original histories and bytes are retained in the new envelope
or already verified in the remote journal. No source write or network operation
is implicit in choosing result bytes. Receive application keeps its existing
revision guards and applies only accepted heads, not imported branch values.

**Consequences.** Linear wire JSON stays unchanged; old readers reject envelopes
with branches. Both sides must be upgraded. Branches share existing byte/revision
quotas and have a 20-revision per-envelope limit. Version 0.20.7 supports same-path live-note resolutions in whole-workspace
upload queues; ADR-052 extends the result choices. Receive-folder local conflicts,
pairing and editor conflict controls remain open.


## ADR-052 — A rename or deletion resolution requires an explicit result choice

**Status:** ACTIVE · Implemented in 0.20.8; extends ADR-051's client workflow.

**Context.** A file-only resolution cannot express which path survives a rename,
or distinguish an intentionally empty note from deletion. Inferring either
choice would discard user intent.

**Decision.** Keep the existing same-path live-note `resolve` contract. Add
`resolve-to` with a workspace-relative result path and a result file, and
`resolve-delete` with a path and no content. Both consume exact divergent heads
of one note and use the same bounded two-parent publication and retained branch
bytes. They stage history only; they do not change source files. Server checks
still require Create for resurrection, Move for changed paths and Delete for
tombstones, including the permissions of imported branches.

**Consequences.** Upload conflicts involving renames and tombstones have explicit
resolution commands. Missing files and empty bytes never infer deletion. Source
rename/deletion application, local receiver conflict capture and editor controls
remain queued. Existing source guards and wire/schema versions are unchanged.


## ADR-053 — Receiver conflicts retain capture and application progress separately

**Status:** ACTIVE · Implemented in 0.20.9 for closed-workspace same-path live notes.

**Context.** A local receiver edit differs from its last source-application receipt.
Replacing that receipt with a captured edit would falsely claim remote progress;
applying intervening remote revisions would overwrite the edit before resolution.

**Decision.** Capture an explicit remote note UUID through its applied local
identity and the actual pinned app data directory, under an exclusive core
session with no drafts. Persist the original bytes as a local branch plus its
observed source revision in client state. Resolve with two parents before upload;
receive mode gains no credential permissions. Capture and choice write no source.

Apply the fetched chosen result under a durable resolution intent and the captured
source precondition. Record same-note ancestor revisions as superseded without
source writes or application acknowledgments. Record unrelated interleaved entries
as deferred and let ordinary guarded application drain them first. Application
state validates these position sets against the immutable history. Acknowledgment
stops at deferred entries and skips superseded ones; counts represent receipts.
Both resolution writes and deferred ordinary writes recover through durable intents.

**Consequences.** Saved same-path receiver conflicts have a complete explicit CLI
workflow without sacrificing either branch or falsifying receipts. Keep the source
closed and stable during the workflow; newer external edits refuse application.
Receiver rename/delete application, editor controls and refreshing unresolved
captures remain separate work. Additive private state fields fail closed in older
readers. Source files remain the user's Markdown, never the queue or SQLite.

**0.20.10 extension.** A remote rename/tombstone may be an ancestor of the
chosen live receiver resolution at its applied path. Require `resolve-to` for
this restoration; the implicit same-path command still refuses these parents.
Superseding those ancestors executes no filesystem effects and creates no
receipts for them. Local move/delete choices remain outside this operation.

**0.20.11 extension.** Explicit recapture appends a saved-edit revision to the
previous captured branch, not to a resolution that was never applied locally.
Keep the original application anchor and replace only the observed capture
precondition. Retain all pending capture bytes; require publication of a prepared
choice first. Refuse unfinished application intents and unchanged bytes. New
resolution remains mandatory, with existing branch/byte limits and no false
application receipt for the earlier published choice.


## ADR-054 — Receiver path effects have durable intent before source changes

**Status:** ACTIVE · Implemented in 0.20.12.

**Decision.** Apply explicit move/tombstone resolutions only in a closed exclusive
core session, retaining captured bytes in history. A move creates the chosen target
without replacement before guarded removal of the original; its destination parent
must exist. Keep identity across recovery and persist the receipt only after all
effects. A tombstone receipt records completed removal; OS trash is not promised.
Collisions, drafts, changed identity and changed BaseRev refuse before mutation.
A durable intent authorizes recovery of partially completed effects. General
rename cycles and automatic source deletion capture remain separate work.

## ADR-055 — Pairing confirms observed identities within a pinned namespace

**Status:** ACTIVE · Implemented in 0.20.12.

**Decision.** Pin the credential's exact workspace/subfolder scope and translate
all paths at the transport boundary. Preserve the server's global cursor even
when scoped pages omit publications. A fresh receive queue previews reconciliation
before confirmation: equal bytes may link identities, local-only files stage
uploads, remote-only files wait for application, and divergent bytes block.
Confirmation binds the snapshot and checks for unseen remote entries. Persist
pairing receipts/baselines with the client state atomically; never pretend that
older superseded publications were applied. Retaining both divergent files requires
an explicit local rename and fresh preview. No new server permission or endpoint
is introduced, and enrollment does not imply automatic two-way synchronization.

## ADR-056 — Explicit note effects use ordered recoverable publications

**Status:** ACTIVE · Implemented in 0.20.13.

**Decision.** Missing inventory entries require an explicit current-head tombstone
request. Correlate closed rename cycles only with unique native identities and
unchanged bytes; order their publications through a temporary same-directory
path. Apply each effect with the existing exclusive core guards and durable
intent. Source capture never mutates user files. Ambiguous external cycles are
not inferred from hashes alone.

## ADR-057 — Referenced attachments travel with immutable note revisions

**Status:** ACTIVE · Implemented in 0.20.13.

**Decision.** The shared Markdown parser selects local references. Capture exact
binary bytes through core, retaining optional bounded manifests with primary
and divergent revisions. Validate references, hashes and scope, including all
historical manifests. Keep Markdown unchanged. Apply attachments with individual
BaseRev checks and durable intents before the note receipt; interrupted bundles
resume but are not multi-file transactions. Preserve locally changed files and
retain branch exports. Do not delete assets merely because references disappear.

## ADR-058 — The desktop schedules transfer independently of source application

**Status:** ACTIVE · Implemented in 0.20.14.

**Decision.** A native worker in `notes-sync-client` owns bounded transport passes,
serialized independently of the editor mutex. Persist one active queue and an
opt-in schedule, storing only the path to an operator-managed credential file.
Reuse the device transport's existing endpoint policy; note-derived URLs have
no access to this channel. Host observations are conservative and expire.
Pause cancels between requests; retries preserve original revision identities.
The worker never applies source effects, merges conflicts, or promises execution
after process exit. Desktop controls call typed individual IPC commands for
pairing, review, explicit application, capture/resolution and recovery exports.
