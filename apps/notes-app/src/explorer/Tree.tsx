import { useCallback, useEffect, useState } from "react";
import { useWorkspace } from "../stores/workspace";
import { useEditor } from "../stores/editor";
import { t } from "../i18n";
import { askConfirm, askText } from "../app/dialog";
import * as ipc from "../ipc";
import { ROOT, type Entry, type RelPath } from "../ipc";

/** Lazy tree: a directory is listed when it is first expanded, never up front. */
export function Tree() {
  const [menu, setMenu] = useState<{ entry: Entry; x: number; y: number } | null>(null);
  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("keydown", close);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", close);
    };
  }, [menu]);

  return (
    <>
      <Level dir={ROOT} depth={0} onMenu={setMenu} />
      {menu && <Actions entry={menu.entry} x={menu.x} y={menu.y} close={() => setMenu(null)} />}
    </>
  );
}

type OpenMenu = (m: { entry: Entry; x: number; y: number }) => void;

function Level({
  dir,
  depth,
  onMenu,
}: {
  dir: RelPath;
  depth: number;
  onMenu: OpenMenu;
}) {
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
              // Rename, duplicate, move and delete. On the row rather than in a
              // toolbar because they act on *that* entry, and a menu that acts
              // on a selection nobody can see is how the wrong file gets
              // deleted.
              onContextMenu={(ev) => {
                ev.preventDefault();
                onMenu({ entry: e, x: ev.clientX, y: ev.clientY });
              }}
              disabled={!isDir && !e.is_note}
              title={t("tree.actions.hint", { path: e.path })}
            >
              <span className="glyph" aria-hidden="true">
                {isDir ? (isOpen ? "▾" : "▸") : e.is_note ? "•" : "·"}
              </span>
              <span className="label">{e.name}</span>
            </button>
            {isDir && isOpen && <Level dir={e.path} depth={depth + 1} onMenu={onMenu} />}
          </li>
        );
      })}
    </ul>
  );
}

/**
 * The four entry operations of 0.1b.
 *
 * Each one re-lists the affected directories rather than patching the tree in
 * place: the disk is the source of truth for what exists, and a listing is
 * cheap (one level, no content read).
 */
function Actions({
  entry,
  x,
  y,
  close,
}: {
  entry: Entry;
  x: number;
  y: number;
  close: () => void;
}) {
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
      } finally {
        close();
      }
    },
    [fail, close],
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

  return (
    <div
      className="menu"
      style={{ left: x, top: y }}
      role="menu"
      onClick={(e) => e.stopPropagation()}
    >
      <button role="menuitem" onClick={rename}>{t("tree.rename")}</button>
      <button role="menuitem" onClick={move}>{t("tree.move")}</button>
      <button role="menuitem" onClick={duplicate}>{t("tree.duplicate")}</button>
      <button role="menuitem" className="danger" onClick={remove}>{t("tree.delete")}</button>
    </div>
  );
}

function parentOf(path: RelPath): RelPath {
  const i = path.lastIndexOf("/");
  return (i < 0 ? "" : path.slice(0, i)) as RelPath;
}
