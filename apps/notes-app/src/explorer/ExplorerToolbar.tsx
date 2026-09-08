import { ArrowDownAZ, ArrowUpZA, FilePlus, FolderPlus, ListCollapse } from "lucide-react";
import { askText } from "../app/dialog";
import { t } from "../i18n";
import * as ipc from "../ipc";
import { ROOT } from "../ipc";
import { useTabs } from "../stores/tabs";
import { useWorkspace } from "../stores/workspace";

/**
 * The explorer's toolbar (`.continue/0.1d-interface.md` §4.2).
 *
 * Four buttons, and all four call something that already existed — they were
 * previously scattered across the window's top bar, where they read as
 * application-wide actions rather than as things done *to the explorer*.
 *
 * New note and new folder create at the **root**, not inside the selected
 * directory. A toolbar acts on the panel; the entry-level operations act on an
 * entry and stay on the entry's own menu, because an action that operates on a
 * selection nobody can see is how the wrong file gets changed.
 */
export function ExplorerToolbar() {
  const refresh = useWorkspace((s) => s.refresh);
  const fail = useWorkspace((s) => s.fail);
  const sort = useWorkspace((s) => s.sort);
  const setSort = useWorkspace((s) => s.setSort);
  const collapseAll = useWorkspace((s) => s.collapseAll);
  const openPath = useTabs((s) => s.openPath);

  const newNote = async () => {
    const name = await askText({
      title: t("tree.newNote"),
      label: t("tree.newNote.prompt"),
      initial: "",
      confirmLabel: t("dialog.create"),
      validate: (v) => (v.trim() ? null : t("dialog.nameRequired")),
    });
    if (!name) return;
    try {
      const entry = await ipc.noteCreate(ROOT, name);
      await refresh(ROOT);
      // Created and opened: creating a note in order to look at an empty
      // explorer is not what anyone meant by "new note".
      await openPath(entry.path);
    } catch (e) {
      fail(e);
    }
  };

  const newFolder = async () => {
    const name = await askText({
      title: t("tree.newFolder"),
      label: t("tree.newFolder.prompt"),
      initial: "",
      confirmLabel: t("dialog.create"),
      validate: (v) => (v.trim() ? null : t("dialog.nameRequired")),
    });
    if (!name) return;
    try {
      await ipc.dirCreate(ROOT, name);
      await refresh(ROOT);
    } catch (e) {
      fail(e);
    }
  };

  const ascending = sort === "name";

  return (
    <div className="ex-toolbar" role="toolbar" aria-label={t("explorer.toolbar")}>
      <span className="ex-title">{t("explorer.title")}</span>
      <span className="spacer" />
      <button
        type="button"
        className="icon-btn"
        aria-label={t("tree.newNote")}
        title={t("tree.newNote")}
        onClick={() => void newNote()}
      >
        <FilePlus size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        className="icon-btn"
        aria-label={t("tree.newFolder")}
        title={t("tree.newFolder")}
        onClick={() => void newFolder()}
      >
        <FolderPlus size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        className="icon-btn"
        // The label says what pressing it *does*, not what is currently true:
        // a toggle labelled with its own state reads backwards to a screen
        // reader the moment it is pressed.
        aria-label={ascending ? t("explorer.sortDesc") : t("explorer.sortAsc")}
        title={ascending ? t("explorer.sortDesc") : t("explorer.sortAsc")}
        onClick={() => setSort(ascending ? "name-desc" : "name")}
      >
        {ascending ? (
          <ArrowDownAZ size={15} aria-hidden="true" />
        ) : (
          <ArrowUpZA size={15} aria-hidden="true" />
        )}
      </button>
      <button
        type="button"
        className="icon-btn"
        aria-label={t("explorer.collapse")}
        title={t("explorer.collapse")}
        onClick={collapseAll}
      >
        <ListCollapse size={15} aria-hidden="true" />
      </button>
    </div>
  );
}
