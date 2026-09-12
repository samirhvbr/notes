# `.continue/` — the queue

> **Status:** `ACTIVE` · Last reviewed 12/09/2026, repository at `1.0.5`.

This folder contains only work that has not been completed. Implemented
contracts, measurements and delivery history live in [`../docs/`](../docs/) and
[`../CHANGELOG.md`](../CHANGELOG.md).

The queue rules live in the `QUEUE-RULE` block in
[`../CLAUDE.md`](../CLAUDE.md). This README is its English index; queue items
are in Portuguese.

## Work remaining

| Item | What remains | Who unblocks it |
|---|---|---|
| **Release gate repairs** | Resolve the existing Clippy needless borrow in `notes-sync-client/src/control.rs:896` and the macOS watcher startup/index-under-change test failures in `notes-core/tests/deep.rs`; rerun the full gate (see [build verification](../docs/runbook.md#linux-build-verification-103)) | Implementation |
| [0.6 — sync](0.6-sync.md) | Installed-build and physical-device owner acceptance | Samir |
| **0.7 — remote MCP** | Expose the existing MCP capabilities through server authentication and workspace/subfolder scopes after sync is ready | Implementation |
| **0.4 — mobile** | Turn the merged foundation into usable iOS/Android applications: entry points, generated projects, UI, folder adapters, lifecycle handling and physical-device validation | Mobile agent / Samir |
| [0.1d — interface acceptance](0.1d-interface.md) | Installed-release owner walk and repeat on the following release | Samir |
| [0.2 — index acceptance](0.2-indice.md) | Installed-release owner walk and repeat on the following release | Samir |
| **0.3 — knowledge and local agents acceptance** | Installed-release owner walk and repeat on the following release; see [ACCEPTANCE-0.3.md](../docs/ACCEPTANCE-0.3.md) | Samir |
| **0.5 — self-hosting acceptance** | Installed-release owner walk and repeat on the following release; see [ACCEPTANCE-0.5.md](../docs/ACCEPTANCE-0.5.md) | Samir |
| **0.0 — platform spike** | Physical checks on Arch/Wayland/NVIDIA, iPhone and Android; see [SPIKE-0.0.md](../docs/SPIKE-0.0.md) | Samir |

## Current implementation order

1. Finish sync 0.6 according to [0.6-sync.md](0.6-sync.md).
2. Build remote MCP 0.7.
3. Continue the mobile 0.4 track.
4. Perform owner acceptance and platform checks when the required installed
   builds and devices are available.

## Where the delivered record lives

| Subject | Record |
|---|---|
| Product specification | [SCOPE.md](../docs/SCOPE.md) |
| Implemented architecture | [ARCHITECTURE.md](../docs/ARCHITECTURE.md) |
| Milestone delivery order | [roadmap.md](../docs/roadmap.md) |
| Sync contract and implementation history | [SYNC-0.6.md](../docs/SYNC-0.6.md) |
| Older planning drafts | [docs/history/](../docs/history/) |
| Version-by-version history | [CHANGELOG.md](../CHANGELOG.md) |

No manual acceptance is inferred from CI, unit tests or command-line checks.
