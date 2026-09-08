import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { t } from "../i18n";
import * as ipc from "../ipc";
import type { QuickMatch, RelPath } from "../ipc";
import { useTabs } from "../stores/tabs";

/**
 * One surface, two lists.
 *
 * Quick open (`Ctrl+P`) and the command palette (`Ctrl+Shift+P`) are the same
 * interaction — filter, arrow, `Enter` — over different rows, so they are one
 * component. They are **not** the same as global search: that one reads files,
 * streams and can be cancelled, and it lives in a panel rather than a modal
 * because its results are something you work through rather than pick from.
 */
export type PaletteMode = "files" | "commands";

export interface Command {
  id: string;
  /** An i18n key, never a sentence. */
  label: string;
  hint?: string;
  run: () => void | Promise<void>;
}

interface Row {
  key: string;
  primary: string;
  secondary?: string;
  hint?: string;
  activate: () => void | Promise<void>;
}

export function Palette({
  mode,
  commands,
  onClose,
}: {
  mode: PaletteMode;
  commands: Command[];
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [files, setFiles] = useState<QuickMatch[]>([]);
  const [cursor, setCursor] = useState(0);
  const input = useRef<HTMLInputElement | null>(null);
  const openPath = useTabs((s) => s.openPath);

  useEffect(() => {
    setQuery("");
    setCursor(0);
    const id = requestAnimationFrame(() => input.current?.focus());
    return () => cancelAnimationFrame(id);
  }, [mode]);

  // Quick open asks the core on every keystroke, which is affordable precisely
  // because it matches a cached path list and never opens a file.
  useEffect(() => {
    if (mode !== "files") return;
    let live = true;
    ipc
      .quickOpen(query, 50)
      .then((r) => live && setFiles(r))
      .catch(() => live && setFiles([]));
    return () => {
      live = false;
    };
  }, [mode, query]);

  const rows: Row[] = useMemo(() => {
    if (mode === "files") {
      return files.map((m) => ({
        key: m.path,
        primary: m.name,
        secondary: parentOf(m.path),
        activate: () => openPath(m.path),
      }));
    }
    const q = query.trim().toLowerCase();
    return commands
      .filter((c) => !q || t(c.label).toLowerCase().includes(q))
      .map((c) => ({ key: c.id, primary: t(c.label), hint: c.hint, activate: c.run }));
  }, [mode, files, commands, query, openPath]);

  useEffect(() => {
    setCursor((c) => Math.min(c, Math.max(rows.length - 1, 0)));
  }, [rows.length]);

  const choose = useCallback(
    async (row: Row | undefined) => {
      if (!row) return;
      onClose();
      await row.activate();
    },
    [onClose],
  );

  return (
    <div
      className="overlay palette-overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="palette"
        role="dialog"
        aria-modal="true"
        aria-label={t(mode === "files" ? "palette.files" : "palette.commands")}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onClose();
          } else if (e.key === "ArrowDown") {
            e.preventDefault();
            setCursor((c) => (rows.length ? (c + 1) % rows.length : 0));
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setCursor((c) => (rows.length ? (c - 1 + rows.length) % rows.length : 0));
          } else if (e.key === "Enter") {
            e.preventDefault();
            void choose(rows[cursor]);
          }
        }}
      >
        <input
          ref={input}
          value={query}
          spellCheck={false}
          autoComplete="off"
          placeholder={t(mode === "files" ? "palette.filesPlaceholder" : "palette.commandsPlaceholder")}
          aria-label={t(mode === "files" ? "palette.files" : "palette.commands")}
          onChange={(e) => {
            setQuery(e.target.value);
            setCursor(0);
          }}
        />

        {rows.length === 0 ? (
          <p className="muted empty-row">{t("palette.noMatch")}</p>
        ) : (
          <ul className="palette-rows" role="listbox">
            {rows.map((row, i) => (
              <li key={row.key}>
                <button
                  role="option"
                  aria-selected={i === cursor}
                  className={i === cursor ? "row on" : "row"}
                  onMouseMove={() => setCursor(i)}
                  onClick={() => void choose(row)}
                >
                  <span className="primary">{row.primary}</span>
                  {row.secondary && <span className="muted secondary">{row.secondary}</span>}
                  {row.hint && <span className="muted hint">{row.hint}</span>}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function parentOf(path: RelPath): string {
  const i = path.lastIndexOf("/");
  return i < 0 ? "" : path.slice(0, i);
}
