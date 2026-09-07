# Documentation — notes

> **Status:** `ACTIVE`

Index of the project's **stable** documentation. Work in progress lives in
[`../.continue/`](../.continue/) and migrates here when it matures.

This index is curated, not exhaustive. Keep it that way: a table of contents
that lists everything stops being read.

## Pages

| Document | What it answers |
|---|---|
| [DECISIONS-0.1a.md](DECISIONS-0.1a.md) | **Calls made while building 0.1a** that the specification did not make — what was decided, which gap it closed, and the alternative if the owner disagrees. |
| [SPIKE-0.0.md](SPIKE-0.0.md) | **What milestone 0.0 has established, and what it has not** — the checks that pass on this machine, what the twelve automated tests actually cover, and the checklist of what can only be seen on Arch/Wayland/NVIDIA, an iPhone and an Android device. |
| [ARCHITECTURE.md](ARCHITECTURE.md) `PROPOSED` | **The architecture 0.1a is built against.** Repository layout, crates, core types, the app-data layout and its schemas, the command contract, `CoreError`, the inter-process lock, the markdown IR, `Caps` and distribution — every decision `.continue/SCOPE_final.md` §20 delegates. Becomes `ACTIVE` in the commit that ships 0.1a. |
| [product.md](product.md) `PROPOSED` | **What notes is** — the local-first constraint the whole product hangs off, the workspace model, the file format and the promise that the app never rewrites a note it was not asked to, the interface, the editor and its view modes, autosave and write safety, search, links, and the explicit list of what the first version does not do. |
| [architecture-v1.md](architecture-v1.md) `SUPERSEDED` | **How it is put together** — the layering rule everything is checked against, the stack, the repository layout at both levels, the filesystem abstraction and why it exists before there is a second platform, the index and `.notes/`, the local security posture, the sync model that is designed for but not built, and the server, REST and MCP surfaces. |
| [roadmap.md](roadmap.md) `PROPOSED` | **The order it gets built in** — the seven product milestones from a desktop editor to an MCP server, what each must do before the next starts, and why each one is useful on its own. |
| [versioning.md](versioning.md) | How a version is set and a commit is written. `version.md` is the sole authority and the version is the **first semver in it**; the `X`/`Y`/`Z` criteria **for this project**; `X.Y.Z - description in English` — and the host's convention instead, in a repository we do not own; **[tags and Releases](versioning.md#tags-and-releases)**; what the two git hooks check. |
| [decisions.md](decisions.md) | **ADRs** — the chronological record of what was decided here and why, so it is not re-litigated. |
| [security.md](security.md) | The normative security document. In a conflict with any other document, it wins. |
| [runbook.md](runbook.md) | From a clean machine to a running environment; deploy; the pre-flight checklist before making the repository public. |
| [repodocs.md](repodocs.md) | **What in this repository came from the fleet standard, and where each piece lives.** The map of the relationship: what travels out of repodocs by copy, what by stamp, and what is only ever linked; the manifest of files and what is lost when one is missing; how to bring an existing repository in; how a fleet rule reaches this one. |

<!-- Add the project's own pages as they are written. The recurring ones across
     this fleet are architecture.md, glossary.md and a playbook. -->

## The norm

The documentation convention this repository follows — queue vs. record vs.
history, filenames that carry state, the status vocabulary, the language rule —
lives once, in the fleet standard:
[samirhvbr/repodocs `docs/conventions.md`](https://github.com/samirhvbr/repodocs/blob/master/docs/conventions.md).
It is deliberately **not** copied here.

[repodocs.md](repodocs.md) is the map of that relationship from this side: which
files here came from the standard, which of them are mirrors that get
regenerated, and which questions are answered upstream rather than here.

## Where a new document goes

| It describes… | It goes to |
|---|---|
| something that has been **built** — a measurement, a contract, a runbook | `docs/`, marked `ACTIVE` |
| a decision that has been taken | `docs/decisions.md`, as an ADR, `ACTIVE` — a decision exists the moment it is taken, code or no code |
| something planned but **not built yet** | `.continue/`, in Portuguese. A worked-out copy may also live here as `PROPOSED`, and the queue is the authority while both exist |
| something that happened, with its date and its why | `CHANGELOG.md` |

**When an item leaves `.continue/` is the `QUEUE-RULE` block in
[`../CLAUDE.md`](../CLAUDE.md)** — regenerated from the fleet standard, and the
source. It is not restated here. It began as a local rule in this repository
([ADR-009](decisions.md#adr-009--an-item-leaves-continue-only-when-it-has-been-built))
and the fleet adopted it the same day.
