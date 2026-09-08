import { useCallback, useEffect, useId, useRef, useState } from "react";

/**
 * One popup menu, and every menu in the application is this one.
 *
 * The workspace selector, the note header's `⋮` and the explorer's context menu
 * are the same interaction over different rows — open, arrow, `Enter` — so they
 * are one component, for the reason `Palette` is one component for quick open
 * and the command palette.
 *
 * **Keyboard is not an extra here** (`.continue/0.1d-interface.md` §3): every
 * menu has a keyboard path and **returns focus to the control that opened it**.
 * A menu that swallows focus leaves a keyboard user stranded on `<body>` with
 * no way back, and that is not a degraded experience — it is a dead end.
 *
 * What this deliberately is *not*: a `<dialog>`, a portal, or a library. It is
 * an absolutely-positioned list next to its trigger, closed by `Escape`, by a
 * click outside, and by choosing something.
 */

export interface MenuItem {
  id: string;
  label: string;
  /** Rendered before the label. An icon element, never text. */
  icon?: React.ReactNode;
  /** Dimmer, secondary text after the label — a path, a shortcut. */
  hint?: string;
  disabled?: boolean;
  /** Shown on hover and to assistive technology when `disabled` explains why. */
  title?: string;
  danger?: boolean;
  run?: () => void | Promise<void>;
}

/** A rule between groups. It is not focusable and it is not an item. */
export interface MenuSeparator {
  separator: true;
  /** An optional group label, e.g. "recent". */
  label?: string;
}

export type MenuRow = MenuItem | MenuSeparator;

const isItem = (r: MenuRow): r is MenuItem => !("separator" in r);
/** Only enabled items take focus. A disabled row is announced, never landed on. */
const isFocusable = (r: MenuRow): r is MenuItem => isItem(r) && !r.disabled;

export function Menu({
  rows,
  open,
  onClose,
  label,
  align = "start",
  trigger,
}: {
  rows: MenuRow[];
  open: boolean;
  onClose: () => void;
  /** Names the menu for assistive technology. Never rendered. */
  label: string;
  align?: "start" | "end";
  /** The control that opened it. Focus goes back here on close. */
  trigger: React.RefObject<HTMLElement | null>;
}) {
  const listRef = useRef<HTMLUListElement | null>(null);
  const [cursor, setCursor] = useState(0);
  const id = useId();

  const focusable = rows.filter(isFocusable);

  // Opening puts the cursor on the first item that can take it. A menu that
  // opens with nothing focused makes the first arrow press do nothing, which
  // reads as a menu that ignores the keyboard.
  useEffect(() => {
    if (open) setCursor(0);
  }, [open]);

  // Focus follows the cursor, so the browser's own focus ring is the highlight
  // and there is no second source of truth about which row is current.
  useEffect(() => {
    if (!open) return;
    const el = listRef.current?.querySelectorAll<HTMLElement>("[data-menu-item]")[cursor];
    el?.focus();
  }, [open, cursor]);

  const close = useCallback(
    (restoreFocus: boolean) => {
      onClose();
      // Back to the trigger. Doing it after the state update, so the menu is
      // gone and cannot steal it again on the same tick.
      if (restoreFocus) queueMicrotask(() => trigger.current?.focus());
    },
    [onClose, trigger],
  );

  const choose = useCallback(
    async (item: MenuItem) => {
      if (item.disabled) return;
      // Closed **before** the action runs: an action that opens a native file
      // chooser or another modal must not do it underneath an open menu, and
      // one that unmounts this tree would leave the focus restore chasing a
      // node that is gone.
      close(false);
      await item.run?.();
    },
    [close],
  );

  // A click anywhere else closes it — including on the trigger, which would
  // otherwise toggle it shut and straight back open on the same click.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (listRef.current?.contains(target)) return;
      if (trigger.current?.contains(target)) return;
      close(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open, close, trigger]);

  if (!open) return null;

  const step = (delta: number) => {
    if (focusable.length === 0) return;
    setCursor((c) => (c + delta + focusable.length) % focusable.length);
  };

  return (
    <ul
      ref={listRef}
      id={id}
      className={`menu menu-${align}`}
      role="menu"
      aria-label={label}
      onKeyDown={(e) => {
        switch (e.key) {
          case "Escape":
            e.preventDefault();
            close(true);
            break;
          case "ArrowDown":
            e.preventDefault();
            step(1);
            break;
          case "ArrowUp":
            e.preventDefault();
            step(-1);
            break;
          case "Home":
            e.preventDefault();
            setCursor(0);
            break;
          case "End":
            e.preventDefault();
            setCursor(Math.max(focusable.length - 1, 0));
            break;
          case "Tab":
            // Tab leaves a menu rather than walking it; the arrows walk it.
            e.preventDefault();
            close(true);
            break;
          default:
            break;
        }
      }}
    >
      {rows.map((row, i) =>
        isItem(row) ? (
          <li key={row.id}>
            <button
              type="button"
              role="menuitem"
              // Disabled rows are still rendered and still announced — the
              // graph entry says "0.3" rather than being absent (§3).
              aria-disabled={row.disabled || undefined}
              disabled={row.disabled}
              title={row.title}
              className={`menu-item${row.danger ? " danger" : ""}`}
              {...(row.disabled ? {} : { "data-menu-item": "" })}
              tabIndex={-1}
              onClick={() => void choose(row)}
            >
              {row.icon && (
                <span className="menu-icon" aria-hidden="true">
                  {row.icon}
                </span>
              )}
              <span className="menu-label">{row.label}</span>
              {row.hint && <span className="menu-hint muted">{row.hint}</span>}
            </button>
          </li>
        ) : (
          <li key={`sep-${i}`} className="menu-sep" role="separator">
            {row.label && <span className="muted">{row.label}</span>}
          </li>
        ),
      )}
    </ul>
  );
}
