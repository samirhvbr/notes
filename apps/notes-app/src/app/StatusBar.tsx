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
 * One short phrase for the watcher's coverage, or nothing when it is complete.
 *
 * A partly watched workspace is not a failure and is not a success: the
 * unwatched part still reaches the application through the five-second scan,
 * a few seconds later than the rest. Both numbers are named, because "degraded"
 * without a count is a word the user can do nothing with.
 */
function coverage(w: WatchStatus): string | null {
  if (w.walking) return t("watch.walking", { dirs: w.dirs });
  if (w.over_limit > 0) return t("watch.overLimit", { count: w.over_limit });
  if (w.unreadable > 0) return t("watch.unreadable", { count: w.unreadable });
  return null;
}

/** `code` is the contract; the message is never read from the backend. */
export function errorText(e: CoreError): string {
  if (e.code === "io") return t(`error.io.${e.kind}`);
  if (e.code === "dirty_buffers") return t("error.dirty_buffers", { count: e.count });
  return t(`error.${e.code}`);
}
