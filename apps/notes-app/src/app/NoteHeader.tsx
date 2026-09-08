import { ArrowLeft, ArrowRight, Eye, MoreVertical, Pencil } from "lucide-react";
import { useRef, useState } from "react";
import { t } from "../i18n";
import { useEditor } from "../stores/editor";
import { useHistory } from "../stores/history";
import { useUi } from "../stores/ui";
import { useWorkspace } from "../stores/workspace";
import { Menu, type MenuRow } from "./Menu";

/**
 * The note's own header (`.continue/0.1d-interface.md` §4.3).
 *
 * Back and forward on the left, the title centred, the Source/Preview toggle
 * and the `⋮` on the right. It belongs to the note rather than to the window,
 * which is why it sits below the tab strip and not in a top bar: at 0.1d's
 * split (D-04) there are two of these, one per pane, and a header in the window
 * chrome could not be.
 *
 * The title is the file name **without `.md`**. The extension is true and it is
 * noise in a heading: the path is on the tab's tooltip and in the status bar
 * for anyone who needs the whole of it.
 */
export function NoteHeader() {
  const doc = useEditor((s) => s.doc);
  const view = useUi((s) => s.view);
  const setView = useUi((s) => s.setView);
  const note = useWorkspace((s) => s.note);
  const { canBack, canForward, back, forward } = useHistory();
  const [menu, setMenu] = useState(false);
  const trigger = useRef<HTMLButtonElement | null>(null);

  if (!doc) return null;

  const name = doc.path.split("/").pop() ?? doc.path;
  const title = name.replace(/\.(md|markdown|mdown|mkd)$/i, "");
  const showing = view === "preview";

  const rows: MenuRow[] = [
    {
      id: "copy-path",
      label: t("note.copyPath"),
      run: async () => {
        await navigator.clipboard.writeText(doc.path);
        note(t("note.pathCopied"));
      },
    },
    {
      id: "reveal",
      label: t("note.showPath"),
      hint: doc.path,
      // Nothing to run: it is the path itself, shown because a centred title
      // that dropped the extension has to be checkable somewhere.
      disabled: true,
    },
  ];

  return (
    <header className="note-head">
      <div className="note-nav">
        <button
          type="button"
          className="icon-btn"
          aria-label={t("note.back")}
          title={t("note.back")}
          disabled={!canBack}
          onClick={() => void back()}
        >
          <ArrowLeft size={15} aria-hidden="true" />
        </button>
        <button
          type="button"
          className="icon-btn"
          aria-label={t("note.forward")}
          title={t("note.forward")}
          disabled={!canForward}
          onClick={() => void forward()}
        >
          <ArrowRight size={15} aria-hidden="true" />
        </button>
      </div>

      <h1 className="note-title" title={doc.path}>
        {title}
      </h1>

      <div className="note-actions">
        <button
          type="button"
          className="icon-btn"
          // One button with two meanings needs its label to say what pressing
          // it does, not which mode is current.
          aria-label={showing ? t("note.toSource") : t("note.toPreview")}
          title={showing ? t("note.toSource") : t("note.toPreview")}
          onClick={() => setView(showing ? "source" : "preview")}
        >
          {showing ? <Pencil size={15} aria-hidden="true" /> : <Eye size={15} aria-hidden="true" />}
        </button>
        <button
          ref={trigger}
          type="button"
          className="icon-btn"
          aria-haspopup="menu"
          aria-expanded={menu}
          aria-label={t("note.more")}
          title={t("note.more")}
          onClick={() => setMenu((m) => !m)}
        >
          <MoreVertical size={15} aria-hidden="true" />
        </button>
        <Menu
          rows={rows}
          open={menu}
          onClose={() => setMenu(false)}
          label={t("note.more")}
          align="end"
          trigger={trigger}
        />
      </div>
    </header>
  );
}
