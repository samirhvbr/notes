import { useCallback, useEffect, useRef, useState } from "react";
import { t } from "../i18n";

/** How far the divider may go, as a percentage of the panes' width. */
const MIN = 20;
const MAX = 80;
/** One arrow press. Coarse enough to be useful, fine enough to aim. */
const STEP = 2;

/**
 * The split's draggable divider (`.continue/0.1d-interface.md` §4.3).
 *
 * **It is a `separator` with a value, not a `<div>` with a mousedown.** A drag
 * handle reachable only by mouse is a control that half the people using this
 * application cannot operate, and §3 puts keyboard first. So: it takes focus,
 * the arrows move it, `Home` and `End` go to the limits, and it reports its
 * position — which is what a screen reader reads out instead of "5 pixels wide".
 *
 * The position lives in a CSS custom property on the panes element rather than
 * in React state per frame. Dragging then costs one style write per mouse move
 * instead of a re-render of two panes, one of which contains CodeMirror.
 */
export function Divider({ panes }: { panes: React.RefObject<HTMLElement | null> }) {
  const [pct, setPct] = useState(50);
  const dragging = useRef(false);

  const apply = useCallback(
    (next: number) => {
      const clamped = Math.min(MAX, Math.max(MIN, next));
      setPct(clamped);
      panes.current?.style.setProperty("--split", `${clamped}%`);
    },
    [panes],
  );

  // The listeners go on the document, not on the handle: a fast drag leaves the
  // 5-pixel handle behind long before the mouse button comes up.
  useEffect(() => {
    const move = (e: MouseEvent) => {
      if (!dragging.current) return;
      const box = panes.current?.getBoundingClientRect();
      if (!box || box.width === 0) return;
      e.preventDefault();
      apply(((e.clientX - box.left) / box.width) * 100);
    };
    const up = () => {
      dragging.current = false;
      document.body.style.removeProperty("cursor");
      document.body.style.removeProperty("user-select");
    };
    document.addEventListener("mousemove", move);
    document.addEventListener("mouseup", up);
    return () => {
      document.removeEventListener("mousemove", move);
      document.removeEventListener("mouseup", up);
    };
  }, [apply, panes]);

  return (
    <div
      className="divider"
      role="separator"
      aria-orientation="vertical"
      aria-label={t("split.divider")}
      aria-valuenow={Math.round(pct)}
      aria-valuemin={MIN}
      aria-valuemax={MAX}
      tabIndex={0}
      onMouseDown={(e) => {
        e.preventDefault();
        dragging.current = true;
        // While dragging, the cursor is the divider's wherever it is, and the
        // text under it must not select — otherwise a drag across the editor
        // highlights the note it is resizing.
        document.body.style.cursor = "col-resize";
        document.body.style.userSelect = "none";
      }}
      onDoubleClick={() => apply(50)}
      onKeyDown={(e) => {
        if (e.key === "ArrowLeft") {
          e.preventDefault();
          apply(pct - STEP);
        } else if (e.key === "ArrowRight") {
          e.preventDefault();
          apply(pct + STEP);
        } else if (e.key === "Home") {
          e.preventDefault();
          apply(MIN);
        } else if (e.key === "End") {
          e.preventDefault();
          apply(MAX);
        } else if (e.key === "Enter") {
          // Back to even, which is the same thing a double click does.
          e.preventDefault();
          apply(50);
        }
      }}
    />
  );
}
