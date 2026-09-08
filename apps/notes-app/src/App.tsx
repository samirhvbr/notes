import { useCallback, useEffect, useState } from "react";
import { Editor } from "./editor/Editor";
import { Tree } from "./explorer/Tree";
import { StatusBar, errorText } from "./app/StatusBar";
import { Welcome } from "./app/Welcome";
import { Dialog } from "./app/Dialog";
import { Tabs } from "./app/Tabs";
import { Palette, type Command, type PaletteMode } from "./app/Palette";
import { SettingsPanel } from "./app/Settings";
import { SearchPanel } from "./search/SearchPanel";
import { useTabs } from "./stores/tabs";
import { useSettings } from "./stores/settings";
import { askText } from "./app/dialog";
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
  const [palette, setPalette] = useState<PaletteMode | null>(null);
  const [searching, setSearching] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const applySettings = useSettings((s) => s.apply);
  const loadSettings = useSettings((s) => s.load);
  const restoreTabs = useTabs((s) => s.restore);
  const closeActiveTab = useTabs((s) => s.closeActive);
  const resetTabs = useTabs((s) => s.reset);

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

  // Settings before the editor mounts, so it is not built once with defaults
  // and rebuilt a frame later with the real font size.
  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  // Tabs belong to a workspace: they are restored when one opens and dropped
  // when it changes. This is half of the 0.1c criterion — the other half is the
  // cursor, which `stores/tabs.ts` hands to the editor after it mounts.
  useEffect(() => {
    if (!info) {
      resetTabs();
      return;
    }
    void restoreTabs();
  }, [info, restoreTabs, resetTabs]);

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
      } else if (key === "p" && !e.shiftKey) {
        e.preventDefault();
        setPalette("files");
      } else if (key === "p" && e.shiftKey) {
        e.preventDefault();
        setPalette("commands");
      } else if (key === "f" && e.shiftKey) {
        // `Ctrl+F` belongs to CodeMirror's in-file panel (0.1b); the workspace
        // search is the shifted one, exactly as scope §34 lists them.
        e.preventDefault();
        setSearching(true);
      } else if (key === "w") {
        e.preventDefault();
        void closeActiveTab();
      } else if (key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [save, cycleView, closeActiveTab]);

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
    const name = await askText({
      title: t("tree.newNote"),
      label: t("dialog.name"),
      confirmLabel: t("dialog.create"),
      validate: (v) => (v.trim() ? null : t("dialog.nameRequired")),
    });
    if (!name) return;
    try {
      const e = await ipc.noteCreate(ipc.ROOT, name);
      await refresh(ipc.ROOT);
      await useTabs.getState().openPath(e.path);
    } catch (err) {
      fail(err);
    }
  }, [refresh, fail]);

  const newFolder = useCallback(async () => {
    const name = await askText({
      title: t("tree.newFolder"),
      label: t("dialog.name"),
      confirmLabel: t("dialog.create"),
      validate: (v) => (v.trim() ? null : t("dialog.nameRequired")),
    });
    if (!name) return;
    try {
      await ipc.dirCreate(ipc.ROOT, name);
      await refresh(ipc.ROOT);
    } catch (err) {
      fail(err);
    }
  }, [refresh, fail]);

  const commands: Command[] = [
    { id: "quick-open", label: "command.quickOpen", hint: "Ctrl+P", run: () => setPalette("files") },
    { id: "search", label: "command.searchWorkspace", hint: "Ctrl+Shift+F", run: () => setSearching(true) },
    { id: "new-note", label: "command.newNote", hint: "Ctrl+N", run: newNote },
    { id: "new-folder", label: "command.newFolder", run: newFolder },
    { id: "close-tab", label: "command.closeTab", hint: "Ctrl+W", run: () => void closeActiveTab() },
    { id: "cycle-view", label: "command.cycleView", hint: "Ctrl+E", run: cycleView },
    { id: "save", label: "command.save", hint: "Ctrl+S", run: () => void save(true) },
    { id: "settings", label: "command.settings", hint: "Ctrl+,", run: () => setSettingsOpen(true) },
  ];

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
        {searching && <SearchPanel onClose={() => setSearching(false)} />}
        <main className="main">
          <Tabs />
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

      <Dialog />
      {palette && (
        <Palette mode={palette} commands={commands} onClose={() => setPalette(null)} />
      )}
      {settingsOpen && (
        <SettingsPanel onClose={() => setSettingsOpen(false)} onChanged={applySettings} />
      )}
      <StatusBar />
    </div>
  );
}
