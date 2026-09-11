import { Dialog } from "./DialogHost";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { t } from "../i18n";
import { askText } from "./dialog";
import * as ipc from "../ipc";
import { useWorkspace } from "../stores/workspace";
import type { WorkspaceEntry } from "../ipc";

export function Welcome() { return <><WelcomeContent/><Dialog/></>; }
function WelcomeContent() {
  const adopt = useWorkspace((s) => s.adopt);
  const fail = useWorkspace((s) => s.fail);
  const error=useWorkspace(s=>s.error);
  const [recent, setRecent] = useState<WorkspaceEntry[]>([]);

  useEffect(() => {
    ipc.workspaceRecent().then(setRecent).catch(() => setRecent([]));
  }, []);

  const pick = async (create: boolean) => {
    const picked = await openDialog({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    try {
      if (create) {
        const name = await askText({
          title: t("welcome.create"),
          label: t("welcome.createName"),
          initial: "notes",
          confirmLabel: t("dialog.create"),
          validate: (v) => (v.trim() ? null : t("dialog.nameRequired")),
        });
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
      <img src="/tura-icon.svg" width="88" height="88" alt="" />
      <h1>{t("welcome.title")}</h1>
      {error && <p role="alert">{t(`error.${error.code}`)}</p>}
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
