import { beforeEach, describe, expect, it } from "vitest";
import { askConfirm, askText, useDialog } from "./dialog";

/**
 * The contract the six flows depend on.
 *
 * Milestone 0.1b shipped with `window.prompt` and `window.confirm` behind new
 * note, new folder, the workspace name, rename, move and delete — dialogs the
 * WebView does not have, so all six were dead. **No 0.1b criterion could have
 * caught it**: every one of them is an assertion about `notes-core`.
 *
 * These tests are the part of that gap a machine can close. They exercise the
 * store rather than the rendered component — resolving, cancelling, validating
 * and not stacking are what a caller can get wrong, and they need no DOM, so
 * they run in the same suite as everything else.
 */
beforeEach(() => {
  useDialog.setState({ current: null });
});

describe("askText", () => {
  it("resolves with the value the dialog settles on", async () => {
    const answer = askText({ title: "New note", confirmLabel: "Create" });
    expect(useDialog.getState().current).not.toBeNull();
    useDialog.getState().settle("minha-nota");
    await expect(answer).resolves.toBe("minha-nota");
    expect(useDialog.getState().current).toBeNull();
  });

  it("resolves with null when cancelled, which is how a caller aborts", async () => {
    const answer = askText({ title: "New note", confirmLabel: "Create" });
    useDialog.getState().settle(null);
    await expect(answer).resolves.toBeNull();
  });

  it("resolves with the empty string rather than null when that is the answer", async () => {
    // `move` uses the empty string to mean the workspace root, so it must be
    // distinguishable from a cancellation. Conflating them would silently
    // refuse to move a note to the root.
    const answer = askText({ title: "Move", confirmLabel: "Move", initial: "sub" });
    useDialog.getState().settle("");
    await expect(answer).resolves.toBe("");
  });

  it("carries the request through so the surface can render it", () => {
    void askText({
      title: "Rename",
      label: "New name",
      initial: "nota.md",
      confirmLabel: "Rename",
    });
    const r = useDialog.getState().current;
    expect(r).toMatchObject({
      kind: "text",
      title: "Rename",
      label: "New name",
      initial: "nota.md",
      confirmLabel: "Rename",
    });
  });

  it("keeps the validator, so an empty name is refused before the core is asked", () => {
    void askText({
      title: "New note",
      confirmLabel: "Create",
      validate: (v) => (v.trim() ? null : "A name is required."),
    });
    const r = useDialog.getState().current;
    expect(r?.kind).toBe("text");
    if (r?.kind !== "text") throw new Error("unreachable");
    expect(r.validate?.("")).toBe("A name is required.");
    expect(r.validate?.("   ")).toBe("A name is required.");
    expect(r.validate?.("nota")).toBeNull();
  });
});

describe("askConfirm", () => {
  it("resolves true when confirmed and false when cancelled", async () => {
    const yes = askConfirm({ title: "Delete", confirmLabel: "Delete" });
    useDialog.getState().settle(true);
    await expect(yes).resolves.toBe(true);

    const no = askConfirm({ title: "Delete", confirmLabel: "Delete" });
    useDialog.getState().settle(false);
    await expect(no).resolves.toBe(false);
  });

  it("marks a destructive question so the surface can say so", () => {
    void askConfirm({ title: "Delete", confirmLabel: "Delete", danger: true });
    expect(useDialog.getState().current).toMatchObject({ kind: "confirm", danger: true });
  });
});

describe("one at a time", () => {
  it("cancels an open request rather than stacking a second over it", async () => {
    const first = askText({ title: "First", confirmLabel: "OK" });
    const second = askText({ title: "Second", confirmLabel: "OK" });

    // The first must resolve — a promise nobody settles is a flow that hangs
    // with no dialog on screen, which is the failure this replaced.
    await expect(first).resolves.toBeNull();
    expect(useDialog.getState().current).toMatchObject({ title: "Second" });

    useDialog.getState().settle("segundo");
    await expect(second).resolves.toBe("segundo");
  });

  it("cancels an open confirm as false, not as null", async () => {
    const first = askConfirm({ title: "Delete", confirmLabel: "Delete" });
    void askText({ title: "Rename", confirmLabel: "Rename" });
    await expect(first).resolves.toBe(false);
  });
});

describe("settling with nothing open", () => {
  it("is a no-op rather than a throw", () => {
    expect(() => useDialog.getState().settle("x")).not.toThrow();
    expect(useDialog.getState().current).toBeNull();
  });
});
