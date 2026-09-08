import { MoreVertical } from "lucide-react";
import { useCallback, useMemo, useRef, useState } from "react";
import { useWorkspace } from "../stores/workspace";
import { useEditor } from "../stores/editor";
import { useTabs } from "../stores/tabs";
import { t } from "../i18n";
import { askConfirm, askText } from "../app/dialog";
import { Menu, type MenuRow } from "../app/Menu";
import * as ipc from "../ipc";
import { ROOT, type Entry, type RelPath } from "../ipc";

/** Lazy tree: a directory is listed when it is first expanded, never up front. */
export function Tree() {
  return <Level dir={ROOT} depth={0} />;
}

function Level({ dir, depth }: { dir: RelPath; depth: number }) {
  const entries = useWorkspace((s) => s.listings[dir]);
  const expanded = useWorkspace((s) => s.expanded);
  const toggle = useWorkspace((s) => s.toggle);
  const sort = useWorkspace((s) => s.sort);
  const open = useTabs((s) => s.openPath);
  const active = useEditor((s) => s.doc?.path);

  // Directories stay first whatever the sort: that is a structural fact about a
  // tree, not a preference about order. The sort reorders within each kind.
  const ordered = useMemo(() => {
    if (!entries) return entries;
    const cmp = (a: Entry, b: Entry) =>
      a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
    return [...entries].sort((a, b) => {
      const da = a.kind !== "Dir" ? 1 : 0;
      const db = b.kind !== "Dir" ? 1 : 0;
      return da - db || (sort === "name" ? cmp(a, b) : cmp(b, a));
    });
  }, [entries, sort]);

  if (!ordered) return null;
  if (ordered.length === 0 && depth === 0)
    return <p className="muted pad">{t("tree.empty")}</p>;

  return (
    <ul className="tree" role="group">
      {ordered.map((e: Entry) => {
        const isDir = e.kind === "Dir";
        const isOpen = expanded.has(e.path);
        return (
          <li key={e.path}>
            <Row entry={e} depth={depth} isDir={isDir} isOpen={isOpen}
                 selected={active === e.path}
                 onActivate={() => (isDir ? toggle(e.path) : open(e.path).catch(() => {}))} />
            {isDir && isOpen && <Level dir={e.path} depth={depth + 1} />}
          </li>
        );
      })}
    </ul>
  );
}

/**
 * One row, and its menu.
 *
 * The four entry operations of 0.1b live **on the entry** rather than in the
 * toolbar, because they act on *that* entry and a menu that operates on a
 * selection nobody can see is how the wrong file gets deleted.
 *
 * Reachable two ways, which `.continue/0.1d-interface.md` §4.2 requires: right
 * click anywhere on the row, or the `⋮` button — which is a real focusable
 * control, so the keyboard gets there by tabbing rather than by a shortcut
 * nobody discovers. The menu itself is the shared one, so arrows, `Escape` and
 * the focus return are the same everywhere in the application.
 */
function Row({
  entry,
  depth,
  isDir,
  isOpen,
  selected,
  onActivate,
}: {
  entry: Entry;
  depth: number;
  isDir: boolean;
  isOpen: boolean;
  selected: boolean;
  onActivate: () => void;
}) {
  const [menu, setMenu] = useState(false);
  const trigger = useRef<HTMLButtonElement | null>(null);
  const rows = useEntryActions(entry);

  return (
    <div
      className={selected ? "row-wrap on" : "row-wrap"}
      onContextMenu={(ev) => {
        ev.preventDefault();
        setMenu(true);
      }}
    >
      <button
        className="row"
        style={{ paddingLeft: 8 + depth * 14 }}
        aria-expanded={isDir ? isOpen : undefined}
        aria-current={selected || undefined}
        onClick={onActivate}
        disabled={!isDir && !entry.is_note}
        title={entry.path}
      >
        <span className="glyph" aria-hidden="true">
          {isDir ? (isOpen ? "▾" : "▸") : entry.is_note ? "•" : "·"}
        </span>
        <span className="label">{entry.name}</span>
      </button>
      <button
        ref={trigger}
        type="button"
        className="row-more"
        aria-haspopup="menu"
        aria-expanded={menu}
        aria-label={t("tree.actions.hint", { path: entry.path })}
        onClick={(e) => {
          e.stopPropagation();
          setMenu((m) => !m);
        }}
      >
        <MoreVertical size={14} aria-hidden="true" />
      </button>
      <Menu
        rows={rows}
        open={menu}
        onClose={() => setMenu(false)}
        label={t("tree.actions.hint", { path: entry.path })}
        align="end"
        trigger={trigger}
      />
    </div>
  );
}

/**
 * Rename · move · duplicate · delete.
 *
 * Each one re-lists the affected directories rather than patching the tree in
 * place: the disk is the source of truth for what exists, and a listing is
 * cheap (one level, no content read).
 */
function useEntryActions(entry: Entry): MenuRow[] {
  const refresh = useWorkspace((s) => s.refresh);
  const fail = useWorkspace((s) => s.fail);
  const note = useWorkspace((s) => s.note);
  const parent = parentOf(entry.path);

  const run = useCallback(
    async (fn: () => Promise<void>) => {
      try {
        await fn();
      } catch (e) {
        fail(e);
      }
    },
    [fail],
  );

  const rename = () =>
    run(async () => {
      const name = await askText({
        title: t("tree.rename"),
        label: t("tree.rename.prompt"),
        initial: entry.name,
        confirmLabel: t("dialog.rename"),
        validate: (v) => (v.trim() ? null : t("dialog.nameRequired")),
      });
      if (!name || name === entry.name) return;
      const moved = await ipc.entryRename(entry.path, name);
      await refresh(parent);
      // The tab keeps its identity, so the open note only needs its new path.
      useEditor.getState().repath(entry.path, moved.path);
    });

  const move = () =>
    run(async () => {
      const dir = await askText({
        title: t("tree.move"),
        label: t("tree.move.prompt"),
        initial: parent,
        confirmLabel: t("dialog.move"),
      });
      if (dir === null) return;
      const target = (dir.trim() === "/" ? "" : dir.trim()) as RelPath;
      const moved = await ipc.entryMove(entry.path, target);
      await refresh(parent);
      await refresh(target);
      useEditor.getState().repath(entry.path, moved.path);
    });

  const duplicate = () =>
    run(async () => {
      const copy = await ipc.entryDuplicate(entry.path);
      await refresh(parent);
      note(t("tree.duplicate.done", { name: copy.name }));
    });

  const remove = () =>
    run(async () => {
      const sure = await askConfirm({
        title: t("tree.delete"),
        body: t("tree.delete.confirm", { name: entry.name }),
        confirmLabel: t("dialog.delete"),
        danger: true,
      });
      if (!sure) return;
      const gone = await ipc.entryDelete(entry.path);
      await refresh(parent);
      const doc = useEditor.getState().doc;
      if (doc && gone.note_ids.includes(doc.noteId)) useEditor.getState().close();
      // Scope §7.7: never delete without saying which of the two happened.
      note(
        gone.outcome === "trashed"
          ? t("tree.delete.trashed", { name: entry.name })
          : t("tree.delete.permanent", { name: entry.name }),
      );
    });

  return [
    { id: "rename", label: t("tree.rename"), run: rename },
    { id: "move", label: t("tree.move"), run: move },
    { id: "duplicate", label: t("tree.duplicate"), run: duplicate },
    { separator: true },
    { id: "delete", label: t("tree.delete"), danger: true, run: remove },
  ];
}

function parentOf(path: RelPath): RelPath {
  const i = path.lastIndexOf("/");
  return (i < 0 ? "" : path.slice(0, i)) as RelPath;
}
