import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ChevronUp, FolderOpen, FolderPlus, LogOut } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { t } from "../i18n";
import * as ipc from "../ipc";
import type { WorkspaceEntry } from "../ipc";
import { useWorkspace } from "../stores/workspace";
import { Menu, type MenuRow } from "./Menu";

/**
 * The workspace selector, in the sidebar footer.
 *
 * **This closes a defect, not a gap in polish.** Every command it needs has
 * existed since 0.1a — `workspace_open`, `workspace_create`,
 * `workspace_recent`, `workspace_close` — and the only surface that reached
 * them was the Welcome screen, which disappears the moment a folder is opened.
 * After that first open there was **no way to change folder at all**
 * (`.continue/0.1d-interface.md` §1). A command with no route to the user is a
 * command that does not exist.
 *
 * Switching is `close` then `open`, in that order and never overlapping, so the
 * one guard that protects unsaved work — `close_workspace`'s `DirtyBuffers`
 * refusal — is on the path rather than beside it (§4.2).
 */
export function WorkspaceMenu() {
  const info = useWorkspace((s) => s.info);
  const leave = useWorkspace((s) => s.leave);
  const switchTo = useWorkspace((s) => s.switchTo);
  const createIn = useWorkspace((s) => s.createIn);
  const [open, setOpen] = useState(false);
  const [recent, setRecent] = useState<WorkspaceEntry[]>([]);
  const trigger = useRef<HTMLButtonElement | null>(null);

  // Read when the menu opens rather than once on mount: the list changes every
  // time a workspace is opened, including by this menu.
  useEffect(() => {
    if (!open) return;
    let live = true;
    ipc
      .workspaceRecent()
      .then((r) => live && setRecent(r))
      .catch(() => live && setRecent([]));
    return () => {
      live = false;
    };
  }, [open]);

  const pickFolder = async (create: boolean) => {
    const picked = await openDialog({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    if (create) await createIn(picked);
    else await switchTo(picked);
  };

  const others = recent.filter((w) => w.root !== info?.root).slice(0, 6);

  const rows: MenuRow[] = [
    {
      id: "open",
      label: t("workspace.open"),
      icon: <FolderOpen size={14} />,
      run: () => pickFolder(false),
    },
    {
      id: "create",
      label: t("workspace.create"),
      icon: <FolderPlus size={14} />,
      run: () => pickFolder(true),
    },
    ...(others.length
      ? ([{ separator: true, label: t("workspace.recent") }] as MenuRow[])
      : []),
    ...others.map(
      (w): MenuRow => ({
        id: w.id,
        label: w.display_name,
        hint: w.root,
        title: w.root,
        run: () => void switchTo(w.root),
      }),
    ),
    { separator: true },
    {
      id: "close",
      label: t("workspace.close"),
      icon: <LogOut size={14} />,
      run: () => void leave(),
    },
  ];

  return (
    <div className="ws-menu">
      <button
        ref={trigger}
        type="button"
        className="ws-trigger"
        aria-haspopup="menu"
        aria-expanded={open}
        title={info?.root ?? undefined}
        onClick={() => setOpen((o) => !o)}
      >
        <ChevronUp size={14} aria-hidden="true" className="ws-chevron" />
        <span className="ws-name">{info?.display_name ?? t("workspace.none")}</span>
      </button>
      <Menu
        rows={rows}
        open={open}
        onClose={() => setOpen(false)}
        label={t("workspace.menu")}
        trigger={trigger}
      />
    </div>
  );
}
