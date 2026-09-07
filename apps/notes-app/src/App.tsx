import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Editor } from "./Editor";
import {
  openWorkspace,
  readNote,
  restoreWorkspace,
  spikeEnv,
  writeNote,
  type NoteEntry,
  type SpikeEnv,
  type WorkspaceInfo,
} from "./api";

type SaveState = "idle" | "dirty" | "writing" | "saved" | "error";

/**
 * Spike 0.0 shell.
 *
 * This is not the 0.1a application and does not pretend to be: there is no
 * BaseRev, no conflict detection, no watcher, no draft recovery. Its only job is
 * to make the four 0.0 acceptance criteria observable on a real device, which is
 * why the diagnostics panel is as prominent as the editor.
 */
export default function App() {
  const [env, setEnv] = useState<SpikeEnv | null>(null);
  const [ws, setWs] = useState<WorkspaceInfo | null>(null);
  const [restoreError, setRestoreError] = useState<string | null>(null);
  const [active, setActive] = useState<NoteEntry | null>(null);
  const [doc, setDoc] = useState("");
  const [save, setSave] = useState<SaveState>("idle");
  const [error, setError] = useState<string | null>(null);
  const buffer = useRef("");

  useEffect(() => {
    spikeEnv().then(setEnv).catch((e) => setError(String(e)));
    // Acceptance criterion 3 is exactly this call succeeding after the process
    // was killed, so a failure is reported rather than treated as "no workspace".
    restoreWorkspace()
      .then((w) => w && setWs(w))
      .catch((e) => setRestoreError(String(e)));
  }, []);

  const pick = useCallback(async () => {
    setError(null);
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    try {
      setWs(await openWorkspace(picked));
      setActive(null);
      setDoc("");
      setRestoreError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const openNote = useCallback(async (entry: NoteEntry) => {
    setError(null);
    try {
      const text = await readNote(entry.relPath);
      buffer.current = text;
      setDoc(text);
      setActive(entry);
      setSave("idle");
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const persist = useCallback(async () => {
    if (!active) return;
    setSave("writing");
    try {
      await writeNote(active.relPath, buffer.current);
      setSave("saved");
    } catch (e) {
      setSave("error");
      setError(String(e));
    }
  }, [active]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void persist();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [persist]);

  return (
    <div className="app">
      <header className="bar">
        <strong>notes</strong>
        <span className="tag">spike 0.0 — not the product</span>
        <button onClick={pick}>Open Folder…</button>
        {active && (
          <>
            <button onClick={persist} disabled={save === "writing"}>
              Save (Ctrl+S)
            </button>
            <span className={`state state-${save}`}>{save}</span>
          </>
        )}
      </header>

      <div className="body">
        <aside className="side">
          <Diagnostics env={env} ws={ws} restoreError={restoreError} />
          <h2>Notes</h2>
          {!ws && <p className="muted">No workspace open.</p>}
          {ws && ws.entries.length === 0 && (
            <p className="muted">No .md files at the root of this folder.</p>
          )}
          <ul className="files">
            {ws?.entries.map((e) => (
              <li key={e.relPath}>
                <button
                  className={active?.relPath === e.relPath ? "on" : ""}
                  onClick={() => openNote(e)}
                >
                  {e.name}
                </button>
              </li>
            ))}
          </ul>
        </aside>

        <main className="main">
          {active ? (
            <Editor
              docKey={active.relPath}
              value={doc}
              onChange={(next) => {
                buffer.current = next;
                setSave("dirty");
              }}
            />
          ) : (
            <div className="empty">
              <p>Pick a folder, then open a note.</p>
              <p className="muted">
                Type <code>ação</code>, a dead-key <code>ç</code> and an emoji.
                Select, paste, undo. That is acceptance criterion 2.
              </p>
            </div>
          )}
        </main>
      </div>

      {error && <div className="err">{error}</div>}
    </div>
  );
}

function Diagnostics({
  env,
  ws,
  restoreError,
}: {
  env: SpikeEnv | null;
  ws: WorkspaceInfo | null;
  restoreError: string | null;
}) {
  if (!env) return <section className="diag muted">reading environment…</section>;
  return (
    <section className="diag">
      <h2>Diagnostics</h2>
      <dl>
        <dt>platform</dt>
        <dd>
          {env.os}/{env.arch} · tauri {env.tauriVersion}
        </dd>
        <dt>session</dt>
        <dd>
          {env.session}
          {env.nvidia ? " · nvidia" : ""}
        </dd>
        <dt>dmabuf</dt>
        <dd>
          <strong>{env.dmabufApplied ? "APPLIED" : "not applied"}</strong> —{" "}
          {env.dmabufWorkaround}
        </dd>
        <dt>app data</dt>
        <dd className="wrap">{env.appDataDir}</dd>
        <dt>workspace</dt>
        <dd className="wrap">
          {ws ? (
            <>
              {ws.root}
              <br />
              <em>{ws.restored ? "restored from disk" : "picked this run"}</em>
            </>
          ) : restoreError ? (
            <span className="bad">restore failed: {restoreError}</span>
          ) : (
            "none"
          )}
        </dd>
      </dl>
    </section>
  );
}
