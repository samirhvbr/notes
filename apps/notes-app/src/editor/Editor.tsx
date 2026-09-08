import { useEffect, useRef } from "react";
import { Annotation, EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { useEditor } from "../stores/editor";
import { t } from "../i18n";

/**
 * CodeMirror 6.
 *
 * `lineSeparator: "\n"` is not a preference: the profile detected on open — not
 * the editor — decides what reaches the disk, so the editor works in `\n` and
 * the core re-applies the file's own endings and BOM on save
 * (docs/ARCHITECTURE.md §5.1).
 */
/** Marks a transaction the *application* made, not the user. */
const External = Annotation.define<boolean>();

export function Editor() {
  const doc = useEditor((s) => s.doc);
  const edit = useEditor((s) => s.edit);
  const host = useRef<HTMLDivElement | null>(null);
  const view = useRef<EditorView | null>(null);
  const editRef = useRef(edit);
  editRef.current = edit;

  const key = doc ? `${doc.noteId}:${doc.savedVersion}` : null;
  const initial = useRef(doc?.text ?? "");
  const readOnly = !!doc?.readOnly;
  if (doc && key !== null) initial.current = doc.text;

  useEffect(() => {
    if (!host.current || key === null) return;
    const state = EditorState.create({
      doc: initial.current,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        markdown({ codeLanguages: languages }),
        EditorView.lineWrapping,
        EditorState.readOnly.of(readOnly),
        EditorState.lineSeparator.of("\n"),
        EditorView.updateListener.of((u) => {
          if (!u.docChanged) return;
          // A reload from disk is not a keystroke: marking the buffer dirty
          // here would start an autosave of text the user never typed.
          if (u.transactions.some((tr) => tr.annotation(External))) return;
          editRef.current(u.state.doc.toString());
        }),
        theme,
      ],
    });
    const created = new EditorView({ state, parent: host.current });
    view.current = created;
    return () => {
      created.destroy();
      if (view.current === created) view.current = null;
    };
    // Keyed by the document, not its text: rebuilding on every keystroke would
    // destroy the undo history and the IME composition.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key === null ? null : key.split(":")[0], readOnly]);

  return <EditorBody doc={doc} host={host} view={view} externalRev={doc?.externalRev ?? 0} />;
}

/**
 * The half that has to react to a reload from disk.
 *
 * Scope §12: *"disco mudou, buffer limpo → recarrega, preserva cursor."* The
 * document is replaced with **one transaction** rather than by rebuilding the
 * view — rebuilding would throw away the undo history and the IME composition,
 * and put the caret at the top of a note the user was reading half-way down.
 * The selection is clamped, because the new text may be shorter.
 */
function EditorBody({
  doc,
  host,
  view,
  externalRev,
}: {
  doc: ReturnType<typeof useEditor.getState>["doc"];
  host: React.MutableRefObject<HTMLDivElement | null>;
  view: React.MutableRefObject<EditorView | null>;
  externalRev: number;
}) {
  const applied = useRef(externalRev);
  useEffect(() => {
    if (applied.current === externalRev) return;
    applied.current = externalRev;
    const v = view.current;
    if (!v || !doc) return;
    if (v.state.doc.toString() === doc.text) return;
    const at = Math.min(v.state.selection.main.head, doc.text.length);
    v.dispatch({
      changes: { from: 0, to: v.state.doc.length, insert: doc.text },
      selection: { anchor: at },
      // Not an edit by the user: the update listener checks this flag so the
      // reload does not mark the buffer dirty and start an autosave.
      annotations: External.of(true),
    });
  }, [externalRev, doc, view]);

  if (!doc) {
    return (
      <div className="empty">
        <p className="muted">{t("editor.pickANote")}</p>
      </div>
    );
  }
  return <div className="editor" ref={host} />;
}

const theme = EditorView.theme(
  {
    "&": { height: "100%", fontSize: "14px" },
    ".cm-scroller": {
      fontFamily: "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      lineHeight: "1.65",
    },
    ".cm-content": { caretColor: "#e6e6e6", padding: "12px 0" },
    "&.cm-focused .cm-cursor": { borderLeftColor: "#e6e6e6" },
    ".cm-gutters": { backgroundColor: "transparent", color: "#4a4a52", border: "none" },
    ".cm-activeLine": { backgroundColor: "#ffffff08" },
    ".cm-activeLineGutter": { backgroundColor: "transparent" },
    ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": { backgroundColor: "#2a3350" },
  },
  { dark: true },
);
