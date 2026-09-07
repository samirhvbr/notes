# notes-app — milestone 0.0 spike

> **Status:** `ACTIVE` for what it is, and what it is is a **spike**. SCOPE §17
> of the milestone: *nada vira produto*. It builds, it is tested, and **it is not
> the application** — milestone 0.1a replaces almost all of it.

## What this is

The shell that milestone 0.0 exists to run on real hardware. Its product is
evidence: what has been established and what has not is recorded in
[`../../docs/SPIKE-0.0.md`](../../docs/SPIKE-0.0.md), including the checklist that
can only be marked on Arch/Wayland/NVIDIA, an iPhone and an Android device.

## Running it

```bash
npm install
npm run tauri dev      # dev, with the Vite server on 1420
cargo test             # the core tests — no Tauri, no window
```

## What it deliberately keeps from the specification

Doing otherwise would make the spike measure the wrong thing:

- **No filesystem capability is granted to the webview.** The capability set is
  `core:default` and `dialog:allow-open`, nothing more; every read and write goes
  through a command in `src-tauri` (SCOPE §2.5).
- **Every path is re-resolved and re-checked against the workspace root at the
  moment of use**, not only when the folder is opened (§7.6). `..`, absolute
  paths and symlinks escaping the root are refused by one test on the resolved
  path rather than by three string rules that each miss a case.
- **Opening a folder writes nothing into it** (§2.3). The chosen path is
  persisted in app data.
- **Writes go through a temp file and a rename** (§7.4).
- **The dmabuf decision is a pure function**, so the Wayland + NVIDIA case this
  machine cannot produce is covered by a test rather than by hope.

## What it is NOT, and must not be mistaken for

Not milestone 0.1a, and not a draft of it:

- no `BaseRev`, so a write can overwrite a concurrent external change — the
  central thing 0.1a adds (§12);
- no identity registry, no `NoteId`;
- no recoverable draft;
- no watcher and no external-change detection;
- no byte policy — BOM, CRLF and mixed EOL are not detected or preserved (§7.5);
- errors cross to the frontend as plain strings, not the coded `CoreError` the
  UI can translate;
- listing is one level, root only, `.md` and `.markdown`.

The identifier is `br.com.samirhv.notes.spike`, suffixed on purpose: the
production identifier is still open, it fixes the app-data path on three
operating systems, and a spike must neither squat on it nor pollute its
directory.

## Where the real thing is specified

[`../../.continue/SCOPE_final.md`](../../.continue/SCOPE_final.md) and
[`../../docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md).
