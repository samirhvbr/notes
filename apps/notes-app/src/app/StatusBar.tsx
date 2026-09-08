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

  const counts = doc ? measure(doc.text) : null;

  return (
    <footer className="statusbar">
      {/* Background work on the left, discreet, and gone when it finishes
          (`.continue/0.1d-interface.md` §4.4). */}
      {watchText && <span className="muted watch">{watchText}</span>}
      {message && <span className="message">{message}</span>}
      <span className="spacer" />
      {/* And the note's own facts on the right: the seven states of scope §9,
          words, characters. **Nothing else.** A backlinks counter would need
          the index that arrives at 0.3, and a counter with no data behind it is
          a lie with the face of a feature (§3). */}
      {info && (
        <span className="muted root" title={info.root}>
          {info.display_name}
        </span>
      )}
      {counts && (
        <>
          <span className="muted count">{t("status.words", { count: counts.words })}</span>
          <span className="muted count">{t("status.chars", { count: counts.chars })}</span>
        </>
      )}
      <span className={`status status-${status}`}>
        <span aria-hidden="true">{GLYPH[status]}</span> {t(`status.${status}`)}
      </span>
    </footer>
  );
}

/**
 * Words and characters of the buffer on screen.
 *
 * Characters are **code points**, not UTF-16 units, so `\u{1F331}` counts once
 * rather than twice — the editor's own `length` would say two, and a person
 * counting a character does not mean a surrogate pair. Words are runs of
 * non-whitespace, which is the definition that does not need a dictionary and
 * does not surprise anyone in either catalogue's language.
 *
 * Markdown syntax is counted: `# Title` is two words. Stripping it would mean
 * parsing on every keystroke to produce a number nobody is auditing, and the
 * count would then disagree with what the editor visibly contains.
 */
function measure(text: string): { words: number; chars: number } {
  const trimmed = text.trim();
  return {
    words: trimmed ? trimmed.split(/\s+/).length : 0,
    chars: [...text].length,
  };
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
