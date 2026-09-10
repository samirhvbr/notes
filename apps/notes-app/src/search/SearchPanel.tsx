import { useCallback, useEffect, useRef, useState } from "react";
import { t } from "../i18n";
import * as ipc from "../ipc";
import type { SearchHit, SearchId, SearchMode } from "../ipc";
import { useEditor } from "../stores/editor";
import { useTabs } from "../stores/tabs";

/**
 * Global search: a panel, not a modal.
 *
 * Results are something you work through — open one, come back, open another —
 * so the panel stays while a note is open. It polls the core the same way
 * reconciliation does, because the core collects and the frontend asks.
 *
 * Two things it must say out loud, both from scope §10: **the search reads what
 * is on disk**, so an unsaved buffer has not been searched; and when the hit cap
 * is reached the list is the first N rather than all of them.
 */
export function SearchPanel({ onClose }: { onClose: () => void }) {
  const [query, setQuery] = useState("");
  const [mode, setMode] = useState<SearchMode>("literal");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [running, setRunning] = useState(false);
  const [truncated, setTruncated] = useState(false);
  const [partial,setPartial]=useState(false);
  const [scanned, setScanned] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const idRef = useRef<SearchId | null>(null);
  const generation = useRef(0);
  const input = useRef<HTMLInputElement | null>(null);
  const openAt = useTabs((s) => s.openAt);
  const doc = useEditor((s) => s.doc);
  const dirty = !!doc && doc.bufferVersion !== doc.savedVersion;

  useEffect(() => {
    const id = requestAnimationFrame(() => input.current?.focus());
    return () => cancelAnimationFrame(id);
  }, []);

  const stop = useCallback(() => {
    generation.current += 1;
    const id = idRef.current;
    idRef.current = null;
    setRunning(false);
    if (id !== null) void ipc.searchCancel(id).catch(() => {});
  }, []);

  // Cancel on unmount: a panel that is closed must not leave a walk running.
  useEffect(() => stop, [stop]);

  const start = useCallback(async () => {
    stop();
    setHits([]);
    setTruncated(false);
    setPartial(false);
    setScanned(0);
    setError(null);
    if (!query.trim()) return;

    const ticket=generation.current;
    setRunning(true);
    let id: SearchId;
    try {
      id = await ipc.searchStart(query, { mode, case_sensitive: mode === "words" ? false : caseSensitive });
    } catch (e) {
      if(ticket!==generation.current)return;
      setRunning(false);
      const err = ipc.asCoreError(e);
      setError(err.code === "invalid_path" ? t("search.badPattern") : t(`error.${err.code}`));
      return;
    }
    if(ticket!==generation.current){void ipc.searchCancel(id).catch(()=>{});return;}
    idRef.current = id;
    setRunning(true);

    // Polling rather than an event stream, following `reconcile_tick`. Fast
    // enough that the first result appears as soon as the core has it — it
    // arrives in about 11 ms on a 10 000-note workspace.
    const poll = async () => {
      if (idRef.current !== id) return;
      try {
        const p = await ipc.searchPoll(id);
        if (idRef.current !== id) return;
        if (p.hits.length) setHits((h) => h.concat(p.hits));
        setScanned(p.files_scanned);
        setTruncated(p.truncated);
        setPartial(p.partial);
        if (p.done) {
          idRef.current = null;
          setRunning(false);
          return;
        }
      } catch (e) {
        if(idRef.current!==id)return;
        setError(t(`error.${ipc.asCoreError(e).code}`));
        idRef.current = null;
        setRunning(false);
        return;
      }
      setTimeout(() => void poll(), 30);
    };
    void poll();
  }, [query, mode, caseSensitive, stop]);

  return (
    <aside className="search-panel" aria-label={t("search.title")}>
      <header>
        <strong>{t("search.title")}</strong>
        <span className="spacer" />
        <button onClick={onClose} aria-label={t("search.close")}>
          ×
        </button>
      </header>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          void start();
        }}
      >
        <input
          ref={input}
          value={query}
          spellCheck={false}
          autoComplete="off"
          placeholder={t("search.placeholder")}
          aria-label={t("search.title")}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
          }}
        />
        <div className="search-options">
          {/* The three semantics keep their names and do not swap underneath a
              running query (scope §10). `words` arrives with FTS5 at 0.2. */}
          <label>
            <span className="muted">{t("search.mode")}</span>
            <select value={mode} onChange={(e) => setMode(e.target.value as SearchMode)}>
              <option value="literal">{t("search.mode.literal")}</option>
              <option value="words">{t("search.mode.words")}</option>
              <option value="regex">{t("search.mode.regex")}</option>
            </select>
          </label>
          <label>
            <input
              type="checkbox"
              disabled={mode === "words"}
              checked={mode === "words" ? false : caseSensitive}
              onChange={(e) => setCaseSensitive(e.target.checked)}
            />
            {t("search.caseSensitive")}
          </label>
          {running ? (
            <button type="button" onClick={stop}>
              {t("search.cancel")}
            </button>
          ) : (
            <button type="submit" className="primary">
              {t("search.run")}
            </button>
          )}
        </div>
      </form>

      {partial && <p className="muted">{t("search.partial")}</p>}
      {error && <p className="bad">{error}</p>}

      <p className="muted note">
        {t("search.readsDisk")}
        {dirty && <strong> {t("search.unsavedWarning")}</strong>}
      </p>

      <p className="muted count">
        {running
          ? t("search.scanning", { files: scanned, hits: hits.length })
          : t("search.found", { hits: hits.length, files: scanned })}
        {truncated && ` · ${t("search.truncated", { hits: hits.length })}`}
      </p>

      <ul className="hits">
        {hits.map((h, i) => (
          <li key={`${h.path}:${h.line}:${h.col}:${i}`}>
            <button
              className="row"
              onClick={() => void openAt(h.path, h.line, h.col)}
              title={`${h.path}:${h.line}`}
            >
              <span className="where">
                {h.path}
                <span className="muted">:{h.line}</span>
              </span>
              <span className="context">{h.context}</span>
            </button>
          </li>
        ))}
      </ul>
    </aside>
  );
}
