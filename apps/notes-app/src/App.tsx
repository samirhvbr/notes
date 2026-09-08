import { useCallback, useEffect, useState } from "react";
import { Editor } from "./editor/Editor";
import { Tree } from "./explorer/Tree";
import { StatusBar, errorText } from "./app/StatusBar";
import { Welcome } from "./app/Welcome";
import { Preview } from "./preview/Preview";
import { Compare } from "./conflict/Compare";
import { t } from "./i18n";
import * as ipc from "./ipc";
import { useEditor } from "./stores/editor";
import { useSync } from "./stores/sync";
import { useUi, type ViewMode } from "./stores/ui";
import { useWorkspace } from "./stores/workspace";

const VIEWS: ViewMode[] = ["source", "preview", "split"];

export default function App() {
  const info = useWorkspace((s) => s.info);
  const restore = useWorkspace((s) => s.restore);
  const refresh = useWorkspace((s) => s.refresh);
  const fail = useWorkspace((s) => s.fail);
  const wsError = useWorkspace((s) => s.error);
  const notice = useWorkspace((s) => s.notice);
  const clearNote = useWorkspace((s) => s.clearNote);
  const doc = useEditor((s) => s.doc);
  const save = useEditor((s) => s.save);
  const keepDraft = useEditor((s) => s.keepDraft);
  const resolveDraft = useEditor((s) => s.resolveDraft);
  const convertEol = useEditor((s) => s.convertEol);
  const setAutosave = useEditor((s) => s.setAutosave);
  const view = useUi((s) => s.view);
  const setView = useUi((s) => s.setView);
  const cycleView = useUi((s) => s.cycleView);
  const comparing = useUi((s) => s.comparing);
  const setComparing = useUi((s) => s.setComparing);
  const hydrateUi = useUi((s) => s.hydrate);
  const startSync = useSync((s) => s.start);
  const stopSync = useSync((s) => s.stop);
  const degraded = useSync((s) => s.degraded);
  const [env, setEnv] = useState<ipc.EnvReport | null>(null);

  useEffect(() => {
    void restore();
    ipc.settingsGet().then((s) => setAutosave(s.files.autosave_ms)).catch(() => {});
    ipc.envReport().then(setEnv).catch(() => {});
  }, [restore, setAutosave]);

  // The session is per workspace, so the view mode is only readable once one is
  // open.
  useEffect(() => {
    if (info) void hydrateUi();
  }, [info, hydrateUi]);

  // The watcher and the reconciliation clocks belong to a workspace, and stop
  // with it.
  useEffect(() => {
    if (!info) return;
    void startSync();
    return () => stopSync();
  }, [info, startSync, stopSync]);

  // Ctrl/Cmd+S forces a flush; the app never depends on it to save.
  // Ctrl/Cmd+E cycles Source → Preview → Split (scope §9).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const key = e.key.toLowerCase();
      if (key === "s") {
        e.preventDefault();
        void save(true);
      } else if (key === "e") {
        e.preventDefault();
        cycleView();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [save, cycleView]);

  // Closing with a dirty buffer keeps it: the draft is written on the way out.
  useEffect(() => {
    const onLeave = () => {
      const d = useEditor.getState().doc;
      if (d && d.bufferVersion !== d.savedVersion) void keepDraft("exit");
    };
    window.addEventListener("beforeunload", onLeave);
    return () => window.removeEventListener("beforeunload", onLeave);
  }, [keepDraft]);

  // A note that leaves conflict has nothing left to compare.
  useEffect(() => {
    if (!doc?.conflict && comparing) setComparing(false);
  }, [doc?.conflict, comparing, setComparing]);

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

        <div className="views" role="group" aria-label={t("view.group")}>
          {VIEWS.map((v) => (
            <button
              key={v}
              className={view === v ? "on" : undefined}
              aria-pressed={view === v}
              onClick={() => setView(v)}
              title={t("view.hint")}
            >
              {t(`view.${v}`)}
            </button>
          ))}
        </div>

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
          {doc?.conflict && !comparing && (
            <div className="banner warn">
              <strong>{t("conflict.title", { name: doc.path })}</strong>
              <span>{t("conflict.body")}</span>
              <button onClick={() => setComparing(true)}>{t("conflict.compare")}</button>
            </div>
          )}
          {/* A mixed-EOL note opens read-only; conversion is the way forward,
              and it keeps the old bytes in `conflicts/`. */}
          {doc?.readOnly === "mixed_eol" && (
            <div className="banner warn">
              <span>{t("readonly.mixed_eol")}</span>
              <button onClick={() => convertEol("Lf").catch(fail)}>{t("eol.toLf")}</button>
              <button onClick={() => convertEol("CrLf").catch(fail)}>{t("eol.toCrLf")}</button>
            </div>
          )}
          {/* Not being able to watch is a state of the workspace, not a
              failure: the app polls instead and says why, and the inotify limit
              arrives with the sysctl that raises it (ARCHITECTURE.md §8). */}
          {degraded && (
            <div className="banner">
              <span>{t("watch.degraded", { reason: degraded })}</span>
            </div>
          )}
          {wsError && <div className="banner warn">{errorText(wsError)}</div>}
          {/* Something went right and the user has to be told which of two
              things it was — a delete that can be undone is not the same event
              as one that cannot (scope §7.7). */}
          {notice && (
            <div className="banner">
              <span>{notice}</span>
              <button onClick={clearNote}>{t("notice.dismiss")}</button>
            </div>
          )}

          {comparing && doc?.conflict ? (
            <Compare />
          ) : (
            <div className={`panes pane-${view}`}>
              {/* With no note open there is nothing to preview, so the editor's
                  own empty state is what the pane shows — a blank Preview pane
                  would say less than "open a note from the sidebar". */}
              {(view !== "preview" || !doc) && <Editor />}
              {view !== "source" && doc && <Preview />}
            </div>
          )}
        </main>
      </div>

      <StatusBar />
    </div>
  );
}
