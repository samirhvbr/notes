# Product — what notes is

> **Status:** `PROPOSED` · **Nothing described here has been built yet.** This
> document is the worked-out form of a specification that still lives in the
> queue, [`../.continue/`](../.continue/README.md), and **the queue is the
> authority on intent while both exist** — intent changes there, and this page is
> updated when the thing is built ([ADR-009](decisions.md#adr-009--an-item-leaves-continue-only-when-it-has-been-built)).
> A section becomes `ACTIVE` when its code exists and works.
>
> The product definition. What the application is, what it
> does, and what it deliberately does not do. The order in which it gets built is
> in [roadmap.md](roadmap.md); how it is built is in
> [architecture.md](architecture-v1.md); why each irreversible choice was made is in
> [decisions.md](decisions.md).

## 1. What it is

**notes** is a local-first Markdown note-taking application for Linux, macOS,
Windows, iOS and Android. The user picks a folder; that folder is the workspace;
the `.md` files inside it are the notes.

The founding constraint, from which most of this document follows:

> **The files belong to the user, not to the application.**

There is no proprietary storage format. A note is a Markdown file on a
filesystem, and it stays fully usable from outside the app — a terminal, VS
Code, `git`, `rsync`, a backup tool, another Markdown editor, an AI agent
writing to disk. The app is one more program that happens to be good at editing
those files; it is never the only one that can read them. This is
[ADR-001](decisions.md#adr-001--markdown-files-on-the-filesystem-are-the-source-of-truth).

## 2. Local-first, and what that rules out

Basic operation must never require:

- an account;
- a login;
- an internet connection;
- a server;
- a proprietary cloud service.

A user who installs the app, points it at a folder and starts writing has the
whole product. Everything on top of that — remote storage, multi-device sync, an
HTTP API, AI agents — is opt-in, arrives later, and is **self-hostable by the
user** rather than a service we run. There is no official cloud.

## 3. The workspace

On first launch the user is offered exactly two paths:

```text
Welcome

[ Open Folder ]        an existing folder that already holds .md files
[ Create Workspace ]   a new folder, created where the user chooses
```

The chosen folder is the workspace, and **the directory tree on disk is the
navigation tree in the app**. There is no separate notion of a "notebook" or a
"collection" that the user has to maintain in parallel with their folders.

```text
notes/
├── work/
│   ├── projects.md
│   └── meetings.md
├── personal/
│   ├── ideas.md
│   └── travel.md
├── servers.md
└── tasks.md
```

The selected workspace persists across restarts.

## 4. File format

The format is `.md`, and it stays readable outside the app. Supported from the
start: CommonMark and GitHub Flavored Markdown — headings, lists, ordered lists,
task checkboxes, links, images, tables, fenced code blocks, blockquotes, bold,
italic, horizontal rules, and links between files in the workspace.

**YAML front matter is supported and optional:**

```markdown
---
title: Servers
tags:
  - infrastructure
  - linux
created: 2026-09-07
---

# Servers
```

**The app never writes metadata into a note that the user did not ask for.** A
file the user created in `vim` and never touched in the app must come back out
of the app byte-identical unless they edited it. Front matter is read when it is
there; it is not injected because the app would find it convenient.

## 5. Interface

Inspired by Obsidian, VS Code and modern Markdown editors — **inspired, not
copied.** The palette, spacing and density are rebuilt; no upstream theme files
are vendored.

**One theme: dark.** There is no theme system in the first version, and that is
a deliberate scope reduction rather than an omission — a single well-made dark
theme is a fraction of the work of a theming layer, and the theming layer buys
nothing until the editor is good.

### Desktop layout

```text
┌─────────────────────────────────────────────────────────────┐
│ Toolbar                                                     │
├───────────────┬─────────────────────────────────────────────┤
│               │ tabs                                        │
│ FILES         ├─────────────────────────────────────────────┤
│               │                                             │
│ > work        │                                             │
│   projects.md │              EDITOR                         │
│   meetings.md │                                             │
│               │                                             │
│ > personal    │                                             │
│   ideas.md    │                                             │
│               │                                             │
├───────────────┴─────────────────────────────────────────────┤
│ status                                                      │
└─────────────────────────────────────────────────────────────┘
```

The sidebar shows directories, Markdown files, search and recent files
(favourites later), and supports create note, create folder, rename, move,
delete and duplicate. Drag-and-drop for moving files and folders is designed for
from the start, even where it lands later.

### Mobile layout

Mobile uses the same concepts and **does not reproduce the desktop layout
literally**. The sidebar becomes a drawer:

```text
┌───────────────────┐
│ Notes          ☰  │
├───────────────────┤
│                   │
│     Editor        │
│                   │
└───────────────────┘
```

Mobile priorities, in order: open, edit, create, search, navigate folders, and —
later — sync. An adapted interface, not a shrunk one.

## 6. Editor

The editor is the centre of the product. **CodeMirror 6.** It must have syntax
highlighting for Markdown, optional line numbers, undo/redo, multiple selection,
keyboard shortcuts, find and replace within the file, autosave, detection of
external modification, and it must stay responsive on large files.

### View modes

| Mode | What it shows |
|---|---|
| **Source** | Raw Markdown |
| **Preview** | Rendered Markdown |
| **Split** | Both, side by side |

**Live Preview** — the Obsidian-style mode where syntax renders inline as you
type — is explicitly **not** in the first MVP. It is the single most expensive
thing in the editor, and Source/Preview/Split covers the same need while the
rest of the app is being built.

## 7. Autosave and write safety

Changes are saved automatically. There is no Save button the product depends on
(`Ctrl+S` exists, and it forces a flush rather than being the only path to
disk).

```text
change → debounce → atomic write → filesystem
```

**A write interrupted midway must never corrupt the note.** Writing is
write-to-temp-then-rename, never truncate-then-write, because the failure being
designed against — power loss, a kill, a full disk — reliably produces a
zero-length note under the naive approach.

## 8. External modification

Living correctly alongside other programs is a **requirement**, not a nicety —
it is the direct consequence of §1.

```text
notes app
    ↓
file.md
    ↑
VS Code / git / a script / an AI agent
```

When another program changes a file, the app detects it and reflects it. The
user never has to close and reopen the workspace to see the truth on disk.
Filesystem watching is used wherever the platform supports it.

## 9. Search

Two distinct things, and they stay distinct:

**In-file search** (`Ctrl+F`) — find and replace inside the open note.

**Workspace search** — across every file: name, path and content.

```text
firewall

infra/firewall.md
Line 24
Firewall configuration...

clients/project-x.md
Line 73
The client's firewall...
```

**Quick Open** (`Ctrl+P`) is a third thing and is deliberately separate from
both: it matches file names and paths only, and it must stay instant.

```text
Ctrl+P  > serv

server.md
servers/dns.md
servers/email.md
```

## 10. Links and backlinks

Two link formats, in this order:

```markdown
[Server](infra/server.md)     standard Markdown — supported first
[[Server]]                    wiki links — later
```

Standard Markdown links come first because they are what makes a note portable:
they keep working in GitHub, in VS Code and in any other Markdown renderer.
Wiki links are an app convenience layered on top.

Once links are indexed, a note can show what points at it:

```text
server.md

Referenced by:
- infrastructure.md
- project-datacenter.md
- checklist.md
```

Backlinks do not block the first MVP.

## 11. Tags

Later. Both forms are recognised and the indexer relates them to each other:

```markdown
#linux #server #project
```

```yaml
tags:
  - linux
  - infrastructure
```

## 12. Tabs, command palette, shortcuts

Desktop opens multiple files in tabs, and the open set is restored on restart.

The **command palette** (`Ctrl+Shift+P` / `Cmd+Shift+P`) exists from early on
for a structural reason: it is where new functionality goes without adding
another button to the interface.

```text
Ctrl+N          New Note
Ctrl+P          Quick Open
Ctrl+Shift+F    Search Workspace
Ctrl+F          Search File
Ctrl+W          Close Tab
Ctrl+S          Force Save
Ctrl+Shift+P    Command Palette
```

macOS gets `Cmd` equivalents throughout. Desktop is keyboard-first.

## 13. Settings

Few, on purpose:

```text
Editor    font size · line numbers · word wrap · tab size
Files     autosave · show hidden files
Markdown  default preview mode
```

No plugin system in the MVP.

## 14. Performance target

The app stays usable at 10, 100, 1 000 and 10 000+ files. Startup must not
require reading every file's contents; indexing is incremental. The number that
matters is time-to-first-usable-window on a 10 000-file workspace, not time to a
complete index.

## 15. Not in the first version

Listed because an unstated exclusion is read as an oversight:

collaborative editing · a full WYSIWYG editor · canvas · plugins ·
multiple themes · web publishing · an embedded AI chat · native Git integration ·
sync · version history · user accounts · an official cloud server.

Each may be reconsidered later. None of them is a reason to compromise §1.

## 16. Git

**The app does not depend on Git** — see
[ADR-006](decisions.md#adr-006--git-is-not-a-dependency-and-not-a-feature-in-the-first-versions).
Because notes are ordinary Markdown files in an ordinary folder, a workspace
*can* be a Git repository:

```text
notes/
├── .git/
├── work/
└── personal/
```

The app must not get in the way of that — most concretely, it must tolerate
`.git/` in the tree and never touch it. Native Git integration is a candidate
for later, not a requirement now.

## Local knowledge in milestone 0.3

Graph view is implemented under ADR-038/041, using the same resolved links as
backlinks. YAML properties, tags, wiki links, clipboard images and standalone
local MCP are described in [KNOWLEDGE-0.3.md](KNOWLEDGE-0.3.md); the source
files remain authoritative and permissions are explicit.
