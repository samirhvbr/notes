// The application's own modal dialogs.
//
// The browser's blocking script dialogs are not used anywhere in this codebase,
// and `tools/no-blocking-dialogs.sh` fails the build if they come back — with no
// exclusions, which is why this comment does not spell them. They belong to a
// *browser*, and the WebView this application ships in —
// WebKitGTK on Linux, WKWebView on Apple, WebView2 on Windows — is not a
// browser: their behaviour ranges from "does nothing" to "blocks the WebView's
// main loop", and every flow behind one is dead without a sound. Six flows of
// milestone 0.1b were behind one, and no core test could have known: the
// criteria of 0.1b are assertions about `notes-core`.
//
// The file picker stays native (`@tauri-apps/plugin-dialog`) because a folder
// chooser is the operating system's job. Everything else is in-app.
import { create } from "zustand";

interface TextRequest {
  kind: "text";
  title: string;
  label?: string;
  initial: string;
  confirmLabel: string;
  /** Returns an error key when the value cannot be accepted. */
  validate?: (value: string) => string | null;
}

interface ConfirmRequest {
  kind: "confirm";
  title: string;
  body?: string;
  confirmLabel: string;
  danger: boolean;
}

type Request = (TextRequest | ConfirmRequest) & {
  resolve: (value: string | boolean | null) => void;
};

interface DialogState {
  current: Request | null;
  push: (r: Request) => void;
  settle: (value: string | boolean | null) => void;
}

export const useDialog = create<DialogState>((set, get) => ({
  current: null,
  push: (r) => {
    // One at a time. A second request while one is open resolves the first as
    // cancelled rather than stacking two modals over each other.
    const open = get().current;
    if (open) open.resolve(open.kind === "confirm" ? false : null);
    set({ current: r });
  },
  settle: (value) => {
    const open = get().current;
    if (!open) return;
    set({ current: null });
    open.resolve(value);
  },
}));

/** Ask for a line of text. Resolves to `null` when cancelled. */
export function askText(opts: {
  title: string;
  label?: string;
  initial?: string;
  confirmLabel: string;
  validate?: (value: string) => string | null;
}): Promise<string | null> {
  return new Promise((resolve) => {
    useDialog.getState().push({
      kind: "text",
      title: opts.title,
      label: opts.label,
      initial: opts.initial ?? "",
      confirmLabel: opts.confirmLabel,
      validate: opts.validate,
      resolve: resolve as (v: string | boolean | null) => void,
    });
  });
}

/** Ask a yes/no question. Resolves to `false` when cancelled. */
export function askConfirm(opts: {
  title: string;
  body?: string;
  confirmLabel: string;
  danger?: boolean;
}): Promise<boolean> {
  return new Promise((resolve) => {
    useDialog.getState().push({
      kind: "confirm",
      title: opts.title,
      body: opts.body,
      confirmLabel: opts.confirmLabel,
      danger: opts.danger ?? false,
      resolve: resolve as (v: string | boolean | null) => void,
    });
  });
}
