# Decisions taken while building 0.1a

> **Status:** `ACTIVE` · Every call made under the standing rule *"choose the
> simplest option compatible with the scope, write the missing section into
> `ARCHITECTURE.md`, and carry on"*. One row per decision: what was decided,
> which gap it closed, and **what the alternative is** if the owner disagrees.
>
> This is not the ADR log. A decision here that turns out to be structural gets
> promoted to [`decisions.md`](decisions.md); the rest are recorded so that none
> of them is invisible.

Gap numbers refer to the eighteen listed in the 07/09/2026 review.

---

## D-01 — Case sensitivity is probed by reading, not by writing a temp file

**Decided.** The probe `list()`s the root, picks an entry with a cased letter,
and `stat()`s that name case-inverted; equal `native_id` on both → insensitive,
flipped name absent → sensitive, no usable entry → `Unknown` treated as
insensitive. Persisted in `registry.json`, self-correcting in **both**
directions. `ARCHITECTURE.md` §3 rewritten.

**Gap closed.** G9, and the mechanism half of decision **C7**.

**Why it deviates from C7.** C7 specified *"escreve temp, testa fold, apaga"*.
Writing a temporary file into the workspace at open time contradicts two things
that outrank the mechanism: scope §2.3 — *"Abrir uma pasta não a modifica.
Nenhum arquivo ou diretório é criado na pasta sem ação explícita do usuário"* —
and the 0.1a acceptance criterion *"Abrir uma pasta não cria nenhum arquivo
nela"*, which would become untestable-as-written. The **intent** of C7 is kept
whole: no assumption from the operating system, and correction in both
directions.

**Unknown resolves to insensitive**, and that asymmetry is deliberate: treating a
sensitive filesystem as insensitive refuses a legitimate name, while the reverse
lets `note_create` pass its `CompareKey` collision check and overwrite a note.
One is an inconvenience, the other is data loss.

**Alternative if you disagree.** Restore the write probe and relax the acceptance
criterion to "leaves no file behind" instead of "creates none" — one line in
`ARCHITECTURE.md` §3 and one in the acceptance document. The read probe is
inconclusive only on an empty root or one whose every name is caseless, and both
fall back to the safe default.

---

## D-02 — `.gitattributes` marks `fixtures/**` as `-text`

**Decided.** `fixtures/** -text` (plus `linguist-generated=true`), disabling all
end-of-line conversion for the corpus.

**Gap closed.** G11, and a hole in the acceptance criterion itself.

**Why.** The 0.1a criterion is *"abrir cada arquivo dos fixtures e salvar sem
editar → `git status` limpo"*, and the corpus deliberately contains CRLF, CR-only
and mixed-EOL files. With git's default `text=auto`, git would normalise those
line endings on commit and re-expand them on checkout — on Windows CI it would
hand the test different bytes than the ones committed. The criterion would then
pass or fail on git's behaviour rather than on the application's, which is worse
than not running it.

**Alternative if you disagree.** Drop the file and keep the corpus on Linux CI
only, accepting that the byte-preservation criterion is unverified on the Windows
runner — where it is most likely to break.

---

## D-03 — `.git/` is not shipped inside `fixtures/basic/`

**Decided.** The ignored-directory fixtures are `.obsidian/`, `.notes/` and
`.trash/`. The `.git` entry of `IGNORE_DEFAULT` is covered by a unit test that
creates the directory in a temp workspace.

**Gap closed.** G10 test coverage.

**Why.** Git refuses to track a nested `.git/` directory, so a committed fixture
containing one is silently absent on every clone — a test that passes because
the hazard is not there.

**Alternative if you disagree.** Ship it as `dot-git/` and have the test rename
it before running, which adds a moving part to every test that touches the
corpus.

---

## D-04 — `CoreError::Io` carries a typed `IoKind`, not a `String`

**Decided.** `Io { op, path, kind: IoKind }` with `IoKind` an enum —
`DiskFull`, `PermissionDenied`, `ReadOnlyFilesystem`, `NotFound`,
`IsADirectory`, `Busy`, `NameTooLong`, `Interrupted`, `Other`. Classification
reads `raw_os_error()` first (ENOSPC 28, EDQUOT 122/69, Windows 112/39) and
falls back to `io::ErrorKind`.

**Gap closed.** G5.

**Why.** `ARCHITECTURE.md` §7.3 had `Io { op, path, kind: String, message }`, and
§7.3 also says the frontend switches on `code` and never inspects `message`. With
a stringly-typed `kind`, the only information distinguishing a full disk from a
denied permission sat in a field the contract says not to read — while the 0.1a
acceptance criterion requires exactly that distinction to be visible.

`ErrorKind` alone is not enough: stable Rust does not surface "storage full" for
every platform, and a disk quota is a different errno from a full filesystem
while meaning the same thing to the person typing.

**Alternative if you disagree.** Keep `kind: String` and have the frontend match
on its contents, which makes the i18n catalogue depend on strings the operating
system chooses.

---

## D-05 — `watch()` returns `Unsupported` at 0.1a; `notify` is not a dependency yet

**Decided.** The method is on the trait from the start and every implementation
answers `Err(Unsupported { cap: "watch" })`.

**Gap closed.** Scope §17 puts the watcher and reconciliation at 0.1b.

**Why.** The method has to exist now so `notes-core` is written against a
filesystem that may not have one — the mobile case, not a hypothetical. Pulling
`notify` before anything consumes it would add a dependency and a background
thread to a milestone that has no use for either.

**Alternative if you disagree.** Add `notify` now and let 0.1a emit events
nothing listens to.

---

## D-06 — No property-testing framework; a fixed table of byte shapes instead

**Decided.** `read(write_atomic(x)) == x` is exercised over a hand-picked list —
empty, single byte, NUL run, invalid UTF-8, BOM, every line ending, and sizes at
1, 255, 4095, 4096, 4097, 65535, 65536 and 1 000 003 bytes.

**Gap closed.** The mandatory property of `ARCHITECTURE.md` §19.

**Why.** `ARCHITECTURE.md` §2 pins each crate's dependency list, so adding
`proptest` is a dependency decision rather than a design one. The table covers
what actually breaks a writer: page and buffer boundaries, and bytes that are not
text.

**Alternative if you disagree.** Add `proptest` to `notes-fs` dev-dependencies
and replace the table with a generator; the test names and assertions stay.

---

## D-07 — NFC and case-folding are implemented in `notes-model`, not pulled in

**Decided.** `CompareKey` composes Latin-1 Supplement and Latin Extended-A
sequences (the marks a Portuguese, Spanish, French or German filename carries)
and folds case with `to_lowercase`.

**Gap closed.** Scope §7.6 collision detection, without breaking
`ARCHITECTURE.md` §2's dependency list for `notes-model`.

**Why.** The case that occurs is macOS handing out NFD for accented names. Any
sequence the table does not cover is left alone, which can only make two paths
compare *unequal* — the safe direction, since a missed fold refuses a collision
rather than allowing one. Full case-folding differs from lowercasing in a handful
of scripts, and in the same safe direction.

**Alternative if you disagree.** Add `unicode-normalization` and
`unicode-case-mapping` to `notes-model`, which is correct and widens the
crate's dependency list that `ARCHITECTURE.md` §2 fixes at four.

---

## D-08 — `delete()` reports `Permanent` at 0.1a; no trash crate yet

**Decided.** `LocalFs::delete` removes the file and returns
`DeleteOutcome::Permanent`. The `trash` crate arrives with `entry_delete`, a
0.1b command.

**Gap closed.** The `trash` capability of `ARCHITECTURE.md` §11.

**Why.** Scope §7.7 forbids a *silent* fallback, not a permanent delete — the
requirement is that the user is told which one happened, and `DeleteOutcome`
is that. 0.1a exposes no delete command at all, so nothing can reach this path
from the UI.

**Alternative if you disagree.** Add `trash` now and set `caps.trash` from
whether it succeeds, ahead of the command that uses it.

---

## D-09 — The registry is populated when a note is **opened**, never by listing

**Decided.** `list_dir` assigns no `NoteId` and reads no file. `open_note`
hashes the note, assigns or refreshes its record, and persists the registry.

**Gap closed.** G1.

**Why.** `ARCHITECTURE.md` §4.1 gives every record a `hash`, and §6.2 says a
`NoteId` is assigned "the first time a note is seen". If *seen* meant *listed*,
opening a workspace would hash every file — and the 0.1a acceptance criterion is
that `fixtures/large` lists in under a second **without reading content**. Lazy
population satisfies both, and costs nothing at 0.1a because nothing consumes a
`NoteId` for a note that has never been opened.

**Alternative if you disagree.** Populate in a background task after the first
listing, which needs a progress state, a cancellation path and a rule for what
happens to a save that arrives mid-scan.

---

## D-10 — `workspaces.json` carries `last_workspace`

**Decided.** The global index gains `last_workspace: Option<WorkspaceId>`, and
`restore_last_workspace()` uses it. A root that no longer exists returns
`Unavailable` rather than `None`.

**Gap closed.** G3.

**Why.** "Persistir o último workspace" is 0.1a scope, and `ARCHITECTURE.md` §4
gave the index only a list with `last_opened` per entry. Picking the maximum
timestamp is a tie-break invented at read time, and it is wrong the moment two
workspaces are opened in the same second. Distinguishing "there has never been a
workspace" from "your notes are not where they were" is the other half: they are
the two answers a user most needs told apart.

**Alternative if you disagree.** Sort by `last_opened` and accept the tie-break.

---

## D-11 — `write_draft` is a command; the core cannot snapshot a buffer it does not hold

**Decided.** `WorkspaceService::write_draft(note_id, text, buffer_version,
base_rev, reason)` persists a buffer **without touching the note**.

**Gap closed.** G2, G16, G17.

**Why.** `ARCHITECTURE.md` §4.2 requires a draft after 30 s of dirty buffer and
on exit, and §5 says that while autosave is suspended "edits keep going to the
draft, every debounce". The frontend owns the buffer (§5, §13), so the core has
no text to write on its own — every one of those three rules was unimplementable
without a command, and none was in §7.1.

**Alternative if you disagree.** Have the frontend keep its own recovery copy in
`localStorage`, which puts the only copy of the user's words in the webview's
storage rather than in the operational directory that has retention rules.

---

## D-12 — The temporary file has a deterministic name

**Decided.** `.{name}.tmp`, not `.{name}.tmp-{random}`.

**Gap closed.** A defect `tools/crash-save-loop.sh` found on its first run.

**Why.** No process cleans up after `SIGKILL`, so a kill between the write and
the rename leaves the temporary file behind — that is inherent, not a bug. With a
random suffix, **every crash leaves a new one and they accumulate in the user's
folder forever**. With one name per note, a crash leaves at most one and the next
save of that note overwrites it. The crash loop asserts that bound directly.

Two of our processes writing the same note are serialised by `write.lock`, so the
shared name cannot collide, and a third-party editor does not use our naming.
`IGNORE_DEFAULT` hides `.*.tmp` from the tree in any case, so the user does not
see one even before it is overwritten.

**Alternative if you disagree.** Keep random names and sweep stale temporaries on
workspace open — which would make opening a folder modify it, against scope §2.3
and the acceptance criterion that tests it.

---

## D-13 — `close_workspace` is told which buffers are dirty

**Decided.** `close_workspace(dirty: &[NoteId])` returns
`DirtyBuffers { note_ids, count }` when the slice is non-empty.

**Gap closed.** G16, the `workspace_close` row of `ARCHITECTURE.md` §7.1.

**Why.** The frontend owns buffers, so the core cannot know which are dirty. The
error names them so the UI can offer to flush rather than only refuse.

**Alternative if you disagree.** Have the core treat "has a draft" as "is dirty",
which would refuse to close a workspace whose only draft is a *resolved* conflict
nobody has cleared.

---

## D-14 — One migration rule, applied to every state file

**Decided.** `state::load` handles `schema` for all of `registry.json`,
`session.json`, `settings.json` and `workspaces.json`: a lower schema is backed
up to `<file>.json.bak-<old>` before rewriting, and a **higher** one opens the
workspace read-only and is never overwritten.

**Gap closed.** G7, G8.

**Why.** `ARCHITECTURE.md` §4.1 stated the rule once, inside the registry
section, naming `registry.json.bak-<old-schema>` — which reads as registry-only,
leaving drafts, sessions and settings with a `schema` field and no rule. Reading
the schema before the body matters too: a file from the future must be detected
even when its shape no longer deserialises into ours.

**Alternative if you disagree.** Migrate per file with its own rule, which is
four rules to keep in step and three of them will drift.

---

## D-15 — `DocStatus` is a type in `notes-model`, not a string assembled in the UI

**Decided.** The seven states of scope §9 are an enum, exported to TypeScript
with everything else. The frontend computes the value; the vocabulary is shared.

**Gap closed.** G4.

**Why.** Scope §9 says `Saved` may only appear after the backend confirms — a
guarantee of the core, not a convention for whoever writes the component. Naming
the states in the model is what lets a test assert on them.

**Alternative if you disagree.** Leave it to the frontend and accept that the
status vocabulary lives in a component.

---

## D-16 — `mtime_ns` crosses the IPC as a string

**Decided.** `BaseRev.mtime_ns` and `Stat.mtime_ns` serialise as decimal
strings; deserialising still accepts a number, so state written before this rule
loads.

**Gap closed.** A defect the ts-rs wiring exposed, not a listed gap.

**Why.** A nanosecond timestamp is around 1.7 × 10¹⁸ and
`Number.MAX_SAFE_INTEGER` is 9.0 × 10¹⁵. Sent as a JSON number it is **rounded
by JavaScript**, and it does not merely display wrong: `BaseRev` travels back to
the core on every save, so a rounded `mtime_ns` would make the cheap check
disagree with the disk on every write and send each one down the hashing path.
Silent, slow, and invisible in any test that stays inside Rust. A test now
asserts the exact round-trip for a value above the safe integer.

`u64` fields that are counters or file sizes stay numbers — neither approaches
2⁵³.

**Alternative if you disagree.** Send milliseconds, which fits in a double and
throws away the resolution that makes the cheap check worth having on a
filesystem with nanosecond timestamps.

---

## D-17 — Generated TypeScript is committed, and CI fails when it drifts

**Decided.** `TS_RS_EXPORT_DIR` in `.cargo/config.toml` points `#[ts(export)]`
at `apps/notes-app/src/ipc/generated`, which is committed. A CI job deletes the
directory, runs `cargo test`, and fails on any diff.

**Gap closed.** G15.

**Why.** `ARCHITECTURE.md` §7 says the frontend never hand-writes an IPC type
but did not say what runs the generator or what happens when someone forgets.
Committing the output keeps the frontend building without a Rust toolchain;
regenerating in CI is what stops the two sides drifting apart about the wire
while both compile.

**Alternative if you disagree.** Generate at build time and gitignore the
output, which makes `npm run build` depend on a Rust toolchain.

---

## D-18 — CI checks the two invariants that a compiler cannot

**Decided.** A grep rejects any `fs:*` permission appearing in a capability
file, and a script fails when `en.json` and `pt-BR.json` do not carry the same
keys.

**Gap closed.** `ARCHITECTURE.md` §12 asks for the first in as many words; the
second follows from §13.

**Why.** Both failures compile, run, and look right. An `fs:` permission added
to a capability file silently undoes the reason every path check exists; a key
missing from one catalogue is a blank label for whoever reads in that language.

**Alternative if you disagree.** Trust review for both.

---

## D-19 — The crash loop does not run on every pull request

**Decided.** `tools/crash-save-loop.sh` runs on pushes to `master` and on a
pull request labelled `run-crash-loop`.

**Why.** A thousand spawn-and-kill rounds take minutes. A check that slows every
pull request is a check people learn to skip, and this is the one that already
found a real defect.

**Alternative if you disagree.** Run it on every PR with `ROUNDS=100`, trading
detection probability for latency.
