import { useEffect, useRef } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";

/**
 * CodeMirror 6 host.
 *
 * This component is the whole point of the 0.0 spike on mobile: the acceptance
 * criterion is typing "ação", a dead-key "ç" and an emoji on a virtual keyboard
 * without duplication or loss, plus select / paste / undo. Nothing here tries to
 * be clever about input — an IME composes through CodeMirror's own handling, and
 * any interference we add would be measuring ourselves rather than the platform.
 */
export function Editor({
  value,
  docKey,
  onChange,
}: {
  value: string;
  /** Changing this remounts the document. Path, not content. */
  docKey: string;
  onChange: (next: string) => void;
}) {
  const host = useRef<HTMLDivElement | null>(null);
  const view = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (!host.current) return;
    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        markdown({ codeLanguages: languages }),
        EditorView.lineWrapping,
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onChangeRef.current(u.state.doc.toString());
        }),
        EditorView.theme(
          {
            "&": { height: "100%", fontSize: "14px" },
            ".cm-scroller": {
              fontFamily:
                "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
              lineHeight: "1.6",
            },
            ".cm-content": { caretColor: "#e6e6e6" },
            "&.cm-focused .cm-cursor": { borderLeftColor: "#e6e6e6" },
            ".cm-gutters": {
              backgroundColor: "transparent",
              color: "#4a4a52",
              border: "none",
            },
            ".cm-activeLine": { backgroundColor: "#ffffff08" },
            ".cm-activeLineGutter": { backgroundColor: "transparent" },
          },
          { dark: true }
        ),
      ],
    });
    const v = new EditorView({ state, parent: host.current });
    view.current = v;
    return () => {
      v.destroy();
      view.current = null;
    };
    // docKey, not value: re-creating the view on every keystroke would destroy
    // undo history and the IME composition this spike exists to measure.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [docKey]);

  return <div className="editor" ref={host} />;
}
