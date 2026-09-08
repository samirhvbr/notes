import { create } from "zustand";
import * as ipc from "../ipc";
import type { Settings } from "../ipc";
import { setLocale } from "../i18n";

/**
 * The settings the editor actually reads.
 *
 * They live in a store rather than being passed down because the editor is
 * rebuilt when they change — CodeMirror's line numbers and wrapping are
 * extensions, not props — and a component that rebuilds itself needs to know
 * *when*, not just *what*.
 */
interface SettingsState {
  settings: Settings | null;
  load: () => Promise<void>;
  apply: (s: Settings) => void;
}

export const useSettings = create<SettingsState>((set) => ({
  settings: null,

  async load() {
    try {
      const s = await ipc.settingsGet();
      if (s.ui.locale !== "auto") setLocale(s.ui.locale);
      set({ settings: s });
    } catch {
      // Defaults are safe and the core already fell back to them; a settings
      // file that cannot be read is not a reason to refuse to start.
    }
  },

  apply: (settings) => set({ settings }),
}));
