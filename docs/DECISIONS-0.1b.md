# Decisions taken while building 0.1b

> **Status:** `ACTIVE` · Every call made under the standing rule *"choose the
> simplest option compatible with the scope, write the missing section into
> `ARCHITECTURE.md`, and carry on"*. One row per decision: what was decided,
> which gap it closed, and **what the alternative is** if the owner disagrees.
>
> This is not the ADR log. A decision here that turns out to be structural gets
> promoted to [`decisions.md`](decisions.md); the rest are recorded so that none
> of them is invisible. The 0.1a log is [`DECISIONS-0.1a.md`](DECISIONS-0.1a.md)
> and is not re-litigated here.

---

## D-01 — The Windows cross-check runs by default, and installs its own target

**Decided.** `tools/check.sh` no longer skips the `x86_64-pc-windows-gnu` clippy
step when the target is not installed: it runs `rustup target add` once and then
runs the step. `NOTES_NO_WINDOWS_CHECK=1` opts out on purpose; a machine with no
`rustup` at all degrades to a printed warning rather than failing the gate.

**Gap closed.** The step existed at `0.7.3` and was conditional, which meant the
one check that would have caught both Windows compile failures was absent on
exactly the machines that had never run it — a developer who has not added the
target is precisely the developer who needs the check. A gate that opts itself
out silently is not a gate.

**Why the warning rather than a failure** when `rustup` is missing: the step
guards a cross-compilation concern, and refusing to run `cargo test` and the
byte-preservation suite over it would trade a real check for a hypothetical one.
The line is loud and names the reason.

**Alternative if you disagree.** Make the missing-target case fatal (one
`return 1` becomes `exit 1`), or move the cross-check out of `check.sh` and rely
on the Windows CI leg alone — which is what cost two rounds at `0.7.2` and
`0.7.3`.

---

## D-02 — The `.continue/` pointer to `ARCHITECTURE.md` was repaired in place

**Decided.** `.continue/README.md` linked `ARCHITECTURE.md` at the repository
root, where it has never lived. The link now points at `../docs/ARCHITECTURE.md`
and the words "at the repository root" are gone.

**Gap closed.** A queue index whose links do not resolve fails at the one job the
index has.

**Why this is not a breach of the queue rule.** The `QUEUE-RULE` block forbids
editing queue *material* as tidying — the specifications of things that do not
exist yet. `README.md` says of itself that it is "the one file here that is not
queue material… the folder's index", and this change edits a pointer rather than
a specification. It was authorised explicitly for this pass.

**Alternative if you disagree.** Move `docs/ARCHITECTURE.md` to the repository
root, which is what the link asserted. It was rejected: `docs/repodocs.md` maps
where documentation lives, and one document promoted to the root for the sake of
one stale link would break that map instead.

---

## D-03 — The full disk is a size-capped `tmpfs` in a user namespace, not a loopback mount

**Decided.** `tools/enospc.sh` creates an unprivileged user namespace
(`unshare -U -r -m`), mounts a 1 MiB `tmpfs` inside it, and runs
`notes-core`'s `tests/enospc.rs` against that directory via `NOTES_TINY_DIR`. It
runs in `tools/check.sh` and on the Linux leg of CI. Acceptance criterion 5 of
0.1a moves from *partly met* to **met**.

**Gap closed.** [`ACCEPTANCE-0.1a.md`](ACCEPTANCE-0.1a.md) §5 — the classifier
was unit-tested for errno 28, but nothing exercised the path from a filesystem
that is really out of room to a `SaveResult::WriteFailed` and a draft holding the
buffer. Two steps sat in that gap: `write_atomic` returning `Err` at the right
moment, and `settle` writing the draft rather than propagating.

**Why not the loopback filesystem** the manual recipe described. `mount -o loop`
needs `CAP_SYS_ADMIN` in the initial namespace — root, in practice `sudo` — so
automating it meant deciding what CI runners are allowed to do, and a developer
running the gate would be prompted for a password by a lint script. The
acceptance document called this "a decision about CI privileges rather than about
this milestone", and it was right; the resolution was that **no privilege is
needed**. Any user may create a user namespace, `tmpfs` is mountable inside one,
and a `tmpfs` over its size limit returns ENOSPC exactly as a full disk does —
the kernel does not have a second, more authentic ENOSPC. The mount is private to
the namespace, so a failed run cannot leave a filesystem mounted on anybody's
machine, which the loopback recipe could.

**What it does not cover.** EDQUOT: a quota cannot be arranged this way, so
`IoKind::classify`'s unit test stays the only evidence that a quota reads as
`DiskFull`. And the *visible* half — the status bar saying "no space left" — is
still asserted in the core rather than observed on screen, along with everything
else the window owes.

**Alternative if you disagree.** Keep the loopback recipe as the automated form
and give the CI job `sudo`; the test binary is unchanged, only the harness that
provides `NOTES_TINY_DIR` differs, and that recipe is still printed in
`ACCEPTANCE-0.1a.md` §5 for kernels that forbid unprivileged user namespaces.
