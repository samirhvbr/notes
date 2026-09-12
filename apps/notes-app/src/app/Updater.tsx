import { useEffect, type ReactNode } from "react";
import { useUpdater } from "../stores/updater";
import { t } from "../i18n";

export function UpdateButton() {
  const { phase, check } = useUpdater();
  return <div className="update-manual">
    <button type="button" disabled={phase === "checking" || phase === "installing"} onClick={() => void check(true)}>{t("update.check")}</button>
    {["checking", "current", "unsupported", "error"].includes(phase) && <p role="status">{t(`update.${phase}`)}</p>}
  </div>;
}

export function UpdaterShell({ children }: { children: ReactNode }) {
  const { phase, version, notes, check, dismiss, install } = useUpdater();
  useEffect(() => {
    const initial = setTimeout(() => void check(), 20_000);
    const periodic = setInterval(() => void check(), 6 * 60 * 60 * 1000);
    return () => { clearTimeout(initial); clearInterval(periodic); };
  }, [check]);
  const visible = version && ["available", "closeWorkspace", "error", "installing"].includes(phase);
  return <>{children}{visible && <aside className="update-banner" aria-label={t("update.title")}>
    <strong>{t("update.title")} {version}</strong>
    {notes && <p>{notes}</p>}
    <p role="status">{t(`update.${phase === "available" ? "confirm" : phase}`)}</p>
    {phase !== "installing" && <div className="actions">
      <button type="button" onClick={() => void install()}>{t("update.install")}</button>
      <button type="button" onClick={dismiss}>{t("update.later")}</button>
    </div>}
  </aside>}</>;
}
