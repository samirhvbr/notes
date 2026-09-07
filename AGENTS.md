# notes — Instructions for coding agents

<!--
  The content below the H1 is duplicated between CLAUDE.md (read by Claude
  Code) and AGENTS.md (read by other tooling, agents.md standard) — keep the
  two byte-identical below the H1. If you edit one, edit the other.
-->

> **Read in this order:** [.continue/README.md](.continue/README.md) (the queue —
> where we stopped, **always first**) · [docs/versioning.md](docs/versioning.md)
> (how a version and a commit are written) ·
> [docs/decisions.md](docs/decisions.md) (ADRs — do not re-litigate a decided
> direction, link the ADR) · [docs/security.md](docs/security.md) (normative;
> wins any conflict).
>
> **The fleet documentation norm is not in this repository.** It lives once, at
> [samirhvbr/repodocs `docs/conventions.md`](https://github.com/samirhvbr/repodocs/blob/master/docs/conventions.md).
> Read it there; do not copy it here.
>
> **[docs/repodocs.md](docs/repodocs.md) is the map of what came from that
> standard** — which files here arrived from the skeleton, which blocks below
> are mirrors that get regenerated rather than edited, and which questions are
> answered upstream. Read it before adding, moving or deleting a documentation
> file.

---

## 🔄 Before you start: `git pull`

**ALWAYS** check for remote updates before writing or changing anything in this
repository:

```bash
git pull
```

Working on a stale base creates conflicts. Pull first, always. To inspect
without merging: `git fetch && git status`.

**Fresh clone — enable the hooks ONCE:**

```bash
git config core.hooksPath tools/git-hooks
```

Two hooks then run, and each exists because the other cannot reach its moment:

| Hook | What it checks | Why there |
|---|---|---|
| `commit-msg` | The subject is `X.Y.Z - description in English`; no Conventional Commits prefix, no vague comment | It is the only moment the message exists and the commit does not |
| `pre-push` | `version.md` against the remote default branch — duplicate and monotonicity | It is the only hook that sees the **remote**, and `commit-msg` provably does not run during a rebase, which is how duplicate versions get born |

⚠️ **`pre-push` runs no suite, and that is a choice:** a push that waits four
minutes becomes `--no-verify` the following week, and then the control is dead.
**With no reachable remote it degrades with a warning, never a refusal.** Escape
hatch declared in both, named by `HOOK_ESCAPE_VAR` at the top of each hook —
rename it to `NOTES_NO_HOOK`.

---

## What this project is

**notes** — a desktop app for writing Markdown, standalone and optionally linked to a public Git repository

<!-- Replace the block below with what an agent actually needs: the stack, where
     the entry points are, what runs locally, and what must never be touched. -->

- **Stack:** _to be filled in._
- **Runs locally with:** _to be filled in._
- **Never touch:** _to be filled in._

---

## Golden rules

1. **`.continue/` is the queue · `docs/` is the record · `CHANGELOG.md` is the
   history.** A document moves from queue to record the moment it describes
   something that already exists. A finished item **leaves** the queue.
2. **If a queue item needs half a page, it is in the wrong place.** Write it in
   `docs/` and leave one line and a pointer.
3. **In a contradiction between the queue and a document, the document wins.**
4. **Every prescriptive document declares its status** on the first lines:
   `ACTIVE` · `HISTORICAL` · `PROPOSED` · `DEPRECATED` · `NOT ADOPTED`. One
   with no declaration is read as `ACTIVE`, which is exactly the failure mode.
5. **A document made stale by a change is fixed in the same pass.** A document
   that ages in silence is worse than a missing one, because it has the
   authority of being written down.
6. **`CLAUDE.md` and `AGENTS.md` are byte-identical below the H1.** Edit one,
   edit the other.
7. **Everything is versioned; the only exception is a secret.** `.claude/` and
   `.continue/` are tracked on purpose. A new `.gitignore` exception beyond
   secrets requires an ADR, never a silent line.
8. **Granting the agent a permission is the owner's act** — written into
   `.claude/settings.json` with its reason and how to revert it, never applied
   silently and never left as a promise in prose.
9. **A new decision becomes an ADR** in `docs/decisions.md`, in the same pass.
10. **You commit, and nothing is finished until you have.** The commit is the
    last step of the task, not a follow-up — never report work as done while it
    sits uncommitted. One subject per commit; a large delivery is split into
    blocks.

---

<!-- LANGUAGE-RULE:repodocs -->

## Language — English (US) at home, the upstream's when we are guests

> Marked echo. The single source is **[samirhvbr/repodocs](https://github.com/samirhvbr/repodocs/blob/master/docs/conventions.md#8-language)**
> — change it there, not here. This block is regenerated.

**Everything that lives in this repository, or in GitHub's interface around it,
is written in English (US)**: documents, **commit messages**, pull request titles
and bodies, issues, code comments, changelog entries, release notes.

Commit format: `X.Y.Z - short description in English`. The version comes from
`version.md` and is bumped in the same commit. Conventional Commits prefixes
(`feat:`, `fix:`, `chore:`) and vague one-word messages are forbidden.

**Two carve-outs, and only two.** The first is end-user-facing strings — UI
text, transactional email, product copy: product i18n for a Brazilian audience,
not repository content. The second is the **Blue3 internal repositories**
(`BLUE3-ISP/*`, `samirhvbr/blue3-intranet`, `samirhvbr/blue3-ai-login`), which
are Portuguese throughout — if you are reading this block inside one of them,
this is the wrong block: they carry `LANGUAGE-RULE-PT`. A repository joins that
set by a written decision, never by argument.

History is not rewritten: Portuguese messages already in the log stay as they
are.

**In a repository that is not ours, the upstream's conventions win — the
language and the commit shape both.** Opening a pull request or an issue on a
repository we do not own makes us guests, and a guest writes in the host's
language. Our `X.Y.Z - description` is meaningless there anyway: they have no
`version.md` of ours, and no version for us to bump.

**Check before you write, and the first signal that answers wins:** a written
instruction (`CONTRIBUTING.md`, a pull request or issue template, a contribution
section in the README), then the last ~20 merged pull requests, then the issues,
then the commit log. A written instruction beats observed practice — if they ask
for English and their log is Portuguese, write English. Below that line the
**clear majority** decides, and clear means clear.

**When you cannot tell, write English (US).** A private repository, an empty
history, no network, a refused `gh` call and a genuinely mixed log all land in
the same place — the house rule. Unverifiable is not a licence to guess.

**This is a scope boundary, not a second carve-out.** Nothing in *our*
repositories changes because a foreign one is Portuguese, and code identifiers
are English wherever you are.

<!-- /LANGUAGE-RULE -->

---

## Branch

**`master`, never `main`.** The default branch of every repository in this fleet
is `master` — a house convention so that every script, hook and runbook can say
`origin/master` and be right. A repo created as `main` gets renamed with GitHub's
rename (it keeps open PRs and redirects old links); every existing clone then
needs `git branch -m main master`, `git fetch origin`,
`git branch -u origin/master master` and — the step people skip —
`git remote set-head origin -a`.

A different branch is fine **only as a written decision**, recorded in
`docs/decisions.md`.

Norm: [samirhvbr/repodocs `docs/conventions.md`](https://github.com/samirhvbr/repodocs/blob/master/docs/conventions.md).

---

<!-- COMMIT-RULE:repodocs -->

## Commits — you commit, and nothing is delivered until you have

> Marked echo. The single source is **[samirhvbr/repodocs](https://github.com/samirhvbr/repodocs/blob/master/docs/versioning.md#who-commits-and-when)**
> — change it there, not here. This block is regenerated.

**Committing is your job.** Not "leave the tree ready and something downstream
packages it" — you run `git commit`, and `git push`, as the last step of the work
you were asked to do. The COMMITTER skill that used to commit on an agent's
behalf is `enabled: false` in every repository of this fleet since 03/09/2026;
what is left of it is a kill-switch, not a scheduler. **If you do not commit,
nobody does.**

**Do not report a task as finished before the commit exists.** "Done",
"delivered", "concluded" mean the work is in `git log` — never that it is sitting
uncommitted where only this session can see it. The commit is the last step *of
the task*, not a follow-up for someone else. If you are about to write
"finished", commit first, then write it.

**Push is part of the delivery, and a refused push is the one place a human enters.**
Commit *and* push, every delivery — a clean push needs nobody's permission and is never
held back for review. When the push is **refused** (conflict, non-fast-forward, protected
branch), stop there and say so: never force, never rewrite history to get past it, never
invent a merge resolution you have not verified. The gate is the refused push, not the
commit.

**Every commit obeys the versioning rules**, with no exception:

- Subject `X.Y.Z - short description in English (US)`, the version taken from
  `version.md` and **bumped in the same commit**.
- The `CHANGELOG.md` entry is written first — its `## X.Y.Z - description`
  heading *is* the subject.
- No Conventional Commits prefix (`feat:`, `fix:`, `chore:`) and no vague
  subject ("update", "ajuste", "wip", "changes", "several improvements").

**The bump is the one clause a repository may override — in writing.** If this
repository's own documentation says the version is stamped some other way, and says
why, follow that. Otherwise the line above applies to you. An override nobody wrote
down is not an exception. Nothing else in this block bends: the changelog entry, the
subject, the language, one subject per commit, and committing before you report done
all hold regardless.

**All of this governs the repositories we own.** In a repository that is not
ours, the host's commit convention governs instead — their subject line, in
their language. `X.Y.Z` is meaningless where there is no `version.md` of ours,
and there is no version there for us to bump. Our versioning rules govern our
remotes, not every remote we can push to.

**One subject per commit.** The subject has to describe the whole commit
honestly. The moment your description needs an "and" to be true, it is two
commits.

**Split a large delivery into blocks.** A complex task is committed as a series
of commits grouped by subject, each small enough to be described in one line and
read on its own. They may share a version — bump `version.md` in the first and
repeat the number in the rest; two commits carrying one version is expected, not
a mistake. **Splitting is the default** for anything non-trivial, because the
history is the documentation of *how* the work was done, and one commit touching
six unrelated subjects documents none of them.

**The standard you are keeping:** someone reading `git log` alone — a year from
now, without the conversation that produced the work — can say what happened,
when, why, and at which version. If your commit would fail that test, it is too
big or its subject is too vague, and both are fixed the same way.

<!-- /COMMIT-RULE -->

---

## Version and commits (mandatory)

Format: `version - short description in English`. The version comes from
[version.md](version.md), **bumped in the same commit**:

<!-- The X/Y/Z slots and the discipline are the fleet convention. What counts as
     a Z is per-project — replace these with this project's real triggers. -->

- **Z** — _e.g. a new screen, a new table, a layout change, a wording fix._
- **Y** — _e.g. a new main service, an API redesign, a compatibility break._
- **X** — reserved; a stable release, by hand.

Forbidden: `feat:` / `fix:` / `chore:` prefixes and vague messages ("ajuste",
"update", "wip"). Several commits may share one version — group by subject, bump
in the first, repeat the number in the rest.

**Write the `CHANGELOG.md` entry first: its `## X.Y.Z - description` heading
*is* the commit subject.** Bodies are narrative — what changed, why, and what it
measured — not bullet lists.

**The version is the FIRST semver in `version.md`** — a bare string and a
markdown document both satisfy that.

**Every version gets a tag named exactly after it — no `v` prefix — and a
published GitHub Release.** `.github/workflows/release.yml` does it on every push
that touches `version.md`, calling `tools/release.sh`; run that script by hand
for a backfill. Both skip what already exists.

**The `version.md` on GitHub equals the Releases on GitHub.** Your local checkout
does not enter the calculation. A PR publishes nothing; the moment it merges, the
Release becomes that version.

**The bump and the Release are one act.** A commit that bumps `version.md` is not
finished until that version has a Release and the `Latest` badge is on it — same
push, not "later". `./tools/release.sh` is idempotent and also repairs a drifted
badge.

Full rules: [docs/versioning.md](docs/versioning.md).

---

## Before closing a version

- [ ] What changed is in `CHANGELOG.md`, with the **why**, not just the what.
- [ ] A new or changed document is in `docs/` — not in the queue, not in the
      commit body.
- [ ] A finished item **left** `.continue/`.
- [ ] A document made stale by this change was corrected in the same pass.
- [ ] `version.md` bumped, in this commit.
- [ ] **The work is committed** — and split into one commit per subject if it
      covered more than one. Nothing is reported as finished while it is
      uncommitted.
- [ ] The **tag and the GitHub Release** exist for this version. Normally the
      workflow does it on push; `./tools/release.sh` if you need it now.
- [ ] The twins still match: `diff <(tail -n +2 CLAUDE.md) <(tail -n +2 AGENTS.md)`.
