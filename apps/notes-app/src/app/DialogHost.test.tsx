// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Dialog } from "./DialogHost";
import { askConfirm, askText, useDialog } from "./dialog";

function Harness({ text = false, answered }: { text?: boolean; answered: (v: unknown) => void }) {
  return <>
    <button onClick={() => void (text
      ? askText({ title: "Rename", label: "Name", initial: "note", confirmLabel: "Save" })
      : askConfirm({ title: "Delete note", confirmLabel: "Delete", danger: true })
    ).then(answered)}>Open</button>
    <Dialog />
    <button>Outside</button>
  </>;
}

afterEach(() => {
  cleanup();
  useDialog.getState().settle(null);
});

async function open(text = false) {
  const answers: unknown[] = [];
  const user = userEvent.setup();
  render(<Harness text={text} answered={(v) => answers.push(v)} />);
  await user.click(screen.getByRole("button", { name: "Open" }));
  await waitFor(() => expect(text ? screen.getByRole("textbox") : screen.getByRole("button", { name: "Delete" })).toHaveFocus());
  return { user, answers };
}

describe("Dialog keyboard behavior", () => {
  it("Enter on Cancel cancels a destructive confirmation", async () => {
    const { user, answers } = await open();
    await user.tab({ shift: true });
    await user.keyboard("{Enter}");
    expect(answers).toEqual([false]);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("button", { name: "Open" })).toHaveFocus();
  });

  it("Enter on the confirmation button confirms", async () => {
    const { user, answers } = await open();
    await user.keyboard("{Enter}");
    expect(answers).toEqual([true]);
    expect(screen.getByRole("button", { name: "Open" })).toHaveFocus();
  });

  it("keeps Tab and Shift+Tab within confirmation controls", async () => {
    const { user } = await open();
    await user.tab();
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();
    await user.tab({ shift: true });
    expect(screen.getByRole("button", { name: "Delete" })).toHaveFocus();
  });

  it("selects the initial text, wraps focus and restores it after Escape", async () => {
    const { user, answers } = await open(true);
    const input = screen.getByRole("textbox") as HTMLInputElement;
    expect(input.selectionStart).toBe(0);
    expect(input.selectionEnd).toBe(4);
    await user.tab({ shift: true });
    expect(screen.getByRole("button", { name: "Save" })).toHaveFocus();
    await user.tab();
    expect(input).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(answers).toEqual([null]);
    expect(screen.getByRole("button", { name: "Open" })).toHaveFocus();
  });

  it("does not send modal keystrokes to application shortcuts", async () => {
    const { user } = await open(true);
    const shortcut = vi.fn();
    window.addEventListener("keydown", shortcut);
    try {
      await user.keyboard("{Control>}p{/Control}");
      expect(shortcut).not.toHaveBeenCalled();
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    } finally {
      window.removeEventListener("keydown", shortcut);
    }
  });

  it("submits edited text with Enter", async () => {
    const { user, answers } = await open(true);
    await user.keyboard("renamed{Enter}");
    expect(answers).toEqual(["renamed"]);
  });
});
