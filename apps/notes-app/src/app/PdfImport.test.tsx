// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { PdfImport } from "./PdfImport";

it("shows extracted text and writes only when the user saves", async () => {
  const user = userEvent.setup();
  const save = vi.fn(async () => {});
  render(<PdfImport name="report" text="plain extracted text" onCancel={vi.fn()} onSave={save} />);
  expect(screen.getByDisplayValue("plain extracted text")).toBeInTheDocument();
  await user.clear(screen.getByLabelText("Name"));
  await user.type(screen.getByLabelText("Name"), "review");
  await user.click(screen.getByRole("button", { name: "Save Markdown note" }));
  expect(save).toHaveBeenCalledWith("review", "plain extracted text");
});
