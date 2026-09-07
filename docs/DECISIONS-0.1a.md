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
