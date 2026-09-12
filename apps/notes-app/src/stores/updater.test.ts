import { beforeEach, describe, expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("./workspace", () => ({ useWorkspace: { getState: vi.fn(() => ({ info: null })) } }));
vi.mock("../ipc/barrier", () => ({ beginSyncBarrier: vi.fn(() => true), endSyncBarrier: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { useWorkspace } from "./workspace";
import { beginSyncBarrier, endSyncBarrier } from "../ipc/barrier";
import { useUpdater } from "./updater";
const available = { supported: true, version: "9.0.0", notes: "New release" };
beforeEach(() => {
  vi.clearAllMocks();
  const storage = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
  });
  useUpdater.setState({ phase: "idle", version: null, notes: null });
  vi.mocked(beginSyncBarrier).mockReturnValue(true);
  vi.mocked(useWorkspace.getState).mockReturnValue({ info: null } as never);
});
describe("desktop updates", () => {
  it("checks automatically without installing", async () => {
    vi.mocked(invoke).mockResolvedValue(available);
    await useUpdater.getState().check();
    expect(useUpdater.getState().phase).toBe("available");
    expect(invoke).toHaveBeenCalledExactlyOnceWith("update_check");
  });
  it("coalesces checks while a request is pending", async () => {
    let finish!: (value: typeof available) => void;
    vi.mocked(invoke).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
    const check = useUpdater.getState().check();
    await useUpdater.getState().check(true);
    expect(invoke).toHaveBeenCalledOnce();
    finish(available); await check;
  });
  it("keeps automatic errors silent but answers a manual check", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("offline"));
    await useUpdater.getState().check(); expect(useUpdater.getState().phase).toBe("idle");
    await useUpdater.getState().check(true); expect(useUpdater.getState().phase).toBe("error");
  });
  it("remembers dismissal and lets manual checks show the version again", async () => {
    vi.mocked(invoke).mockResolvedValue(available);
    await useUpdater.getState().check(); useUpdater.getState().dismiss();
    await useUpdater.getState().check(); expect(useUpdater.getState().phase).toBe("idle");
    await useUpdater.getState().check(true); expect(useUpdater.getState().phase).toBe("available");
  });
  it("never installs with an open workspace", async () => {
    useUpdater.setState({ phase: "available", version: "9.0.0" });
    vi.mocked(useWorkspace.getState).mockReturnValue({ info: { id: "workspace" } } as never);
    await useUpdater.getState().install();
    expect(invoke).not.toHaveBeenCalled();
    expect(useUpdater.getState().phase).toBe("closeWorkspace");
  });
  it("does not install across pending edits or another exclusive operation", async () => {
    useUpdater.setState({ phase: "available", version: "9.0.0" });
    vi.mocked(beginSyncBarrier).mockReturnValue(false);
    await useUpdater.getState().install(); expect(invoke).not.toHaveBeenCalled();
  });
  it("releases the input barrier after installation fails", async () => {
    useUpdater.setState({ phase: "available", version: "9.0.0" });
    vi.mocked(invoke).mockRejectedValue(new Error("signature mismatch"));
    await useUpdater.getState().install();
    expect(invoke).toHaveBeenCalledWith("update_install");
    expect(endSyncBarrier).toHaveBeenCalledOnce();
    expect(useUpdater.getState().phase).toBe("error");
  });
});
