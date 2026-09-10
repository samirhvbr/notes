// @vitest-environment jsdom
import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { Graph, KnowledgePanel, openWiki, WikiDialog } from "./Knowledge";
import * as ipc from "../ipc";
import { useTabs } from "../stores/tabs";
import { useWorkspace } from "../stores/workspace";
import { useEditor } from "../stores/editor";
vi.mock("../ipc", async (original) => ({
  ...(await original<typeof import("../ipc")>()),
  knowledgeGet: vi.fn(),
  metadataGet: vi.fn(async () => ({
    title: null,
    tags: [],
    properties: [],
    error: null,
  })),
  wikiCandidates: vi.fn(),
}));
beforeEach(() => {
  useWorkspace.setState({ info: { id: "workspace" } as ipc.WorkspaceInfo });
  useEditor.setState({ doc: null });
  useTabs.setState({ openPath: vi.fn(async () => {}) });
  vi.mocked(ipc.knowledgeGet).mockResolvedValue({
    notes: [
      { path: "a.md", tags: ["rust"] },
      { path: "b.md", tags: ["ideas"] },
    ],
    edges: [{ from: "a.md", to: "b.md" }],
    unresolved: [],
    partial: false,
  } as ipc.Knowledge);
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it("never opens an arbitrary homonym; the selected candidate opens", async () => {
  vi.mocked(ipc.wikiCandidates).mockResolvedValue([
    "a/Same.md",
    "b/Same.md",
  ] as ipc.RelPath[]);
  render(<WikiDialog />);
  await act(async () => {
    await openWiki("Same");
  });
  expect(useTabs.getState().openPath).not.toHaveBeenCalled();
  await userEvent.click(
    await screen.findByRole("button", { name: "b/Same.md" }),
  );
  expect(useTabs.getState().openPath).toHaveBeenCalledWith("b/Same.md");
});
it("tag filtering uses indexed tags rather than text matching", async () => {
  render(<KnowledgePanel mode="tags" />);
  await screen.findByRole("button", { name: "a.md" });
  await userEvent.selectOptions(screen.getByRole("combobox"), "rust");
  expect(screen.queryByRole("button", { name: "b.md" })).toBeNull();
});
it("graph exposes linked notes and keyboard navigation", async () => {
  const { container } = render(<Graph />);
  const node = await screen.findAllByRole("button", { name: "b.md" });
  expect(container.querySelectorAll("line")).toHaveLength(1);
  node[0].focus();
  await userEvent.keyboard("{Enter}");
  expect(useTabs.getState().openPath).toHaveBeenCalledWith("b.md");
});
