import { useCallback, useEffect, useRef, useState } from "react";
import * as ipc from "../ipc";
import { t } from "../i18n";
import { useEditor } from "../stores/editor";
import { useUi } from "../stores/ui";
import { useWorkspace } from "../stores/workspace";

/**
 * The preview pane.
 *
 * `innerHTML` is assigned here, and there is exactly one reason that is
 * allowed: the string came from `markdown_render`, which is `notes-markdown`
 * behind `ammonia` (docs/ARCHITECTURE.md §10). **Nothing else in this
 * application may assign `innerHTML`**, and this component must never render a
 * string it did not get from that command.
 *
 * Rendering is debounced 300 ms and **skipped while the pane is hidden**, so
 * typing in Source mode costs nothing (§13).
 */
export function Preview() {
  const doc = useEditor((s) => s.doc);
  const open = useEditor((s) => s.open);
  const view = useUi((s) => s.view);
  const fail = useWorkspace((s) => s.fail);
  const host = useRef<HTMLDivElement | null>(null);
  const [html, setHtml] = useState("");
  const [blocked, setBlocked] = useState<string[]>([]);
  const [failed, setFailed] = useState(false);

  const visible = view !== "source";
  const text = doc?.text ?? "";
  const path = doc?.path;

  useEffect(() => {
    if (!visible || !path) return;
    let live = true;
    const id = setTimeout(() => {
      ipc
        .markdownRender(path, text)
        .then((r) => {
          if (!live) return;
          setHtml(r.html);
          setBlocked(r.blocked_remote);
          setFailed(false);
        })
        .catch(() => live && setFailed(true));
    }, 300);
    return () => {
      live = false;
      clearTimeout(id);
    };
  }, [visible, path, text]);

  // The rendered HTML is trusted because of where it came from, and for no
  // other reason. See the note on this component.
  useEffect(() => {
    if (host.current) host.current.innerHTML = html;
  }, [html]);

  /**
   * A click inside the preview never navigates.
   *
   * A relative link opens the note in-app; an `http(s)` link goes to the
   * operating system's browser through a command that checks the scheme again.
   * Everything else was already dropped by the renderer, so there is nothing
   * left to decide here.
   */
  const onClick = useCallback(
    (e: React.MouseEvent) => {
      const anchor = (e.target as HTMLElement).closest("a");
      if (!anchor) return;
      e.preventDefault();

      const notePath = anchor.getAttribute("data-note-path");
      if (notePath) {
        void open(notePath as ipc.RelPath).catch(fail);
        return;
      }
      const href = anchor.getAttribute("href") ?? "";
      if (href.startsWith("#")) {
        host.current?.querySelector(`[id="${CSS.escape(href.slice(1))}"]`)
          ?.scrollIntoView({ block: "start" });
        return;
      }
      if (/^https?:/i.test(href)) void ipc.shellOpen(href).catch(fail);
    },
    [open, fail],
  );

  const allowRemote = useCallback(async () => {
    try {
      await ipc.markdownTrustSet(null, true);
      if (path) {
        const r = await ipc.markdownRender(path, text);
        setHtml(r.html);
        setBlocked(r.blocked_remote);
      }
    } catch (e) {
      fail(e);
    }
  }, [path, text, fail]);

  if (!doc) return null;

  return (
    <div className="preview-pane">
      {blocked.length > 0 && (
        <div className="banner">
          <span>{t("preview.blocked", { count: blocked.length })}</span>
          <button onClick={allowRemote}>{t("preview.blocked.allow")}</button>
        </div>
      )}
      {failed && <div className="banner warn">{t("preview.failed")}</div>}
      {/* eslint-disable-next-line jsx-a11y/no-static-element-interactions */}
      <div className="preview" ref={host} onClick={onClick} />
    </div>
  );
}
