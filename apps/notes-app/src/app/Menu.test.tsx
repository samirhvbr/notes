// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef, useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Menu, type MenuRow } from "./Menu";
import { Dialog } from "./DialogHost";
import { askText, useDialog } from "./dialog";

/**
 * `.continue/0.1d-interface.md` §7: *"Navegação por teclado dos menus … teste de
 * componente com foco rastreado."*
 *
 * Focus is the whole subject, which is why this file runs in a DOM and the
 * store tests do not. A reducer could prove which row is *selected*; only a
 * document can prove the focus went back to the button that opened the menu —
 * and that is the failure that strands a keyboard user on `<body>` with no way
 * back to the interface.
 */

function Harness({ rows }: { rows: MenuRow[] }) {
  const trigger = useRef<HTMLButtonElement | null>(null);
  const [open, setOpen] = useState(false);
  return (
    <div>
      <button ref={trigger} onClick={() => setOpen((o) => !o)}>
        open menu
      </button>
      <Menu
        rows={rows}
        open={open}
        onClose={() => setOpen(false)}
        label="test menu"
        trigger={trigger}
      />
    </div>
  );
}

// `globals: false`, so testing-library's automatic cleanup never registers and
// each render would otherwise pile onto the last one's document.
afterEach(() => {
  cleanup();
  useDialog.getState().settle(null);
});

const chose: string[] = [];
function rows(): MenuRow[] {
  return [
    { id: "a", label: "First", run: () => void chose.push("a") },
    { id: "b", label: "Second", run: () => void chose.push("b") },
    { separator: true, label: "recent" },
    { id: "c", label: "Third", run: () => void chose.push("c") },
    { id: "d", label: "Graph", disabled: true, title: "0.3" },
  ];
}

async function openMenu() {
  const user = userEvent.setup();
  render(<Harness rows={rows()} />);
  await user.click(screen.getByText("open menu"));
  return user;
}

describe("Menu keyboard navigation", () => {
  it("focuses the first item as soon as it opens", async () => {
    await openMenu();
    expect(document.activeElement).toHaveTextContent("First");
  });

  it("walks with the arrows and wraps at both ends", async () => {
    const user = await openMenu();
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toHaveTextContent("Second");
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toHaveTextContent("Third");

    // Past the end, back to the start — and the separator is not a stop.
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toHaveTextContent("First");
    await user.keyboard("{ArrowUp}");
    expect(document.activeElement).toHaveTextContent("Third");
  });

  it("never lands on a disabled item", async () => {
    const user = await openMenu();
    for (let i = 0; i < 8; i++) {
      await user.keyboard("{ArrowDown}");
      expect(document.activeElement).not.toHaveTextContent("Graph");
    }
  });

  it("Home and End go to the ends", async () => {
    const user = await openMenu();
    await user.keyboard("{End}");
    expect(document.activeElement).toHaveTextContent("Third");
    await user.keyboard("{Home}");
    expect(document.activeElement).toHaveTextContent("First");
  });

  it("Enter runs the focused item and closes", async () => {
    chose.length = 0;
    const user = await openMenu();
    await user.keyboard("{ArrowDown}{Enter}");
    expect(chose).toEqual(["b"]);
    expect(screen.queryByRole("menu")).toBeNull();
  });

  /** The one that matters: a menu that keeps focus is a dead end. */
  it("Escape closes and gives focus back to the trigger", async () => {
    const user = await openMenu();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toHaveTextContent("open menu");
  });

  it("Tab leaves the menu rather than walking it, and restores focus", async () => {
    const user = await openMenu();
    await user.keyboard("{Tab}");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toHaveTextContent("open menu");
  });

  it("a click outside closes it", async () => {
    const user = await openMenu();
    await user.click(document.body);
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("carries its name and its rows to assistive technology", async () => {
    await openMenu();
    expect(screen.getByRole("menu")).toHaveAttribute("aria-label", "test menu");
    // A disabled row is present and announced, not hidden: §3's rule that a
    // thing which is coming is shown inert with its milestone on it.
    const graph = screen.getByText("Graph").closest("button")!;
    expect(graph).toHaveAttribute("aria-disabled", "true");
    expect(graph).toHaveAttribute("title", "0.3");
  });

  it("a disabled item cannot be run by clicking either", async () => {
    chose.length = 0;
    const user = await openMenu();
    const graph = screen.getByText("Graph").closest("button")!;
    await user.click(graph);
    expect(chose).toEqual([]);
  });

  /**
   * The action runs **after** the menu is gone. Anything that opens a native
   * file chooser — which is most of the workspace selector — must not do it
   * underneath an open menu.
   */
  it("closes before running the action", async () => {
    const order: string[] = [];
    const trigger = { current: null } as React.RefObject<HTMLElement | null>;
    const onClose = vi.fn(() => order.push("closed"));
    const user = userEvent.setup();
    render(
      <Menu
        rows={[{ id: "x", label: "Go", run: () => void order.push("ran") }]}
        open
        onClose={onClose}
        label="m"
        trigger={trigger}
      />,
    );
    await user.click(screen.getByText("Go"));
    expect(order).toEqual(["closed", "ran"]);
  });
});


describe("Menu action focus", () => {
  it("returns focus after selecting an ordinary action", async () => {
    const user = await openMenu();
    await user.keyboard("{Enter}");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByText("open menu")).toHaveFocus();
  });

  it("gives a dialog a surviving return target when launched from a menu", async () => {
    const user = userEvent.setup();
    render(<>
      <Harness rows={[{ id: "rename", label: "Rename", run: async () => {
        await askText({ title: "Rename note", label: "Name", confirmLabel: "Save" });
      } }]} />
      <Dialog />
    </>);
    await user.click(screen.getByText("open menu"));
    await user.keyboard("{Enter}");
    await waitFor(() => expect(screen.getByRole("textbox")).toHaveFocus());
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByText("open menu")).toHaveFocus();
  });

  it("does not steal focus from a destination chosen by the action", async () => {
    const user = userEvent.setup();
    render(<>
      <Harness rows={[{ id: "go", label: "Go", run: () => {
        screen.getByRole("textbox").focus();
      } }]} />
      <input aria-label="Destination" />
    </>);
    await user.click(screen.getByText("open menu"));
    await user.keyboard("{Enter}");
    expect(screen.getByRole("textbox")).toHaveFocus();
  });
});
