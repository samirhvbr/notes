import { useEffect, useRef } from "react";
import { Annotation, EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { search, searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { markdown } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { useEditor } from "../stores/editor";
import { pendingCursor, useTabs } from "../stores/tabs";
import { useSettings } from "../stores/settings";
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
  const noteIdRef = useRef(doc?.noteId);
  noteIdRef.current = doc?.noteId;
  const reportCursor = useRef((line: number, col: number, scrollTop: number) => {
    const id = noteIdRef.current;
    if (id) useTabs.getState().noteCursor(id, line, col, scrollTop);
  }).current;
  const host = useRef<HTMLDivElement | null>(null);
  const view = useRef<EditorView | null>(null);
  const editRef = useRef(edit);
  editRef.current = edit;

  // The editor's own settings (0.1c). Line numbers and wrapping are CodeMirror
  // *extensions*, so changing one rebuilds the view — which is why they are in
  // the key below rather than applied afterwards.
  const editorSettings = useSettings((s) => s.settings?.editor);
  const fontSize = editorSettings?.font_size ?? 14;
  const lineNumbersOn = editorSettings?.line_numbers ?? true;
  const wrapOn = editorSettings?.word_wrap ?? true;
  const tabSize = editorSettings?.tab_size ?? 2;

  const key = doc ? `${doc.noteId}:${doc.savedVersion}` : null;
  const initial = useRef(doc?.text ?? "");
  const readOnly = !!doc?.readOnly;
  if (doc && key !== null) initial.current = doc.text;

  useEffect(() => {
    if (!host.current || key === null) return;
    const state = EditorState.create({
      doc: initial.current,
      extensions: [
        ...(lineNumbersOn ? [lineNumbers()] : []),
        highlightActiveLine(),
        EditorState.tabSize.of(tabSize),
        history(),
        // Search and replace **within the file** — the 0.1b half of scope §10.
        // Global search is 0.1c and is a different thing entirely: it scans the
        // workspace in the core, streams results and is cancellable. This one
        // is `Ctrl+F`, it runs on the buffer in front of the user, and it
        // therefore searches what is being typed rather than what is saved.
        search({ top: true }),
        highlightSelectionMatches(),
        // `searchKeymap` first: `Ctrl+F` and `Ctrl+H` must reach the panel
        // rather than whatever `defaultKeymap` would do with them.
        keymap.of([...searchKeymap, ...defaultKeymap, ...historyKeymap]),
        markdown({ codeLanguages: languages }),
        ...(wrapOn ? [EditorView.lineWrapping] : []),
        EditorState.readOnly.of(readOnly),
        EditorState.lineSeparator.of("\n"),
        // The caret and the scroll position belong to the tab, not to the
        // document: the 0.1c criterion is that reopening the application puts
        // them back (`stores/tabs.ts`).
        EditorView.updateListener.of((u) => {
          if (u.docChanged || u.selectionSet || u.geometryChanged) {
            const head = u.state.selection.main.head;
            const line = u.state.doc.lineAt(head);
            reportCursor(
              line.number,
              head - line.from + 1,
              Math.round(u.view.scrollDOM.scrollTop),
            );
          }
          if (!u.docChanged) return;
          // A reload from disk is not a keystroke: marking the buffer dirty
          // here would start an autosave of text the user never typed.
          if (u.transactions.some((tr) => tr.annotation(External))) return;
          editRef.current(u.state.doc.toString());
        }),
        theme,
        EditorView.theme({ "&": { fontSize: `${fontSize}px` } }),
      ],
    });
    const created = new EditorView({ state, parent: host.current });
    view.current = created;

    // Put the caret back where the tab left it. After creation, because the
    // document has to exist before a position in it means anything.
    const want = pendingCursor(noteIdRef.current);
    if (want) {
      const lines = created.state.doc.lines;
      const line = created.state.doc.line(Math.min(Math.max(want.line, 1), lines));
      const pos = Math.min(line.from + Math.max(want.col - 1, 0), line.to);
      created.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
    }
    return () => {
      created.destroy();
      if (view.current === created) view.current = null;
    };
    // Keyed by the document, not its text: rebuilding on every keystroke would
    // destroy the undo history and the IME composition.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    // Settings are in the dependency list because each of them is an extension:
    // there is no way to change them on a live view without rebuilding it.
  }, [key === null ? null : key.split(":")[0], readOnly, lineNumbersOn, wrapOn, tabSize, fontSize]);

  // A jump into the note already on screen: the view will not rebuild, so the
  // caret is moved directly. Clicking a search hit is the case (C8).
  const gotoRev = useTabs((s) => s.gotoRev);
  useEffect(() => {
    if (!gotoRev) return;
    const v = view.current;
    const want = pendingCursor(noteIdRef.current);
    if (!v || !want) return;
    const lines = v.state.doc.lines;
    const line = v.state.doc.line(Math.min(Math.max(want.line, 1), lines));
    const pos = Math.min(line.from + Math.max(want.col - 1, 0), line.to);
    v.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
    v.focus();
  }, [gotoRev]);

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
