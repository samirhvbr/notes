# Spike 0.0 — findings

> **Status:** `ACTIVE` · The record of what milestone 0.0 has actually
> established, and what it has not. **0.0 is not closed.** Its queue item stays
> in [`../.continue/`](../.continue/README.md) until the checklist in §2 is
> marked, because the questions the spike exists to answer are about hardware
> this was not written on.

The spike's product is evidence, not software. SCOPE §17: *nada vira produto*.

The application lives at [`../apps/notes-app/`](../apps/notes-app/README.md).

---

## 1. Verified here — Debian 13 (trixie), X11, no NVIDIA

Machine: `Linux 6.12.107+deb13-amd64`, `XDG_SESSION_TYPE=x11`,
WebKitGTK `2.52.6`, libsoup `3.6.5`, Rust `1.96.0`, Node `24.15.0`,
Tauri `2.11.5`.

| Check | Result |
|---|---|
| `cargo build` on the workspace | passes, 0 warnings |
| `cargo clippy --all-targets` | 0 warnings |
| `cargo test` — **runs with no Tauri app and no window** | **12 passed, 0 failed** |
| `npm install` | 127 packages, 0 vulnerabilities |
| `npm run build` — `tsc --noEmit` then Vite | passes |

### What the 12 tests actually cover

Six on the dmabuf decision, six on path and byte handling:

- the workaround **is** applied on Wayland + NVIDIA;
- it is **not** applied on Wayland without NVIDIA, on X11 even with NVIDIA, or
  off Linux;
- `NOTES_NO_DMABUF_WORKAROUND=1` beats detection;
- a value the user already set is never overridden;
- `..`, `sub/../../`, and an absolute path are all refused against the workspace
  root, and a path inside it resolves;
- the atomic write round-trips bytes exactly for empty, plain, CRLF, BOM-led and
  accented/emoji payloads, and leaves no temp file behind.

The decision logic was split into a pure function precisely so the Wayland +
NVIDIA case — the one this machine cannot produce — is covered by a test rather
than by hope. **That proves the decision, not the rendering.**

### What this machine reports at startup

```
session   x11
nvidia    false
dmabuf    not applied — session is not wayland
```

Which is correct, and is also why criterion 1 cannot be closed here.

### Not verified here, and not claimed

- **The window has not been launched.** No GUI was started on this machine, so
  "renders correctly" is unverified even for Debian/X11. To do it:
  `cd apps/notes-app && npm run tauri dev`.
- Wayland, NVIDIA, macOS, Windows, iOS, Android: none of them exist here.

### Decisions taken inside the spike, and their scope

- **Bundle identifier is `br.com.samirhv.notes.spike`**, deliberately suffixed.
  The production identifier is still open, and it fixes the app-data path on
  three operating systems — changing it later strands the state of everyone who
  installed. A spike must not squat on it, and must not pollute its directory.
- Errors cross to the frontend as plain strings. Milestone 0.1a replaces this
  with a coded `CoreError` the UI can translate; a spike inventing its own error
  contract would only have to unpick it.
- The webview is granted `core:default` and `dialog:allow-open`. **No filesystem
  capability** — every read and write is a command that re-resolves and re-checks
  the path against the root at the moment of use (SCOPE §2.5, §7.6).

---

## 2. Pending on other hardware

Nothing below can be done from this machine. **Mark a box only after seeing it,
not after reasoning that it should work.**

### Arch Linux · Wayland · NVIDIA

- [ ] The diagnostics panel reads `session wayland`, `nvidia true`, and
      **`dmabuf APPLIED`** — the panel prints the word, so this is read, not
      inferred.
- [ ] The window renders correctly: no black window, no flicker, on launch and
      after resize.
- [ ] With `NOTES_NO_DMABUF_WORKAROUND=1` the panel says the workaround was
      skipped — and whatever the window then does is the observation that says
      whether the workaround is still needed on current WebKitGTK.
- [ ] Built against the **current** `webkit2gtk-4.1` on rolling, not Debian's.

### iPhone (iOS 17+)

- [ ] Type `ação` and a dead-key `ç` in the editor: no duplication, no loss.
- [ ] Type an emoji.
- [ ] IME composition, selection, paste and undo all behave with the virtual
      keyboard up.
- [ ] Pick a folder, write, **kill the app**, reopen: the folder is still
      accessible and the diagnostics panel says `restored from disk`.
- [ ] Record what `tauri-plugin-fs` already covers on iOS, and what a
      security-scoped bookmark needs beyond it.

### Android 11+

- [ ] Same four input checks as iOS.
- [ ] Pick a folder via SAF, write, kill, reopen: still accessible.
- [ ] Record whether `takePersistableUriPermission` survives, and what happens
      when the document is moved afterwards.

---

## 3. Closing 0.0

0.0 closes when every box above is marked and this document records what was
found — including the failures, which are the part with value. Only then does
its queue item leave `.continue/`.
