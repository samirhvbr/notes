import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { t } from "../i18n";
import * as ipc from "../ipc";
import { useWorkspace } from "../stores/workspace";
import type { WorkspaceEntry } from "../ipc";

export function Welcome() {
  const adopt = useWorkspace((s) => s.adopt);
  const fail = useWorkspace((s) => s.fail);
  const [recent, setRecent] = useState<WorkspaceEntry[]>([]);

  useEffect(() => {
    ipc.workspaceRecent().then(setRecent).catch(() => setRecent([]));
  }, []);

  const pick = async (create: boolean) => {
    const picked = await openDialog({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    try {
      if (create) {
        const name = window.prompt("Workspace name", "notes");
        if (!name) return;
        await adopt(await ipc.workspaceCreate(picked, name));
      } else {
        await adopt(await ipc.workspaceOpen(picked));
      }
    } catch (e) {
      fail(e);
    }
  };

  return (
    <div className="welcome">
      <h1>{t("welcome.title")}</h1>
      <p className="muted">{t("welcome.subtitle")}</p>
      <div className="actions">
        <button onClick={() => pick(false)}>{t("welcome.open")}</button>
        <button onClick={() => pick(true)}>{t("welcome.create")}</button>
      </div>
      {recent.length > 0 && (
        <section className="recent">
          <h2>{t("welcome.recent")}</h2>
          <ul>
            {recent.slice(0, 8).map((w) => (
              <li key={w.id}>
                <button
                  onClick={() =>
                    ipc.workspaceOpen(w.root).then(adopt).catch(fail)
                  }
                  title={w.root}
                >
                  {w.display_name}
                  <span className="muted"> — {w.root}</span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
