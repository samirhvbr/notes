import { t } from "../i18n";
import { useEditor } from "../stores/editor";
import { useSync } from "../stores/sync";
import { useWorkspace } from "../stores/workspace";
import type { CoreError, DocStatus, WatchStatus } from "../ipc";

/**
 * The seven states of scope §9.
 *
 * `saved` appears only after the backend confirms — the store sets it from a
 * `SaveResult`, never optimistically. State is never carried by colour alone:
 * each has its own word and its own glyph.
 */
const GLYPH: Record<DocStatus, string> = {
  saved: "✓",
  pending: "●",
  writing: "◌",
  conflict: "⚠",
  read_only: "🔒",
  error: "✕",
  unavailable: "⊘",
};

export function StatusBar() {
  const doc = useEditor((s) => s.doc);
  const info = useWorkspace((s) => s.info);
  const wsError = useWorkspace((s) => s.error);
  const watch = useSync((s) => s.watch);
  const watchText = watch ? coverage(watch) : null;

  const status: DocStatus = wsError?.code === "unavailable"
    ? "unavailable"
    : doc?.status ?? "saved";

  const message = doc?.lastError
    ? errorText(doc.lastError)
    : wsError
      ? errorText(wsError)
      : doc?.readOnly
        ? t(`readonly.${doc.readOnly}`)
        : null;

  return (
    <footer className="statusbar">
      <span className={`status status-${status}`}>
        <span aria-hidden="true">{GLYPH[status]}</span> {t(`status.${status}`)}
      </span>
      {doc && <span className="muted path">{doc.path}</span>}
      {message && <span className="message">{message}</span>}
      <span className="spacer" />
      {/* Background work, while there is any. Opening a workspace returns as
          soon as the tree can be drawn; the watcher's walk carries on behind
          it, so the bar says so instead of leaving the user to guess whether
          an unwatched folder is a bug (docs/DECISIONS-0.1c.md D-09). */}
      {watchText && <span className="muted watch">{watchText}</span>}
      {info && <span className="muted root" title={info.root}>{info.display_name}</span>}
    </footer>
  );
}

/**
 * The watcher's coverage **while it is still filling**, and nothing once it is.
 *
 * This is the transient half: opening a workspace returns as soon as the tree
 * can be drawn, and the walk that installs the watches carries on behind it
 * (ADR-034), so the bar says so rather than leaving the user to wonder whether
 * an unwatched folder is a bug. The two states that *settle* — a full watch
 * table, a folder that cannot be read — are banners in `App.tsx`, because they
 * need a number and a sentence rather than a corner of the status bar.
 */
function coverage(w: WatchStatus): string | null {
  return w.walking ? t("watch.walking", { dirs: w.dirs }) : null;
}

/** `code` is the contract; the message is never read from the backend. */
export function errorText(e: CoreError): string {
  if (e.code === "io") return t(`error.io.${e.kind}`);
  if (e.code === "dirty_buffers") return t("error.dirty_buffers", { count: e.count });
  return t(`error.${e.code}`);
}
