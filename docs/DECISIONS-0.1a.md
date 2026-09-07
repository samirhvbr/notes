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
