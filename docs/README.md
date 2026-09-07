# Documentation — franknote

> **Status:** `ACTIVE`

Index of the project's **stable** documentation. Work in progress lives in
[`../.continue/`](../.continue/) and migrates here when it matures.

This index is curated, not exhaustive. Keep it that way: a table of contents
that lists everything stops being read.

## Pages

| Document | What it answers |
|---|---|
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
| something that already exists — a measurement, a contract, a runbook, an ADR | `docs/` |
| something still to be done, in one line | `.continue/` + a pointer |
| something that happened, with its date and its why | `CHANGELOG.md` |

If a queue item needs half a page, it is in the wrong place: write it here and
leave one line and a pointer in the queue.
