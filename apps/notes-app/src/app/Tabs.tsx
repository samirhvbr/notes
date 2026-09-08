import { t } from "../i18n";
import { useEditor } from "../stores/editor";
import { useTabs } from "../stores/tabs";

/**
 * The tab strip.
 *
 * A tab shows the file name; the path is the title, because two notes called
 * `notas.md` in different folders are the case where a name alone stops being
 * an identifier. The dirty marker is a dot rather than a colour, for the same
 * reason the status bar carries a word and a glyph: state never rides on colour
 * alone.
 */
export function Tabs() {
  const tabs = useTabs((s) => s.tabs);
  const activeId = useTabs((s) => s.activeId);
  const activate = useTabs((s) => s.activate);
  const close = useTabs((s) => s.close);
  const doc = useEditor((s) => s.doc);

  if (tabs.length === 0) return null;

  return (
    <div className="tabs" role="tablist" aria-label={t("tabs.label")}>
      {tabs.map((tab) => {
        const active = tab.noteId === activeId;
        const dirty = active && doc ? doc.bufferVersion !== doc.savedVersion : false;
        const name = tab.path.split("/").pop() ?? tab.path;
        return (
          <div key={tab.noteId} className={active ? "tab on" : "tab"} title={tab.path}>
            <button
              role="tab"
              aria-selected={active}
              className="tab-label"
              onClick={() => void activate(tab.noteId)}
            >
              <span className="tab-dirty" aria-hidden="true">
                {dirty ? "●" : ""}
              </span>
              {name}
            </button>
            <button
              className="tab-close"
              aria-label={t("tabs.close", { name })}
              onClick={(e) => {
                e.stopPropagation();
                void close(tab.noteId);
              }}
            >
              ×
            </button>
          </div>
        );
      })}
    </div>
  );
}
