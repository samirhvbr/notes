import { create } from "zustand";
import * as ipc from "../ipc";
import type { NoteId, RelPath, Session, Tab } from "../ipc";
import { useEditor } from "./editor";

/**
 * Tabs, milestone 0.1c.
 *
 * A **separate** store rather than a rewrite of the editor's, on purpose: the
 * editor holds exactly one loaded document and everything in 0.1a and 0.1b is
 * written against that. Making every consumer tab-aware to gain a tab strip
 * would put the whole write protocol back in play for a navigation feature.
 *
 * So this store owns the *list* and the editor owns the *document*. A tab
 * carries only what has to survive a restart — path, identity, cursor, scroll —
 * and the buffer lives where it always did.
 *
 * Switching away from a dirty note **flushes it** first, and a note in conflict
 * writes its draft instead of being saved. Neither is a special case invented
 * here: they are the two things `ARCHITECTURE.md` §5 already says happen when a
 * buffer stops being looked at.
 */
export interface TabEntry {
  noteId: NoteId;
  path: RelPath;
  line: number;
  col: number;
  scrollTop: number;
  pinned: boolean;
}

interface TabsState {
  tabs: TabEntry[];
  activeId: NoteId | null;
  /** Set while `restore` is applying a saved session, so the cursor coming from
   *  a freshly mounted editor does not overwrite the one being restored. */
  restoring: boolean;

  openPath: (path: RelPath) => Promise<void>;
  /** Open a note **at a position** — what clicking a search hit means. */
  openAt: (path: RelPath, line: number, col: number) => Promise<void>;
  /** Bumped when a jump is requested into a note that is already mounted, so
   *  the editor moves the caret instead of waiting for a rebuild that will not
   *  happen. */
  gotoRev: number;
  activate: (noteId: NoteId) => Promise<void>;
  close: (noteId: NoteId) => Promise<void>;
  closeActive: () => Promise<void>;
  noteCursor: (noteId: NoteId, line: number, col: number, scrollTop: number) => void;
  repath: (noteId: NoteId, to: RelPath) => void;
  forget: (noteIds: NoteId[]) => void;
  reset: () => void;

  persist: () => Promise<void>;
  restore: () => Promise<void>;
}

/** Session writes are debounced: a cursor moves on every keystroke. */
let persistTimer: ReturnType<typeof setTimeout> | null = null;

export const useTabs = create<TabsState>((set, get) => ({
  tabs: [],
  activeId: null,
  restoring: false,
  gotoRev: 0,

  reset() {
    if (persistTimer) clearTimeout(persistTimer);
    void import("./history").then((m) => m.useHistoryStore.getState().reset());
    set({ tabs: [], activeId: null, restoring: false });
  },

  async openPath(path) {
    const existing = get().tabs.find((t) => t.path === path);
    if (existing) return get().activate(existing.noteId);

    await leaveCurrent();
    await useEditor.getState().open(path);
    const doc = useEditor.getState().doc;
    if (!doc) return;

    set((s) => ({
      tabs: s.tabs.some((t) => t.noteId === doc.noteId)
        ? s.tabs
        : [...s.tabs, { noteId: doc.noteId, path: doc.path, line: 1, col: 1, scrollTop: 0, pinned: false }],
      activeId: doc.noteId,
    }));
    visited(doc.path);
    schedulePersist(get);
  },

  async openAt(path, line, col) {
    const existing = get().tabs.find((t) => t.path === path);
    if (existing) {
      // Set the target first: if the editor rebuilds it reads this, and if it
      // does not, `gotoRev` tells it to move.
      set((s) => ({
        tabs: s.tabs.map((t) => (t.noteId === existing.noteId ? { ...t, line, col } : t)),
      }));
      await get().activate(existing.noteId);
      set((s) => ({ gotoRev: s.gotoRev + 1 }));
      return;
    }
    await get().openPath(path);
    const doc = useEditor.getState().doc;
    if (!doc || doc.path !== path) return;
    set((s) => ({
      tabs: s.tabs.map((t) => (t.noteId === doc.noteId ? { ...t, line, col } : t)),
      gotoRev: s.gotoRev + 1,
    }));
  },

  async activate(noteId) {
    if (get().activeId === noteId && useEditor.getState().doc?.noteId === noteId) return;
    const tab = get().tabs.find((t) => t.noteId === noteId);
    if (!tab) return;

    await leaveCurrent();
    await useEditor.getState().open(tab.path);
    set({ activeId: noteId });
    visited(tab.path);
    schedulePersist(get);
  },

  async close(noteId) {
    const { tabs, activeId } = get();
    const i = tabs.findIndex((t) => t.noteId === noteId);
    if (i < 0) return;

    if (activeId === noteId) await leaveCurrent();
    const rest = tabs.filter((t) => t.noteId !== noteId);

    if (activeId !== noteId) {
      set({ tabs: rest });
      schedulePersist(get);
      return;
    }
    // Closing the active tab activates its neighbour — the one to the right,
    // then the one to the left, which is what every editor does.
    const next = rest[i] ?? rest[i - 1] ?? null;
    set({ tabs: rest, activeId: next?.noteId ?? null });
    if (next) {
      await useEditor.getState().open(next.path);
    } else {
      useEditor.getState().close();
    }
    schedulePersist(get);
  },

  async closeActive() {
    const id = get().activeId;
    if (id) await get().close(id);
  },

  noteCursor(noteId, line, col, scrollTop) {
    if (get().restoring) return;
    let changed = false;
    const tabs = get().tabs.map((t) => {
      if (t.noteId !== noteId) return t;
      if (t.line === line && t.col === col && t.scrollTop === scrollTop) return t;
      changed = true;
      return { ...t, line, col, scrollTop };
    });
    if (!changed) return;
    set({ tabs });
    schedulePersist(get);
  },

  repath(noteId, to) {
    set((s) => ({ tabs: s.tabs.map((t) => (t.noteId === noteId ? { ...t, path: to } : t)) }));
    schedulePersist(get);
  },

  /** A note that no longer exists loses its tab — deletion, or an external
   *  removal reconciliation reported. */
  forget(noteIds) {
    if (noteIds.length === 0) return;
    set((s) => {
      const tabs = s.tabs.filter((t) => !noteIds.includes(t.noteId));
      const activeGone = s.activeId !== null && noteIds.includes(s.activeId);
      return { tabs, activeId: activeGone ? (tabs[0]?.noteId ?? null) : s.activeId };
    });
    schedulePersist(get);
  },

  async persist() {
    if (persistTimer) {
      clearTimeout(persistTimer);
      persistTimer = null;
    }
    const { tabs, activeId } = get();
    try {
      const previous = await ipc.sessionGet();
      const session: Session = {
        ...previous,
        tabs: tabs.map(
          (t): Tab => ({
            note_id: t.noteId,
            path: t.path,
            line: t.line,
            col: t.col,
            scroll_top: t.scrollTop,
            pinned: t.pinned,
          }),
        ),
        active_tab: activeId,
      };
      await ipc.sessionSave(session);
    } catch {
      // A session that cannot be written is not worth interrupting the work
      // for; it is UI state, and the core treats an unreadable one as empty.
    }
  },

  /**
   * Reopen what was open. The 0.1c criterion is *"reabrir o app restaura
   * workspace, abas, aba ativa e cursor"*, and the cursor is the part that is
   * easy to lose: it has to be applied **after** the editor has mounted the
   * document, which is why `restoring` exists.
   */
  async restore() {
    let session: Session;
    try {
      session = await ipc.sessionGet();
    } catch {
      return;
    }
    if (session.tabs.length === 0) {
      set({ tabs: [], activeId: null });
      return;
    }

    const tabs: TabEntry[] = session.tabs.map((t) => ({
      noteId: t.note_id,
      path: t.path,
      line: t.line,
      col: t.col,
      scrollTop: t.scroll_top,
      pinned: t.pinned,
    }));
    const active = session.active_tab ?? tabs[0]?.noteId ?? null;
    set({ tabs, activeId: active, restoring: true });

    const tab = tabs.find((t) => t.noteId === active) ?? tabs[0];
    if (tab) {
      try {
        await useEditor.getState().open(tab.path);
      } catch {
        // The note is gone. Its tab goes with it rather than sitting there
        // failing to open every time it is clicked.
        set((s) => ({
          tabs: s.tabs.filter((t) => t.noteId !== tab.noteId),
          activeId: null,
        }));
      }
    }
    set({ restoring: false });
  },
}));

/** The cursor the editor should place after mounting, if any. */
export function pendingCursor(noteId: NoteId | undefined): { line: number; col: number } | null {
  if (!noteId) return null;
  const t = useTabs.getState().tabs.find((x) => x.noteId === noteId);
  return t ? { line: t.line, col: t.col } : null;
}

/**
 * Tell the back/forward history a note is now on screen.
 *
 * Imported lazily so the two stores do not import each other at module load —
 * `history.ts` needs `openPath` to navigate, and this needs `visited` to
 * record. The cycle is real and it is fine at call time; at import time it is
 * an undefined function.
 */
function visited(path: RelPath) {
  void import("./history").then((m) => m.useHistoryStore.getState().visited(path));
}

/** Leave the current document safely before another takes its place. */
async function leaveCurrent(): Promise<void> {
  const doc = useEditor.getState().doc;
  if (!doc) return;
  const dirty = doc.bufferVersion !== doc.savedVersion;
  if (!dirty) return;
  if (doc.conflict) {
    // Suspended: the buffer goes to the draft, never to the note.
    await useEditor.getState().keepDraft("conflict");
  } else {
    await useEditor.getState().save(true);
  }
}

function schedulePersist(get: () => TabsState) {
  if (persistTimer) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    void get().persist();
  }, 1000);
}
