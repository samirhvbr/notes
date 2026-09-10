import { useEffect, useRef, useState } from "react";
import { t } from "../i18n";
import { useDialog } from "./dialog";

/**
 * The single modal surface. Mounted once; renders only while a request is open.
 *
 * Escape cancels and Enter confirms, because those are the two keys a person
 * already expects from the dialogs this replaces. Focus moves into the dialog on
 * open and returns to whatever had it on close — a modal that steals focus and
 * does not give it back leaves keyboard navigation stranded, and this
 * application is keyboard-first.
 */
export function Dialog() {
  const current = useDialog((s) => s.current);
  const settle = useDialog((s) => s.settle);
  const input = useRef<HTMLInputElement | null>(null);
  const restoreTo = useRef<HTMLElement | null>(null);
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!current) return;
    restoreTo.current = document.activeElement as HTMLElement | null;
    setValue(current.kind === "text" ? current.initial : "");
    setError(null);
    const id = requestAnimationFrame(() => {
      input.current?.focus();
      input.current?.select();
    });
    return () => {
      cancelAnimationFrame(id);
      restoreTo.current?.focus?.();
    };
  }, [current]);

  if (!current) return null;

  const cancel = () => settle(current.kind === "confirm" ? false : null);

  const accept = () => {
    if (current.kind === "confirm") return settle(true);
    const problem = current.validate?.(value) ?? null;
    if (problem) return setError(problem);
    settle(value);
  };

  return (
    <div
      className="overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) cancel();
      }}
    >
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-label={current.title}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            cancel();
          }
          if (e.key === "Enter" && current.kind === "confirm") {
            e.preventDefault();
            accept();
          }
        }}
      >
        <h2>{current.title}</h2>

        {current.kind === "text" ? (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              accept();
            }}
          >
            {current.label && <label htmlFor="dialog-input">{current.label}</label>}
            <input
              id="dialog-input"
              ref={input}
              value={value}
              spellCheck={false}
              autoComplete="off"
              aria-invalid={error ? true : undefined}
              aria-describedby={error ? "dialog-error" : undefined}
              onChange={(e) => {
                setValue(e.target.value);
                if (error) setError(null);
              }}
            />
            {error && (
              <p id="dialog-error" className="bad">
                {error}
              </p>
            )}
            <div className="actions">
              <button type="button" onClick={cancel}>
                {t("dialog.cancel")}
              </button>
              <button type="submit" className="primary">
                {current.confirmLabel}
              </button>
            </div>
          </form>
        ) : (
          <>
            {current.body && <p>{current.body}</p>}
            <div className="actions">
              <button type="button" onClick={cancel}>
                {t("dialog.cancel")}
              </button>
              <button
                type="button"
                ref={input as unknown as React.RefObject<HTMLButtonElement>}
                className={current.danger ? "danger" : "primary"}
                onClick={accept}
              >
                {current.confirmLabel}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
