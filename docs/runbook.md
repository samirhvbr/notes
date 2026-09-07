# Runbook — notes

> **Status:** `ACTIVE` · From a clean machine to a running environment, and the
> checklists that gate a release.

<!-- Sections 1–4 are yours to fill in. Sections 5–7 are inherited from the
     fleet standard and are already correct — keep them. -->

## 1. Requirements

_Fill in: runtimes and versions, system packages, accounts and access needed._

## 2. From a clean machine to running

```bash
git clone git@github.com:samirhvbr/notes.git
cd notes
git config core.hooksPath tools/git-hooks   # once per clone — see §5

# install, configure, run — fill this in
cp .env.example .env
```

_Say which of the paths this is: running it with Docker, or developing with the
whole chain. If there are two, say what each one requires and what to do when it
does not come up._

## 3. Configuration

_Fill in: what each variable in `.env.example` means, which ones are required,
and which ones are secrets that never get committed
([security.md §5](security.md#5-secrets-and-configuration))._

## 4. Deploy

_Fill in: how a change reaches production, who may do it, what it restarts, and
what to do when it fails halfway. Be specific about the traps — a
`config:cache` newer than the `.env` means the `.env` does not apply, and a
worker that loaded config at startup does not re-read it._

## 5. The git hooks

Once per clone — yours, and every collaborator's:

```bash
git config core.hooksPath tools/git-hooks
```

Confirm both directions before you rely on them:

```bash
git commit --allow-empty -m "feat: teste"                      # must be REJECTED
git commit --allow-empty -m "0.1.0 - primeiro commit do repo"  # must be accepted
```

Rename `HOOK_ESCAPE_VAR` at the top of each hook to `NOTES_NO_HOOK`.
Rules: [versioning.md](versioning.md).

## 6. Pre-flight before making a repository public

A private repository accumulates content that assumed privacy. Before flipping
visibility:

- [ ] `git log -p | grep -iE 'password|secret|token|api[_-]?key'` — scan the
      **history**, not just the working tree. A secret removed from HEAD is
      still in every clone.
- [ ] Any secret ever committed has been **rotated**, not merely deleted.
- [ ] No internal hostname, private IP range or infrastructure path that should
      not be public.
- [ ] `LICENSE` is present and `NOTICE` agrees with it about the license and the
      copyright holder.
- [ ] `SECURITY.md` names a reporting address that is actually monitored.
- [ ] Every prescriptive document declares its status; nothing reads as current
      while describing something abandoned.
- [ ] `.continue/` holds no finished item and no half-page detail.
- [ ] `README.md` is in English and describes what the project **is**, not what
      it was going to be.
- [ ] No placeholder left over from the skeleton:
      `grep -rn '<[A-Z_]\+>' . --exclude-dir=.git`

## 7. Verifying the repository still conforms

```bash
# the twins are identical below the H1
diff <(tail -n +2 CLAUDE.md) <(tail -n +2 AGENTS.md)

# version.md is a bare X.Y.Z
grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' version.md && echo ok

# settings.json parses
python3 -m json.tool .claude/settings.json > /dev/null && echo ok

# every version in history has a tag and a Release (prints what is missing)
./tools/release.sh --backfill --dry-run
```

If the `diff` on the twins reports anything other than the mirroring comment,
an edit was applied to one file and not the other.
