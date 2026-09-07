import { useCallback, useEffect, useState } from "react";
import { Editor } from "./editor/Editor";
import { Tree } from "./explorer/Tree";
import { StatusBar, errorText } from "./app/StatusBar";
import { Welcome } from "./app/Welcome";
import { t } from "./i18n";
import * as ipc from "./ipc";
import { useEditor } from "./stores/editor";
import { useWorkspace } from "./stores/workspace";

export default function App() {
  const info = useWorkspace((s) => s.info);
  const restore = useWorkspace((s) => s.restore);
  const refresh = useWorkspace((s) => s.refresh);
  const fail = useWorkspace((s) => s.fail);
  const doc = useEditor((s) => s.doc);
  const save = useEditor((s) => s.save);
  const keepDraft = useEditor((s) => s.keepDraft);
  const resolveDraft = useEditor((s) => s.resolveDraft);
  const setAutosave = useEditor((s) => s.setAutosave);
  const [env, setEnv] = useState<ipc.EnvReport | null>(null);

  useEffect(() => {
    void restore();
    ipc.settingsGet().then((s) => setAutosave(s.files.autosave_ms)).catch(() => {});
    ipc.envReport().then(setEnv).catch(() => {});
  }, [restore, setAutosave]);

  // Ctrl/Cmd+S forces a flush; the app never depends on it to save.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void save(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [save]);

  // Closing with a dirty buffer keeps it: the draft is written on the way out.
  useEffect(() => {
    const onLeave = () => {
      const d = useEditor.getState().doc;
      if (d && d.bufferVersion !== d.savedVersion) void keepDraft("exit");
    };
    window.addEventListener("beforeunload", onLeave);
    return () => window.removeEventListener("beforeunload", onLeave);
  }, [keepDraft]);

  const newNote = useCallback(async () => {
    const name = window.prompt(t("tree.newNote"));
    if (!name) return;
    try {
      const e = await ipc.noteCreate(ipc.ROOT, name);
      await refresh(ipc.ROOT);
      await useEditor.getState().open(e.path);
    } catch (err) {
      fail(err);
    }
  }, [refresh, fail]);

  const newFolder = useCallback(async () => {
    const name = window.prompt(t("tree.newFolder"));
    if (!name) return;
    try {
      await ipc.dirCreate(ipc.ROOT, name);
      await refresh(ipc.ROOT);
    } catch (err) {
      fail(err);
    }
  }, [refresh, fail]);

  if (!info) return <Welcome />;

  return (
    <div className="app">
      <header className="toolbar">
        <strong>notes</strong>
        <button onClick={newNote}>{t("tree.newNote")}</button>
        <button onClick={newFolder}>{t("tree.newFolder")}</button>
        <span className="spacer" />
        {env && (
          <span className="muted diag" title={env.dmabufExplanation}>
            {env.os}/{env.session}
            {env.dmabufApplied ? " · dmabuf off" : ""}
          </span>
        )}
      </header>

      <div className="body">
        <aside className="side">
          <Tree />
        </aside>
        <main className="main">
          {doc?.draft && (
            <div className="banner">
              <span>{t("draft.found", { name: doc.path })}</span>
              <button onClick={() => resolveDraft(true)}>{t("draft.restore")}</button>
              <button onClick={() => resolveDraft(false)}>{t("draft.discard")}</button>
            </div>
          )}
          {doc?.conflict && (
            <div className="banner warn">
              <strong>{t("conflict.title", { name: doc.path })}</strong>
              <span>{t("conflict.body")}</span>
            </div>
          )}
          {useWorkspace.getState().error && (
            <div className="banner warn">{errorText(useWorkspace.getState().error!)}</div>
          )}
          <Editor />
        </main>
      </div>

      <StatusBar />
    </div>
  );
}
