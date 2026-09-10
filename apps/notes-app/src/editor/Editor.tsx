import { isSyncLocked } from "../ipc/barrier";
import * as ipc from "../ipc";
import {useWorkspace} from "../stores/workspace";
import { useEffect, useRef } from "react";
import { Annotation, EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { search, searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { markdown } from "@codemirror/lang-markdown";
import { languages } from "@codemirror/language-data";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";
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
        EditorState.transactionFilter.of(tr => isSyncLocked() && tr.docChanged && !tr.annotation(External) ? [] : tr),
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
        EditorView.domEventHandlers({paste(event,editor){
          const file=Array.from(event.clipboardData?.files??[]).find(f=>f.type.startsWith("image/"));
          if(isSyncLocked())return true;
          if(!file || readOnly)return false;
          event.preventDefault();
          const current=useEditor.getState().doc;
          if(!current)return true;
          const before=editor.state;
          const workspace=useWorkspace.getState().info?.id;
          if(file.size>8*1024*1024){useWorkspace.getState().fail({code:"unsupported",cap:"clipboard image over 8 MiB"});return true;}
          void file.arrayBuffer().then(async buffer=>{
            if(useWorkspace.getState().info?.id!==workspace)return;
            const result=await ipc.attachmentImport(current.path,Array.from(new Uint8Array(buffer)));
            if(useEditor.getState().doc?.noteId===current.noteId && editor.state===before && useWorkspace.getState().info?.id===workspace){editor.dispatch(editor.state.replaceSelection(result.markdown));editor.focus();}
            else useWorkspace.getState().note(t("attachment.created",{path:result.path}));
          }).catch(useWorkspace.getState().fail);
          return true;
        }}),
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
        syntaxHighlighting(highlight),
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

/**
 * The editor's look (`.continue/0.1d-interface.md` §4.3 and §5).
 *
 * **Colours come from the CSS custom properties, not from hex here.** They were
 * hard-coded, which put four of them outside `tools/contrast.sh`'s reach — and a
 * checker that reads `:root` cannot see a colour written in a TypeScript object
 * (`DECISIONS-0.1d.md` D-07). `var()` resolves against the document, so the
 * editor now moves with the palette rather than beside it.
 *
 * The **column** is here rather than in the stylesheet because CodeMirror owns
 * the scroller: centring `.cm-content` with a `max-width` keeps the scrollbar
 * at the window's edge, where it belongs, instead of at the column's.
 *
 * The editor stays a plain-text editor. Markdown syntax is highlighted, not
 * replaced: Live Preview is §18 and this is not a step towards it.
 */
const theme = EditorView.theme(
  {
    "&": { height: "100%", backgroundColor: "var(--bg)", color: "var(--fg)" },
    // **The sans, not the mono.** §5: *"uma sans para interface e corpo, uma
    // mono para código"* — a note is body text, and the editor is where it is
    // written. Setting the whole editor in mono also breaks §4.3's rule that
    // switching Source ↔ Preview must not move the text under the reader: two
    // fonts at the same size do not occupy the same space. Code keeps the mono,
    // below, where it belongs.
    ".cm-scroller": { fontFamily: "var(--font-body)", lineHeight: "1.7" },
    ".cm-content": {
      caretColor: "var(--fg)",
      padding: "28px 0 40vh",
      // The centred column, with margins that grow with the window (D-03).
      // The bottom padding is deliberate: it lets the last line of a note be
      // scrolled to the middle of the screen instead of sitting on the floor.
      maxWidth: "var(--column)",
      marginInline: "auto",
      width: "100%",
    },
    "&.cm-focused .cm-cursor": { borderLeftColor: "var(--accent)", borderLeftWidth: "2px" },
    ".cm-gutters": {
      backgroundColor: "transparent",
      color: "var(--disabled)",
      border: "none",
    },
    ".cm-activeLine": { backgroundColor: "var(--hover-soft)" },
    ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--fg-dim)" },
    ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
      backgroundColor: "var(--selected)",
    },
    // Markdown structure, at the weights §5 asks for: a heading is a heading
    // before it is read.
    ".cm-line": { paddingInline: "4px" },
  },
  { dark: true },
);

/**
 * Markdown, highlighted rather than replaced.
 *
 * `.continue/0.1d-interface.md` §4.3 asks for a large, heavy H1 and code on its
 * own ground. That is highlighting: the `#` stays on screen, the text stays
 * plain, and nothing is hidden or substituted. Hiding the syntax is Live
 * Preview, which is §18 and is not what a bigger heading is a step towards.
 *
 * Sizes are `em`, so they scale with the font size the settings panel controls
 * instead of ignoring it.
 */
const highlight = HighlightStyle.define(
  [
    { tag: tags.heading1, fontSize: "1.9em", fontWeight: "700", lineHeight: "1.3" },
    { tag: tags.heading2, fontSize: "1.5em", fontWeight: "700", lineHeight: "1.35" },
    { tag: tags.heading3, fontSize: "1.25em", fontWeight: "600" },
    { tag: [tags.heading4, tags.heading5, tags.heading6], fontWeight: "600" },
    { tag: tags.strong, fontWeight: "700", color: "var(--fg)" },
    { tag: tags.emphasis, fontStyle: "italic" },
    { tag: tags.strikethrough, textDecoration: "line-through", color: "var(--fg-dim)" },
    { tag: tags.link, color: "var(--accent)" },
    { tag: tags.url, color: "var(--accent)" },
    {
      tag: [tags.monospace, tags.literal],
      color: "var(--good)",
      fontFamily: "var(--font-mono)",
    },
    { tag: tags.quote, color: "var(--fg-dim)", fontStyle: "italic" },
    { tag: tags.list, color: "var(--accent)" },
    // The punctuation Markdown is made of — `#`, `*`, backticks. Dimmed rather
    // than removed: it is what the user typed and it is what they will edit.
    { tag: tags.processingInstruction, color: "var(--disabled)" },
    { tag: tags.contentSeparator, color: "var(--disabled)" },
  ],
  { themeType: "dark" },
);
