import { create } from "zustand";
import * as ipc from "../ipc";

/** Source · Preview · Split (scope §9). Live Preview and WYSIWYG are out of the MVP. */
export type ViewMode = "source" | "preview" | "split";

const MODES: ViewMode[] = ["source", "preview", "split"];

interface UiState {
  view: ViewMode;
  /** The conflict comparison screen. Not a command — it changes nothing on
   *  disk and reads two strings this frontend already holds
   *  (docs/ARCHITECTURE.md §17.1). */
  comparing: boolean;

  setView: (v: ViewMode) => void;
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
  comparing: false,

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
