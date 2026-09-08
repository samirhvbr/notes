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

---

## D-04 — The preview corpus is golden files, blessed and then **read**

**Decided.** `fixtures/markdown/` holds `NAME.md` with `NAME.html` and
`NAME.doc.json` beside it, compared byte for byte with no normalisation.
`NOTES_BLESS=1 cargo test -p notes-markdown` rewrites them from the renderer.

**Gap closed.** "Fixtures do preview antes do parser: markdown de entrada com a
saída HTML esperada ao lado." A prose description of expected output cannot fail
a build.

**Why blessing exists, and why it is not the same as accepting.** Hand-authoring
the exact bytes `pulldown-cmark` and `ammonia` produce — attribute order, where
the newlines fall inside a table — is guesswork, and a first commit of thirty
hand-written goldens is thirty diffs against incidental formatting rather than
against the contract. So the inputs and the **contract** are written first
(`fixtures/markdown/README.md`, one row per file), the goldens are generated, and
then every one is read against that table before the commit. A golden blessed
without being read records a bug as if it were a decision, which is the only
failure mode a corpus like this has.

**That reading was not ceremonial.** It caught four defects on the first pass,
each of which the test suite would have frozen: `outra.md#uma-secao` lost its
fragment; `<alguem@example.com>` was classified as a *relative path* and rendered
as a note link to a file with an `@` in its name; a refused image dropped its alt
text and left an empty paragraph; and a bare `https://…` in prose was not
linkified at all, which scope §8.1 lists among the GFM features.

**Alternative if you disagree.** Hand-author the goldens and forbid `NOTES_BLESS`
— the comparison is unchanged, only how the expected file is first written. Or
normalise whitespace before comparing, which makes the corpus tolerant of a
`pulldown-cmark` upgrade and blind to a real change in output.

---

## D-05 — `RelPath` accepts `""` on the wire, and `parse` still refuses it

**Decided.** `TryFrom<String> for RelPath` maps the empty string to
`RelPath::root()`. `RelPath::parse` is unchanged and still returns
`PathError::Empty`.

**Gap closed.** A defect from 0.1a, found while wiring this crate: `RelPath::root()`
**serialises** to `""` and `try_from` **refused** `""`, so the type could not
deserialise a value it produces. The frontend's `ROOT` is that string, and
`tree_list` takes a `RelPath` — every listing of the workspace root was rejected
by argument deserialisation before the command body ran. It is the sidebar's
first call on every launch.

**Why here rather than in `parse`.** `parse` reads something a user typed, and an
empty name is not a path; relaxing it would let `note_create(dir, "")` through to
a collision check. The wire form is a different question with a different answer,
and `RelPath` already has two constructors for exactly that reason.

**Alternative if you disagree.** Give the root its own wire value — `"."`, or a
`Option<RelPath>` on every command that can take it. Both cost a change to every
signature that names a directory, and `"."` is a segment `parse` refuses on
purpose.

---

## D-06 — `mailto:` is refused, because the scope and the capability file both say so

**Decided.** A `mailto:` link, and an email autolink `<alguem@example.com>`,
render as **text**. Only `http` and `https` become anchors.

**Gap closed.** Scope §8.4: *"Links externos `http(s)` abrem no navegador do SO
por clique. Outros esquemas recusados."*

**Why not make an exception.** `shell:allow-open` in
`src-tauri/capabilities/default.json` is restricted to `{ "url": "https://**" }`
and `{ "url": "http://**" }`. Rendering a `mailto:` anchor would therefore
produce a link that does nothing when clicked — worse than text, because it
looks like it works. Making it work means widening a capability, and **granting
the agent a permission is the owner's act** (CLAUDE.md golden rule 7): written
into the capability file with its reason, never applied on the way past.

**The email autolink is the same decision wearing a different hat.**
`pulldown-cmark` reports `<alguem@example.com>` with the scheme stripped, so
without this rule it was classified as a relative path and rendered as a link to
a note called `alguem@example.com`. The golden corpus caught it.

**Alternative if you disagree.** Add `{ "url": "mailto:*" }` to `shell:allow-open`
and change one match arm in `notes-markdown/src/url.rs`. The capability edit is
yours to make; the code half is two lines.

---

## D-07 — Exactly one attribute is force-set on `<input>`, because `ammonia` reorders the ones it forces

**Decided.** `ammonia` forces `type="checkbox"` on every `<input>`; `disabled`
and `checked` are allowlisted rather than forced.

**Gap closed.** A flapping golden. Forcing both `type` and `disabled` made
`fixtures/markdown/tasklists.html` differ between runs of the same code:
`ammonia` lifts a forced attribute out of its position and re-appends it while
iterating a `HashMap`, whose order is per-process. With one forced attribute the
output is fixed; with two it is a coin toss, and a byte-exact corpus would have
been abandoned as "flaky" rather than read as the real finding it is.

**What it costs.** An `<input>` written by a note in a workspace that has opted
into raw HTML comes out as an **enabled** checkbox rather than a disabled one.
That is the whole difference, and it carries nothing: `name`, `value` and every
`on*` attribute are dropped, `<form>` and `<button>` are refused outright, so
there is nothing to send and nowhere to send it. The task-list markers the
application itself renders always carry `disabled` — the writer emits it and the
golden pins it.

**Why it was found here and not in review.** The suite ran green three times
before the corpus was committed; the failure only appeared when `cargo test`
happened to run the golden target in a process whose hash seed ordered the map
the other way. A test that is byte-exact turns "occasionally different" into a
red build, which is the argument for byte-exact comparison rather than against
it.

**Alternative if you disagree.** Force both and normalise attribute order before
comparing — the corpus then tolerates any future reordering, including one that
matters. Or drop `<input>` from the allowlist entirely and render task markers
as a `<span>` with a glyph, which loses the semantics a screen reader uses.

---

## D-08 — The preview serves raster images only; SVG waits for a decision

**Decided.** `notes-asset://` serves `png`, `jpg`/`jpeg`, `gif`, `webp`, `bmp`
and `avif`, and refuses everything else with `Unsupported`. There is a 64 MiB
ceiling on a single asset.

**Gap closed.** Scope §8.4 says *"Imagens: só relativas ao workspace, formatos
suportados"* and never names the formats. This is the list.

**Why SVG is not on it.** An `<img>` cannot execute the script inside an SVG —
the specification forbids scripting in that context — so admitting it would
probably be safe. *Probably safe because of a browser rule* is a different thing
from *safe by decision*, and the format is a document rather than a picture. A
user who wants a diagram in a note can use a raster export today; a user who
loses their notes to a WebView quirk cannot undo it.

**Alternative if you disagree.** Add `("svg", "image/svg+xml")` to `IMAGE_TYPES`
in `notes-core/src/preview.rs`. The asset response already carries
`default-src 'none'; sandbox` and `nosniff`, which is the second layer that
would make it defensible; the ADR should say so.

---

## D-09 — `shell().open` stays deprecated rather than migrating a capability on the way past

**Decided.** `shell_open` keeps `tauri_plugin_shell`'s deprecated `open`, under
an `#[allow(deprecated)]` that names the reason. Its `http(s)`-only check is in
the Rust as well as in the capability file.

**Gap closed.** Clippy's `-D warnings` gate, which the deprecation would
otherwise fail.

**Why not migrate.** `tauri-plugin-opener` is the successor, and adopting it
means a new dependency **and** replacing `shell:allow-open` with
`opener:allow-open-url` in `capabilities/default.json`. Scope §19 sends a change
that touches permissions or a central dependency to the owner rather than
letting an agent make it in passing, and CLAUDE.md's golden rule 7 says the same
of granting permissions. The scope of what may be opened is identical either
way, so nothing is lost by waiting.

**Alternative if you disagree** — the whole change, for whoever makes it:
add `tauri-plugin-opener` to `src-tauri/Cargo.toml`, `.plugin(tauri_plugin_opener::init())`
in `lib.rs`, swap the `shell:allow-open` block in `capabilities/default.json` for
`opener:allow-open-url` with the same two `url` entries, and replace the body of
`shell_open` with `app.opener().open_url(&url, None::<&str>)`. The scheme check
above it stays either way: the capability is what the WebView may ask for, and
that check is what the process will do.
