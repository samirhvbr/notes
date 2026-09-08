import { useEffect, useMemo, useState } from "react";
import * as ipc from "../ipc";
import { t } from "../i18n";
import { useEditor } from "../stores/editor";
import { useUi } from "../stores/ui";
import { useWorkspace } from "../stores/workspace";
import { diffLines } from "./diff";

/**
 * The comparison screen of scope §12 — *comparar · manter o meu · usar o do
 * disco · salvar como `nome (local).md`*.
 *
 * **Comparing is not a command**, and that is the whole design of this file:
 * it changes nothing on disk and reads two strings the application already has
 * — the buffer, and what `note_reload` returns — so it is a screen
 * (docs/ARCHITECTURE.md §17.1). The three buttons under it are one call to
 * `conflict_resolve`, which keeps the version the user did not pick.
 *
 * It was the piece 0.1a shipped without: the core suspended autosave and wrote
 * the draft, and the interface said only that something had happened.
 */
export function Compare() {
  const doc = useEditor((s) => s.doc);
  const resolve = useEditor((s) => s.resolveConflict);
  const setComparing = useUi((s) => s.setComparing);
  const fail = useWorkspace((s) => s.fail);

  const [disk, setDisk] = useState<string | null>(null);
  const [gone, setGone] = useState(false);
  const [busy, setBusy] = useState(false);

  const noteId = doc?.noteId;
  useEffect(() => {
    if (!noteId) return;
    let live = true;
    // `note_reload` reads the file and touches no buffer, which is exactly what
    // this screen needs and the reason it is not a bespoke command.
    ipc
      .noteReload(noteId)
      .then((n) => live && setDisk(n.text))
      .catch((e) => {
        if (!live) return;
        if (ipc.asCoreError(e).code === "not_found") setGone(true);
        else fail(e);
      });
    return () => {
      live = false;
    };
  }, [noteId, fail]);

  const diff = useMemo(
    () => (disk === null ? null : diffLines(doc?.text ?? "", disk)),
    [doc?.text, disk],
  );

  if (!doc) return null;

  const act = async (choice: ipc.ConflictChoice) => {
    setBusy(true);
    try {
      await resolve(choice);
      setComparing(false);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="compare">
      <header className="compare-head">
        <strong>{t("conflict.compare.title", { name: doc.path })}</strong>
        <span className="muted">
          {gone
            ? t("conflict.compare.removed")
            : diff
              ? t("conflict.compare.summary", { count: diff.changed })
              : t("conflict.compare.loading")}
        </span>
        <span className="spacer" />
        <button onClick={() => act("keep_local")} disabled={busy}>
          {t("conflict.keepMine")}
        </button>
        <button onClick={() => act("use_disk")} disabled={busy}>
          {gone ? t("conflict.acceptDeletion") : t("conflict.useDisk")}
        </button>
        <button onClick={() => act("save_as_copy")} disabled={busy}>
          {t("conflict.saveAsCopy")}
        </button>
        <button onClick={() => setComparing(false)} disabled={busy}>
          {t("conflict.compare.close")}
        </button>
      </header>

      <p className="compare-note muted">{t("conflict.compare.kept")}</p>

      {diff?.coarse && <p className="banner warn">{t("conflict.compare.coarse")}</p>}

      <div className="compare-cols" role="table" aria-label={t("conflict.compare.title", { name: doc.path })}>
        <div className="compare-col-head">{t("conflict.mine")}</div>
        <div className="compare-col-head">{t("conflict.theirs")}</div>
        {diff?.rows.map((row, i) => (
          <Line key={i} row={row} />
        ))}
      </div>
    </div>
  );
}

function Line({ row }: { row: ReturnType<typeof diffLines>["rows"][number] }) {
  // The kind is carried by a word in the gutter as well as by the colour:
  // state is never conveyed by colour alone (scope §9).
  const mark = { same: " ", changed: "~", mine: "+", theirs: "−" }[row.kind];
  return (
    <>
      <div className={`compare-line k-${row.kind}`}>
        <span className="no">{row.mineNo ?? ""}</span>
        <span className="mark" aria-hidden="true">
          {row.mine === null ? "" : mark}
        </span>
        <span className="text">{row.mine ?? ""}</span>
      </div>
      <div className={`compare-line k-${row.kind}`}>
        <span className="no">{row.theirsNo ?? ""}</span>
        <span className="mark" aria-hidden="true">
          {row.theirs === null ? "" : mark}
        </span>
        <span className="text">{row.theirs ?? ""}</span>
      </div>
    </>
  );
}
