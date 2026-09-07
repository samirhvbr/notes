import { create } from "zustand";
import * as ipc from "../ipc";
import type { CoreError, Entry, RelPath, WorkspaceInfo } from "../ipc";

interface WorkspaceState {
  info: WorkspaceInfo | null;
  /** Directory listings, keyed by path. The tree is lazy: a directory is
   *  listed when it is first expanded, never up front. */
  listings: Record<string, Entry[]>;
  expanded: Set<string>;
  error: CoreError | null;

  restore: () => Promise<void>;
  adopt: (info: WorkspaceInfo) => Promise<void>;
  list: (dir: RelPath) => Promise<void>;
  toggle: (dir: RelPath) => Promise<void>;
  refresh: (dir: RelPath) => Promise<void>;
  fail: (e: unknown) => void;
}

export const useWorkspace = create<WorkspaceState>((set, get) => ({
  info: null,
  listings: {},
  expanded: new Set(),
  error: null,

  fail: (e) => set({ error: ipc.asCoreError(e) }),

  async restore() {
    try {
      const info = await ipc.workspaceRestoreLast();
      if (info) await get().adopt(info);
    } catch (e) {
      // A workspace that moved is reported, not silently forgotten: "no
      // workspace" and "your notes are not where they were" are different.
      get().fail(e);
    }
  },

  async adopt(info) {
    set({ info, listings: {}, expanded: new Set(), error: null });
    await get().list(ipc.ROOT);
  },

  async list(dir) {
    try {
      const entries = await ipc.treeList(dir);
      set((s) => ({ listings: { ...s.listings, [dir]: entries } }));
    } catch (e) {
      get().fail(e);
    }
  },

  async toggle(dir) {
    const expanded = new Set(get().expanded);
    if (expanded.has(dir)) {
      expanded.delete(dir);
      set({ expanded });
      return;
    }
    expanded.add(dir);
    set({ expanded });
    if (!get().listings[dir]) await get().list(dir);
  },

  async refresh(dir) {
    await get().list(dir);
  },
}));
