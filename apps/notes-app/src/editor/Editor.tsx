import { useEffect, useRef } from "react";
import { EditorState } from "@codemirror/state";
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
export function Editor() {
  const doc = useEditor((s) => s.doc);
  const edit = useEditor((s) => s.edit);
  const host = useRef<HTMLDivElement | null>(null);
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
          if (u.docChanged) editRef.current(u.state.doc.toString());
        }),
        theme,
      ],
    });
    const view = new EditorView({ state, parent: host.current });
    return () => view.destroy();
    // Keyed by the document, not its text: rebuilding on every keystroke would
    // destroy the undo history and the IME composition.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key === null ? null : key.split(":")[0], readOnly]);

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
