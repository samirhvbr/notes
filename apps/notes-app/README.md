# notes-app — milestone 0.0 spike

> **Status:** `INCOMPLETE — PARKED` · Committed deliberately unfinished, and not
> to be built on until milestone 0.0 is resumed by decision. Nothing here is
> product code: SCOPE §17 says of 0.0, *"nada vira produto"*.

## What this is

The shell that milestone 0.0 exists to run on real hardware. Its only job is to
make the four 0.0 acceptance criteria observable — which is why the diagnostics
panel is as prominent as the editor.

## What is here

| Path | State |
|---|---|
| `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html` | written |
| `src/main.tsx`, `src/App.tsx`, `src/Editor.tsx` (CodeMirror 6), `src/api.ts` | written |
| `src/styles.css` | **missing** |
| `src-tauri/` — `Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/`, `src/` | **missing entirely** |

## What is NOT here, and must not be assumed

- **The Rust side does not exist.** `src/api.ts` declares `spike_env`,
  `open_workspace`, `restore_workspace`, `read_note` and `write_note` as Tauri
  commands. **None of them is implemented.** The frontend cannot run.
- `npm install` has never been run here; there is no lockfile.
- `cargo build` has never been run against this app.
- The Wayland + NVIDIA `WEBKIT_DISABLE_DMABUF_RENDERER` detection — 0.0
  acceptance criterion 1 — is **designed and not written**: it belongs in the
  missing `src-tauri/src/lib.rs`, before the webview is created.

## What it is not, even when finished

Not milestone 0.1a. No `BaseRev`, no concurrency guard, no conflict detection,
no watcher, no draft recovery, no atomic-write guarantees beyond a plain
temp-and-rename. Those are 0.1a and are specified in `.continue/SCOPE_final.md`
§7 and §12.

## Resuming

0.0 cannot be closed from a Debian/X11 machine: its acceptance needs
Arch/Wayland/NVIDIA, an iPhone and an Android device. Findings go to
`docs/SPIKE-0.0.md`, which does not exist yet either.
