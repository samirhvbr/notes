# Documentation — notes

> **Status:** `ACTIVE`

Index of the project's **stable** documentation. Work in progress lives in
[`../.continue/`](../.continue/) and migrates here when it matures.

This index is curated, not exhaustive. Keep it that way: a table of contents
that lists everything stops being read.

## Pages

| Document | What it answers |
|---|---|
| [ACCEPTANCE-0.1d.md](ACCEPTANCE-0.1d.md) | **Milestone 0.1d's acceptance — the interface** — ten areas nobody has ticked, the narrow automated half that is honest about being narrow, and the twenty-five flows of 0.1b and 0.1c re-indexed with their steps rewritten for the window that now exists. |
| [DECISIONS-0.1d.md](DECISIONS-0.1d.md) | **Calls made while building 0.1d** — including the three questions the milestone left open by name, and the two contrast rules that were wrong the first time. |
| [ACCEPTANCE-0.1c.md](ACCEPTANCE-0.1c.md) | **Milestone 0.1c's criteria, written from its first commit** — the two acceptance criteria and, from the same moment, the thirteen interface flows that no core test can reach. |
| [ACCEPTANCE-0.1b.md](ACCEPTANCE-0.1b.md) | **Every 0.1b acceptance criterion against a named test or a documented manual step** — the watcher, identity across an external rename, the sanitizer census over `fixtures/xss/`, the trash — plus what the preview IR actually costs, measured, and the list of what nobody has looked at because the window has never been launched. |
| [DECISIONS-0.1c.md](DECISIONS-0.1c.md) | **Calls made while building 0.1c** — including the one that refused to pull `notes-index` forward, and why an ACTIVE ADR made that a stop rather than a preference. |
| [DECISIONS-0.1b.md](DECISIONS-0.1b.md) | **Calls made while building 0.1b** — including the two that were deliberately *not* made on the way past, because they touch a permission or a dependency and belong to the owner. |
| [ACCEPTANCE-0.1a.md](ACCEPTANCE-0.1a.md) | **Every 0.1a acceptance criterion against a named test or a documented manual step**, with the measurements. All eight are met; the full-disk half of §5 became automated during 0.1b. |
| [DECISIONS-0.1a.md](DECISIONS-0.1a.md) | **Calls made while building 0.1a** that the specification did not make — what was decided, which gap it closed, and the alternative if the owner disagrees. |
| [SPIKE-0.0.md](SPIKE-0.0.md) | **What milestone 0.0 has established, and what it has not** — the checks that pass on this machine, what the twelve automated tests actually cover, and the checklist of what can only be seen on Arch/Wayland/NVIDIA, an iPhone and an Android device. |
| [SCOPE.md](SCOPE.md) | **The product specification** — the durable source for requirements implemented and planned; the queue contains only the work still to do. |
| [ARCHITECTURE.md](ARCHITECTURE.md) | **The architecture this application is built against.** Repository layout, crates, core types, the app-data layout and its schemas, the command contract, `CoreError`, the inter-process lock, the markdown IR, `Caps` and distribution. `ACTIVE`; §§7–10 describe code that exists since 0.1b, and sections for later milestones stay `PROPOSED`. |
| [product.md](product.md) `PROPOSED` | **What notes is** — the accessible product narrative for the original plan. Consult [SCOPE.md](SCOPE.md) for the durable specification and current implementation boundary. |
| [architecture-v1.md](architecture-v1.md) `SUPERSEDED` | **How it is put together** — the layering rule everything is checked against, the stack, the repository layout at both levels, the filesystem abstraction and why it exists before there is a second platform, the index and `.notes/`, the local security posture, the sync model that is designed for but not built, and the server, REST and MCP surfaces. |
| [history/](history/) | **Superseded planning drafts** — preserved for provenance, never implementation authority. |
| [roadmap.md](roadmap.md) | **The order it gets built in** — the seven product milestones from a desktop editor to an MCP server, their delivered portions and the work that remains queued. |
| [versioning.md](versioning.md) | How a version is set and a commit is written. `version.md` is the sole authority and the version is the **first semver in it**; the `X`/`Y`/`Z` criteria **for this project**; `X.Y.Z - description in English` — and the host's convention instead, in a repository we do not own; **[tags and Releases](versioning.md#tags-and-releases)**; what the two git hooks check. |
| [decisions.md](decisions.md) | **ADRs** — the chronological record of what was decided here and why, so it is not re-litigated. |
| [security.md](security.md) | The normative security document. In a conflict with any other document, it wins. |
| [updater.md](updater.md) | Signed desktop update behavior, publisher setup, retry rules and installed acceptance. |
| [runbook.md](runbook.md) | From a clean machine to a running environment; **the release** — what ships, what does not and why, how to build the packages locally, and what to do when it fails halfway; the pre-flight checklist before making the repository public. |
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

- [Acceptance — milestone 0.2](ACCEPTANCE-0.2.md): index, search semantics, reference review, recovery and pending installed-release flows.

- [Local knowledge and agents](KNOWLEDGE-0.3.md): metadata, wiki resolution, graph, clipboard import and standalone MCP configuration/recovery.
- [Acceptance — milestone 0.3](ACCEPTANCE-0.3.md): automated evidence and pending installed-release owner flows.

- [Mobile acceptance](ACCEPTANCE-0.4.md): merged compilation foundation and remaining usable-mobile scope.

## Self-hosted server

- [SERVER-0.5.md](SERVER-0.5.md) — operator CLI, REST, HTTPS and backup/restore.
- [ACCEPTANCE-0.5.md](ACCEPTANCE-0.5.md) — automated coverage and pending owner walk.

## Synchronization in progress

- [SYNC-0.6.md](SYNC-0.6.md) — implemented causal model and pairing preview; remaining sync work stays queued.
