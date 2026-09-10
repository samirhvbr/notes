import { create } from "zustand";
import * as ipc from "../ipc";

/** Source · Preview · Split (scope §9). Live Preview and WYSIWYG are out of the MVP. */
export type ViewMode = "source" | "preview" | "split";

const MODES: ViewMode[] = ["source", "preview", "split"];

/**
 * Which panel the sidebar is showing, or `null` for a collapsed sidebar.
 *
 * The rail's icons toggle this, and clicking the icon of the panel already
 * showing collapses the sidebar (`.continue/0.1d-interface.md` §4.1). Graph
 * uses the main area and shares the knowledge index with backlinks.
 */
export type Panel = "files" | "search" | "graph";

interface UiState {
  view: ViewMode;
  /** `null` means the sidebar is collapsed. */
  panel: Panel | null;
  /** The conflict comparison screen. Not a command — it changes nothing on
   *  disk and reads two strings this frontend already holds
   *  (docs/ARCHITECTURE.md §17.1). */
  comparing: boolean;

  setView: (v: ViewMode) => void;
  /** Show a panel; showing the one already shown collapses the sidebar. */
  togglePanel: (p: Panel) => void;
  /** `Ctrl+E`, per scope §9's shortcut table. */
  cycleView: () => void;
  setComparing: (b: boolean) => void;
  hydrate: () => Promise<void>;
}

function isMode(v: string): v is ViewMode {
  return (MODES as string[]).includes(v);
}

/** Persisted through `session_save`, debounced, exactly like every other piece
 *  of UI state (docs/ARCHITECTURE.md §4.4). A failure to persist is never
 *  allowed to break the toggle itself. */
let persist: ReturnType<typeof setTimeout> | null = null;
function remember(view: ViewMode) {
  if (persist) clearTimeout(persist);
  persist = setTimeout(() => {
    void ipc
      .sessionGet()
      .then((s) => ipc.sessionSave({ ...s, view_mode: view }))
      .catch(() => {});
  }, 1000);
}

export const useUi = create<UiState>((set, get) => ({
  view: "source",
  panel: "files",
  comparing: false,

  togglePanel(p) {
    set((s) => ({ panel: s.panel === p ? null : p }));
  },

  setView(view) {
    set({ view });
    remember(view);
  },

  cycleView() {
    const next = MODES[(MODES.indexOf(get().view) + 1) % MODES.length];
    get().setView(next);
  },

  setComparing: (comparing) => set({ comparing }),

  async hydrate() {
    try {
      const session = await ipc.sessionGet();
      if (isMode(session.view_mode)) set({ view: session.view_mode });
    } catch {
      // An unreadable session starts an empty one and never stops the app
      // opening (docs/ARCHITECTURE.md §4.4).
    }
  },
}));
