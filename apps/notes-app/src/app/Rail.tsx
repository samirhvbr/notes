import { Files, Network, Search, Settings } from "lucide-react";
import { t } from "../i18n";
import { useUi, type Panel } from "../stores/ui";

/**
 * The icon rail (`.continue/0.1d-interface.md` §4.1).
 *
 * Four entries and one of them does nothing. **Graph is rendered disabled with
 * its milestone in the tooltip** rather than left out: ADR-038 moved it from
 * "out of scope" to 0.3, and a promise with a date on it is worth more to
 * someone deciding whether to adopt this than a rail with a gap in it. It is
 * `disabled`, so it takes no focus and cannot be activated by any route — §3's
 * *nada mente na tela* is about what a control claims, not about hiding what is
 * coming.
 *
 * Clicking the icon of the panel already showing collapses the sidebar, which
 * is the only way to give the editor the whole window.
 */
export function Rail({ onSettings }: { onSettings: () => void }) {
  const panel = useUi((s) => s.panel);
  const togglePanel = useUi((s) => s.togglePanel);

  const entry = (id: Panel, label: string, icon: React.ReactNode) => (
    <button
      type="button"
      className={panel === id ? "rail-btn on" : "rail-btn"}
      // `aria-pressed` rather than `aria-selected`: these are toggles, not tabs
      // in a tablist — pressing the active one turns it off.
      aria-pressed={panel === id}
      aria-label={label}
      title={label}
      onClick={() => togglePanel(id)}
    >
      {icon}
    </button>
  );

  return (
    <nav className="rail" aria-label={t("rail.label")}>
      {entry("files", t("rail.files"), <Files size={18} aria-hidden="true" />)}
      {entry("search", t("rail.search"), <Search size={18} aria-hidden="true" />)}
      <button
        type="button"
        className="rail-btn"
        disabled
        aria-label={t("rail.graph")}
        title={t("rail.graphSoon")}
      >
        <Network size={18} aria-hidden="true" />
      </button>

      <span className="rail-spacer" />

      <button
        type="button"
        className="rail-btn"
        aria-label={t("rail.settings")}
        title={t("rail.settings")}
        onClick={onSettings}
      >
        <Settings size={18} aria-hidden="true" />
      </button>
    </nav>
  );
}
