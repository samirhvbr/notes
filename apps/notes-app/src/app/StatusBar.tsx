import { t } from "../i18n";
import { useEditor } from "../stores/editor";
import { useWorkspace } from "../stores/workspace";
import type { CoreError, DocStatus } from "../ipc";

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
      {info && <span className="muted root" title={info.root}>{info.display_name}</span>}
    </footer>
  );
}

/** `code` is the contract; the message is never read from the backend. */
export function errorText(e: CoreError): string {
  if (e.code === "io") return t(`error.io.${e.kind}`);
  if (e.code === "dirty_buffers") return t("error.dirty_buffers", { count: e.count });
  return t(`error.${e.code}`);
}
