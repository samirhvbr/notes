import { create } from "zustand";
import { askConfirm, askText } from "../app/dialog";
import { t as tr } from "../i18n";
import * as ipc from "../ipc";
import type { CoreError, Entry, NoteId, RelPath, WorkspaceInfo } from "../ipc";
import { useEditor } from "./editor";
import { useTabs } from "./tabs";

/** Directories always come first; this orders what follows within each kind. */
export type SortMode = "name" | "name-desc";

interface WorkspaceState {
  info: WorkspaceInfo | null;
  /** Directory listings, keyed by path. The tree is lazy: a directory is
   *  listed when it is first expanded, never up front. */
  listings: Record<string, Entry[]>;
  expanded: Set<string>;
  error: CoreError | null;
  /**
   * How the explorer orders a directory.
   *
   * The **core's** order is directories first, then names case-insensitively,
   * and it is stable so the tree does not reshuffle between listings
   * (`LocalFs::list`). That stays the default and the sort here reorders what
   * came back — it never asks the core for a different order, because the order
   * a directory listing arrives in is a filesystem question and the order a
   * person wants to read it in is not.
   */
  sort: SortMode;

  restore: () => Promise<void>;
  adopt: (info: WorkspaceInfo) => Promise<void>;
  /**
   * Put the current workspace down. `true` when it is down.
   *
   * `false` means the user was asked about unsaved work and said no — which is
   * an answer, not a failure, and the caller must not carry on as though the
   * workspace had closed.
   */
  leave: () => Promise<boolean>;
  /** Leave, then open `root`. In that order, so the guard is on the path. */
  switchTo: (root: string) => Promise<void>;
  /** Ask for a name, leave, then create a workspace under `parent`. */
  createIn: (parent: string) => Promise<void>;
  list: (dir: RelPath) => Promise<void>;
  toggle: (dir: RelPath) => Promise<void>;
  refresh: (dir: RelPath) => Promise<void>;
  setSort: (s: SortMode) => void;
  /** Fold every expanded directory. The listings are kept — they are cheap and
   *  still correct; only the expansion state changes. */
  collapseAll: () => void;
  fail: (e: unknown) => void;
  /** A short, non-error message — "moved to the trash", "duplicated as …".
   *  Scope §7.7 requires the application to *say* which of two things it did,
   *  and an error banner is the wrong shape for something that went right. */
  notice: string | null;
  note: (message: string) => void;
  clearNote: () => void;
}

export const useWorkspace = create<WorkspaceState>((set, get) => ({
  info: null,
  listings: {},
  expanded: new Set(),
  error: null,
  sort: "name",

  notice: null,

  setSort: (sort) => set({ sort }),
  collapseAll: () => set({ expanded: new Set() }),

  fail: (e) => set({ error: ipc.asCoreError(e) }),
  note: (notice) => set({ notice }),
  clearNote: () => set({ notice: null }),

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

  async leave() {
    // The editor holds one document (ADR-030), so the dirty set is at most one
    // note — but it is sent as a list because that is the shape the core's
    // refusal names, and 0.2's split panes will make it more than one.
    const dirty = dirtyNotes();
    try {
      await ipc.workspaceClose(dirty);
    } catch (e) {
      const err = ipc.asCoreError(e);
      if (err.code !== "dirty_buffers") {
        get().fail(e);
        return false;
      }
      // Named, so the question can be about *this* note rather than about
      // "unsaved changes" in the abstract. Never `window.confirm`: it does
      // nothing in a WebView and takes the flow behind it with it.
      const doc = useEditor.getState().doc;
      const ok = await askConfirm({
        title: tr("workspace.dirtyTitle"),
        body: tr("workspace.dirtyBody", { name: doc?.path ?? "" }),
        confirmLabel: tr("workspace.dirtySave"),
      });
      if (!ok) return false;
      await useEditor.getState().save(true);
      try {
        await ipc.workspaceClose(dirtyNotes());
      } catch (again) {
        // The save did not take — a full disk, a permission, a conflict. The
        // workspace stays open, which is the only answer that keeps the buffer.
        get().fail(again);
        return false;
      }
    }
    // Only now: the core has let go, so nothing here can be pointing at a note
    // in a workspace that is no longer open.
    useEditor.getState().close();
    useTabs.getState().reset();
    set({ info: null, listings: {}, expanded: new Set(), error: null, notice: null });
    return true;
  },

  async switchTo(root) {
    if (!(await get().leave())) return;
    try {
      await get().adopt(await ipc.workspaceOpen(root));
    } catch (e) {
      // Left, and could not arrive. Welcome is the honest place to be, and the
      // error says why the folder did not open.
      get().fail(e);
    }
  },

  async createIn(parent) {
    const name = await askText({
      title: tr("welcome.create"),
      label: tr("welcome.createName"),
      initial: "notes",
      confirmLabel: tr("dialog.create"),
      validate: (v) => (v.trim() ? null : tr("dialog.nameRequired")),
    });
    if (!name) return;
    if (!(await get().leave())) return;
    try {
      await get().adopt(await ipc.workspaceCreate(parent, name));
    } catch (e) {
      get().fail(e);
    }
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

/**
 * The notes the core has to be told about before it will close.
 *
 * The same rule `stores/sync.ts` applies for reconciliation, and for the same
 * reason: the buffers live here and the core cannot see them
 * (`ARCHITECTURE.md` §5).
 */
function dirtyNotes(): NoteId[] {
  const doc = useEditor.getState().doc;
  return doc && doc.bufferVersion !== doc.savedVersion ? [doc.noteId] : [];
}
