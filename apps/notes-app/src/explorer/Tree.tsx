import { useWorkspace } from "../stores/workspace";
import { useEditor } from "../stores/editor";
import { t } from "../i18n";
import { ROOT, type Entry, type RelPath } from "../ipc";

/** Lazy tree: a directory is listed when it is first expanded, never up front. */
export function Tree() {
  return <Level dir={ROOT} depth={0} />;
}

function Level({ dir, depth }: { dir: RelPath; depth: number }) {
  const entries = useWorkspace((s) => s.listings[dir]);
  const expanded = useWorkspace((s) => s.expanded);
  const toggle = useWorkspace((s) => s.toggle);
  const open = useEditor((s) => s.open);
  const active = useEditor((s) => s.doc?.path);

  if (!entries) return null;
  if (entries.length === 0 && depth === 0)
    return <p className="muted pad">{t("tree.empty")}</p>;

  return (
    <ul className="tree" role="group">
      {entries.map((e: Entry) => {
        const isDir = e.kind === "Dir";
        const isOpen = expanded.has(e.path);
        return (
          <li key={e.path}>
            <button
              className={active === e.path ? "row on" : "row"}
              style={{ paddingLeft: 8 + depth * 14 }}
              aria-expanded={isDir ? isOpen : undefined}
              onClick={() => (isDir ? toggle(e.path) : open(e.path).catch(() => {}))}
              disabled={!isDir && !e.is_note}
              title={e.path}
            >
              <span className="glyph" aria-hidden="true">
                {isDir ? (isOpen ? "▾" : "▸") : e.is_note ? "•" : "·"}
              </span>
              <span className="label">{e.name}</span>
            </button>
            {isDir && isOpen && <Level dir={e.path} depth={depth + 1} />}
          </li>
        );
      })}
    </ul>
  );
}
