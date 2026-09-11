import { DeviceSync } from "./DeviceSync";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { t } from "../i18n";
import * as ipc from "../ipc";
import { beginSyncBarrier, endSyncBarrier, isSyncLocked, subscribeBarrier, setComposing } from "../ipc/barrier";
import { acceptSyncReload, useEditor } from "../stores/editor";
import type { OpenDoc } from "../stores/editor";
import { useSync } from "../stores/sync";
import { useWorkspace } from "../stores/workspace";
import { errorText } from "./StatusBar";

export function ReceivedSyncShell({ children }: { children: ReactNode }) {
  const locked = useSyncExternalStore(subscribeBarrier, isSyncLocked);
  useEffect(() => {
    const guard = (event: Event) => {
      if (!isSyncLocked()) return;
      const target = event.target;
      if (target instanceof Element && target.closest("[data-sync-controls]")) return;
      event.preventDefault();
      event.stopImmediatePropagation();
    };
    const startComposition = () => setComposing(true);
    const endComposition = () => setComposing(false);
    document.addEventListener("compositionstart", startComposition, true);
    document.addEventListener("compositionend", endComposition, true);
    const events = ["keydown", "beforeinput", "paste", "cut", "drop", "pointerdown", "click"];
    for (const event of events) document.addEventListener(event, guard, true);
    return () => {
      for (const event of events) document.removeEventListener(event, guard, true);
      document.removeEventListener("compositionstart", startComposition, true);
      document.removeEventListener("compositionend", endComposition, true);
      setComposing(false);
    };
  }, []);
  return <div className="sync-shell">
    <ReceivedSyncControls />
    <DeviceSync />
    <div className="sync-content" inert={locked} aria-busy={locked}>{children}</div>
  </div>;
}

function snapshots(doc: OpenDoc | null): ipc.BufferSnapshot[] {
  // ADR-030: inactive tabs retain positions, not buffers. Split is a preview of
  // this same document. A future multi-buffer editor must extend this inventory.
  return doc ? [{ note_id: doc.noteId, base_rev: doc.baseRev, buffer_version: doc.bufferVersion, saved_version: doc.savedVersion }] : [];
}
export function ReceivedSyncControls() {
  const info = useWorkspace(s => s.info);
  const locked = useSyncExternalStore(subscribeBarrier, isSyncLocked);
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [recovery, setRecovery] = useState(false);
  const [working, setWorking] = useState(false);
  const frozen = useRef<OpenDoc | null>(null);
  const connected = !!info && workspace === info.id;

  async function openReceived() {
    if (info || locked || working) return;
    if (document.querySelector('[role="dialog"], [role="alertdialog"]')) { setMessage(t("received.busy")); return; }
    setWorking(true);
    let admitted = false;
    try {
      const picked = await openDialog({ directory: true, multiple: false, title: t("received.pick") });
      if (typeof picked !== "string") return;
      admitted = beginSyncBarrier();
      if (!admitted) { setMessage(t("received.busy")); return; }
      const opened = await ipc.syncOpen(picked);
      setWorkspace(opened.id);
      endSyncBarrier();
      await useWorkspace.getState().adopt(opened);
      setMessage(t("received.ready"));
    } catch (error) { setMessage(errorText(ipc.asCoreError(error))); }
    finally { if (admitted) endSyncBarrier(); setWorking(false); }
  }

  function finish(report: ipc.SyncApplyResult) {
    // Even an error can follow earlier writes or a lost receipt. Never release
    // the input barrier until the exact frozen clean document has been reloaded.
    if (report.reload_failed || !acceptSyncReload(frozen.current, report.refreshed)) {
      setRecovery(true);
      setMessage(t("received.recovery"));
      return;
    }
    setRecovery(false);
    setMessage(report.error ? errorText(report.error) : report.applied === null ? t("received.reloaded") : t("received.applied", { count: report.applied }));
    endSyncBarrier();
    void useWorkspace.getState().refresh(ipc.ROOT);
    void useSync.getState().start();
  }

  async function apply() {
    if (!connected || locked || working) return;
    if (document.querySelector('[role="dialog"], [role="alertdialog"]')) { setMessage(t("received.busy")); return; }
    const doc = useEditor.getState().doc;
    if (doc && (doc.bufferVersion !== doc.savedVersion || doc.status === "writing" || doc.conflict || doc.draft)) {
      setMessage(t("received.dirty")); return;
    }
    useSync.getState().stop();
    if (!beginSyncBarrier()) {
      setMessage(t("received.busy"));
      void useSync.getState().start();
      return;
    }
    frozen.current = doc;
    setWorking(true);
    setMessage(t("received.applying"));
    try { finish(await ipc.syncApply(snapshots(doc))); }
    catch {
      setRecovery(true);
      setMessage(t("received.recovery"));
    } finally { setWorking(false); }
  }
  async function recover() {
    if (!recovery || working) return;
    setWorking(true);
    try { finish(await ipc.syncReload(snapshots(frozen.current))); }
    catch { setMessage(t("received.recovery")); }
    finally { setWorking(false); }
  }

  return <section className="received-controls" data-sync-controls aria-label={t("received.title")}>
    <button disabled={!!info || locked || working} title={info ? t("received.closeFirst") : t("received.pick")} onClick={() => void openReceived()}>{t("received.open")}</button>
    {connected && <button disabled={locked || working} onClick={() => void apply()}>{t("received.apply")}</button>}
    {recovery && <button disabled={working} onClick={() => void recover()}>{t("received.retryReload")}</button>}
    <span role="status" aria-live="polite">{message}</span>
  </section>;
}
