// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ReceivedSyncShell } from "./ReceivedSync";
import * as ipc from "../ipc";
import { beginSyncBarrier, endSyncBarrier, isSyncLocked, setComposing, tracked } from "../ipc/barrier";
import { useEditor } from "../stores/editor";
import { useWorkspace } from "../stores/workspace";
import { useSync } from "../stores/sync";
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => "/receive-state") }));
vi.mock("../ipc", async original => ({ ...await original<typeof import("../ipc")>(), syncOpen: vi.fn(), syncApply: vi.fn(), syncReload: vi.fn() }));
vi.mock("./DeviceSync", () => ({ DeviceSync: () => null }));
const originals = { ws: useWorkspace.getState(), sync: useSync.getState() };
const rev = { hash: "b3:abc", size: 3, mtime_ns: "1" } as ipc.BaseRev;
const fresh = { note_id: "note", path: "test.md", text: "new", base_rev: rev, read_only: null, draft: null, profile: {} } as ipc.OpenedNote;
function doc() {
  useEditor.setState({ doc: { noteId: fresh.note_id, path: fresh.path, text: "old", baseRev: rev, readOnly: null, bufferVersion: 0, savedVersion: 0, status: "saved", externalRev: 0, conflict: null, draft: null, lastError: null } });
}
async function connected() {
  useWorkspace.setState({ info: null, adopt: async info => { useWorkspace.setState({ info }); }, refresh: vi.fn(async () => {}) });
  useSync.setState({ start: vi.fn(async () => {}), stop: vi.fn() });
  vi.mocked(ipc.syncOpen).mockResolvedValue({ id: "workspace", root: "/target" } as ipc.WorkspaceInfo);
  const navigate = vi.fn();
  render(<ReceivedSyncShell><button onClick={navigate}>Navigate</button></ReceivedSyncShell>);
  fireEvent.click(screen.getByRole("button", { name: "Open received workspace" }));
  await screen.findByRole("button", { name: "Apply received revisions" });
  doc();
  return navigate;
}
afterEach(() => {
  cleanup(); endSyncBarrier(); setComposing(false);
  useWorkspace.setState(originals.ws); useSync.setState(originals.sync);
  useEditor.setState({ doc: null }); vi.clearAllMocks();
});
it("holds input and IPC until a verified reload, including partial failure", async () => {
  const navigate = await connected();
  let resolve!: (r: ipc.SyncApplyResult) => void;
  vi.mocked(ipc.syncApply).mockImplementation(() => new Promise(r => { resolve = r; }));
  fireEvent.click(screen.getByRole("button", { name: "Apply received revisions" }));
  expect(isSyncLocked()).toBe(true);
  useEditor.getState().edit("typing during apply");
  fireEvent.click(screen.getByRole("button", { name: "Navigate", hidden: true }));
  expect(navigate).not.toHaveBeenCalled();
  expect(useEditor.getState().doc?.text).toBe("old");
  const unrelated = vi.fn(async () => 1);
  await expect(tracked(unrelated)).rejects.toMatchObject({ code: "unsupported" });
  expect(unrelated).not.toHaveBeenCalled();
  await act(async () => { resolve({ applied: null, error: { code: "unsupported", cap: "rename" }, refreshed: [fresh], reload_failed: false }); });
  expect(isSyncLocked()).toBe(false);
  expect(useEditor.getState().doc?.text).toBe("new");
  expect(useEditor.getState().doc?.externalRev).toBe(1);
});
it("keeps the old document frozen after an unknown outcome until recovery succeeds", async () => {
  await connected();
  vi.mocked(ipc.syncApply).mockRejectedValue(new Error("lost response"));
  vi.mocked(ipc.syncReload)
    .mockResolvedValueOnce({ applied: null, error: { code: "unsupported", cap: "reload unavailable" }, refreshed: [], reload_failed: true })
    .mockResolvedValueOnce({ applied: null, error: null, refreshed: [], reload_failed: false })
    .mockResolvedValue({ applied: null, error: null, refreshed: [fresh], reload_failed: false });
  fireEvent.click(screen.getByRole("button", { name: "Apply received revisions" }));
  const retry = await screen.findByRole("button", { name: "Retry safe reload" });
  expect(isSyncLocked()).toBe(true);
  expect(useEditor.getState().doc?.text).toBe("old");
  for (let attempts = 1; attempts <= 2; attempts++) {
    await act(async () => { fireEvent.click(retry); });
    expect(ipc.syncReload).toHaveBeenCalledTimes(attempts);
    expect(isSyncLocked()).toBe(true);
    expect(useEditor.getState().doc?.text).toBe("old");
  }
  fireEvent.click(retry);
  await waitFor(() => expect(isSyncLocked()).toBe(false));
  expect(useEditor.getState().doc?.text).toBe("new");
});
it("refuses dirty buffers without sending, saving or replacing them", async () => {
  await connected();
  useEditor.setState(s => ({ doc: s.doc && { ...s.doc, text: "my changes", bufferVersion: 1 } }));
  fireEvent.click(screen.getByRole("button", { name: "Apply received revisions" }));
  expect(ipc.syncApply).not.toHaveBeenCalled(); expect(isSyncLocked()).toBe(false);
  expect(useEditor.getState().doc?.text).toBe("my changes");
});
it("refuses admission while RPC continuations or IME composition are active", async () => {
  let resolve!: () => void;
  const pending = tracked(() => new Promise<void>(r => { resolve = r; }));
  expect(beginSyncBarrier()).toBe(false);
  resolve(); await pending;
  expect(beginSyncBarrier()).toBe(false);
  await new Promise(r => setTimeout(r, 5));
  setComposing(true); expect(beginSyncBarrier()).toBe(false);
  setComposing(false); expect(beginSyncBarrier()).toBe(true);
});
