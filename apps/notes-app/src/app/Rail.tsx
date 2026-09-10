import { Files, Network, Search, Settings } from "lucide-react";
import { t } from "../i18n";
import { useUi, type Panel } from "../stores/ui";

/** Sidebar panels and the graph share the same keyboard-accessible rail. */
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
      {entry("graph", t("rail.graph"), <Network size={18} aria-hidden="true" />)}

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
