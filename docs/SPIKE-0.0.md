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

> **Corrected on 08/09/2026.** This section previously read `nvidia false` and
> concluded that criterion 1 could not be exercised here. **That was written
> from expectation rather than from a run**, and it was wrong: the machine has a
> GeForce GTX 1060 with `nvidia`, `nvidia_drm`, `nvidia_modeset` and `nvidia_uvm`
> loaded, and `/proc/driver/nvidia/version` present. The detection reported
> `true` all along; what was false was this document.

Run on 08/09/2026 with `WEBKIT_DISABLE_DMABUF_RENDERER` unset, after ADR-033:

```
[notes] dmabuf: applied — proprietary nvidia driver detected
[notes] window main: focused=true
```

No GBM error, and the window renders. With the **pre-ADR-033** rule, the same
machine produced the failure recorded in the checklist below.

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

> **Note from 08/09/2026.** The owner's shell already exports
> `WEBKIT_DISABLE_DMABUF_RENDERER`, and `linux.rs` correctly refuses to override
> a value the user set — it logs *"left alone — already set"*. A run in that
> environment proves nothing about criterion 1. **Unset the variable before
> testing**, or the first box below is unanswerable.

### Debian 13 · X11 · NVIDIA proprietary — **verified, and it failed**

This is the machine the project is developed on, and until 08/09/2026 it was
being reported as if it had no NVIDIA driver. It is a checklist item now, with
both boxes answered.

- [x] **The failure reproduces with the pre-ADR-033 rule** — the workaround
      declines to apply on X11, and the next four lines are the reason it exists:

      [notes] dmabuf: not needed — session is not wayland
      src/nv_gbm.c:288: GBM-DRV error (nv_gbm_create_device_native): …failed (ret=-1)
      KMS: DRM_IOCTL_MODE_CREATE_DUMB failed: Permission denied
      Failed to create GBM buffer of size 1100x720: Permission denied
      [notes] window main: close requested
      [notes] window main: destroyed

      Status `0`, because Tauri ends its loop when the last window is gone. This
      is the "window disappears on Open Folder" of `DECISIONS-0.1b.md` D-20, and
      the chooser was only the first thing to ask for a surface.

- [x] **It is fixed by ADR-033** — with the driver alone deciding, the same
      machine logs `applied — proprietary nvidia driver detected`, produces no
      GBM error, and renders.

- [ ] **With `settings.linux.webkit_dmabuf_workaround = "off"`**, the failure
      returns. Unwalked: it would confirm that the workaround, and not something
      else that changed, is what fixed it.

### Arch Linux · Wayland · NVIDIA

- [ ] The diagnostics panel reads `nvidia true` and **`dmabuf APPLIED`** — the
      panel prints the word, so this is read, not inferred. The session is shown
      too and is **no longer part of the condition** (ADR-033); it is reported,
      not required.
- [ ] The window renders correctly: no black window, no flicker, on launch and
      after resize.
- [ ] With `settings.linux.webkit_dmabuf_workaround = "off"` the panel says the
      workaround was skipped — and whatever the window then does is the
      observation that says whether it is still needed on current WebKitGTK.
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
