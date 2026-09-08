import { create } from "zustand";
import type { RelPath } from "../ipc";
import { useTabs } from "./tabs";

/**
 * Back and forward, for the note header's two arrows.
 *
 * **One history for the window, not one per tab.** The milestone document says
 * *"histórico da aba"*, and a per-tab history is the wrong shape for what this
 * application does: a tab here is a note, not a viewport — opening a note from
 * the tree, from quick open or from a search hit either activates its existing
 * tab or makes one (ADR-030). There is no navigation *within* a tab to have a
 * history of. What a person actually means by "back" is *the note I was looking
 * at before this one*, and that is a window-level fact
 * (`DECISIONS-0.1d.md` D-06).
 *
 * The list is paths rather than `NoteId`s so that a note deleted while it is in
 * the history simply fails to open and is dropped, instead of holding an
 * identity the registry has forgotten.
 */
interface HistoryState {
  /** Oldest first. `at` indexes the entry currently on screen. */
  entries: RelPath[];
  at: number;
  /** Set while back/forward is driving, so the visit it causes is not recorded. */
  navigating: boolean;

  /** Called by the tabs store every time a note becomes the active one. */
  visited: (path: RelPath) => void;
  go: (delta: number) => Promise<void>;
  reset: () => void;
}

/** Bounded, because it is a convenience and not a record. */
const LIMIT = 100;

export const useHistoryStore = create<HistoryState>((set, get) => ({
  entries: [],
  at: -1,
  navigating: false,

  visited(path) {
    const { entries, at, navigating } = get();
    if (navigating) return;
    if (entries[at] === path) return;

    // Everything ahead of the cursor is dropped, which is what makes this a
    // history rather than a ring: arriving somewhere new from the middle means
    // the forward branch is no longer the one you were on.
    const kept = entries.slice(0, at + 1);
    kept.push(path);
    const trimmed = kept.slice(-LIMIT);
    set({ entries: trimmed, at: trimmed.length - 1 });
  },

  async go(delta) {
    const { entries, at } = get();
    const next = at + delta;
    if (next < 0 || next >= entries.length) return;
    set({ navigating: true, at: next });
    try {
      await useTabs.getState().openPath(entries[next]);
    } catch {
      // The note is gone. The cursor stays where it landed rather than being
      // wound back — pressing back twice on a deleted note should keep going
      // back, not stick.
    } finally {
      set({ navigating: false });
    }
  },

  reset: () => set({ entries: [], at: -1, navigating: false }),
}));

/** What the header binds to. */
export function useHistory() {
  const entries = useHistoryStore((s) => s.entries);
  const at = useHistoryStore((s) => s.at);
  const go = useHistoryStore((s) => s.go);
  return {
    canBack: at > 0,
    canForward: at >= 0 && at < entries.length - 1,
    back: () => go(-1),
    forward: () => go(1),
  };
}
