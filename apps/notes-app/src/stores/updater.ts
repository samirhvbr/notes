import { create } from "zustand";
import { checkUpdate, installUpdate } from "../ipc/updater";
import { beginSyncBarrier, endSyncBarrier } from "../ipc/barrier";
import { useWorkspace } from "./workspace";

type Phase = "idle" | "checking" | "available" | "current" | "unsupported" | "error" | "closeWorkspace" | "installing";
interface State {
  phase: Phase;
  version: string | null;
  notes: string | null;
  check: (manual?: boolean) => Promise<void>;
  dismiss: () => void;
  install: () => Promise<void>;
}
const dismissedKey = "tura-dismissed-update";
function dismissed(): string | null { try { return localStorage.getItem(dismissedKey); } catch { return null; } }
export const useUpdater = create<State>((set, get) => ({
  phase: "idle", version: null, notes: null,
  check: async (manual = false) => {
    if (["checking", "installing"].includes(get().phase)) return;
    const previous = get();
    set({ phase: "checking" });
    try {
      const result = await checkUpdate();
      if (!result.supported) { set({ phase: manual ? "unsupported" : "idle", version: null, notes: null }); return; }
      const show = result.version && (manual || result.version !== dismissed());
      set({ version: result.version, notes: result.notes, phase: show ? "available" : manual ? "current" : "idle" });
    } catch {
      set({ phase: manual ? "error" : previous.phase });
    }
  },
  dismiss: () => {
    try { if (get().version) localStorage.setItem(dismissedKey, get().version!); } catch { /* session-only dismissal */ }
    set({ phase: "idle" });
  },
  install: async () => {
    if (!["available", "closeWorkspace", "error"].includes(get().phase) || !get().version) return;
    if (useWorkspace.getState().info) { set({ phase: "closeWorkspace" }); return; }
    if (!beginSyncBarrier()) { set({ phase: "error" }); return; }
    set({ phase: "installing" });
    try { await installUpdate(); }
    catch { set({ phase: "error" }); }
    finally { endSyncBarrier(); if (get().phase === "installing") set({ phase: "idle" }); }
  },
}));
