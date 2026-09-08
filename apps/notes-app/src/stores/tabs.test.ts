import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NoteId, RelPath, Session } from "../ipc";

/**
 * The frontend half of the 0.1c restart criterion.
 *
 * `notes-core`'s `tests/session.rs` proves the session survives a restart with
 * its tabs, active tab and cursors. This proves the store puts them back — and,
 * as importantly, that **leaving a tab does not lose a buffer**: a dirty note is
 * flushed and a note in conflict writes its draft, which is the rule
 * `ARCHITECTURE.md` §5 states and the one a tab strip is most likely to break.
 */

const saved: Session[] = [];
let stored: Session = blankSession();

vi.mock("../ipc", () => ({
  sessionGet: vi.fn(async () => structuredClone(stored)),
  sessionSave: vi.fn(async (s: Session) => {
    stored = structuredClone(s);
    saved.push(structuredClone(s));
  }),
  asCoreError: (e: unknown) => ({ code: "internal", message: String(e) }),
}));

async function defaultOpen(path: RelPath) {
  editor.doc = { noteId: idFor(path), path, bufferVersion: 0, savedVersion: 0, conflict: null };
}

const editor = {
  doc: null as null | {
    noteId: NoteId;
    path: RelPath;
    bufferVersion: number;
    savedVersion: number;
    conflict: unknown;
  },
  open: vi.fn(defaultOpen),
  close: vi.fn(() => {
    editor.doc = null;
  }),
  save: vi.fn(async () => {
    if (editor.doc) editor.doc.savedVersion = editor.doc.bufferVersion;
  }),
  keepDraft: vi.fn(async () => {}),
};

vi.mock("./editor", () => ({
  useEditor: { getState: () => editor },
}));

const { useTabs, pendingCursor } = await import("./tabs");

function blankSession(): Session {
  return {
    schema: 1,
    tabs: [],
    active_tab: null,
    view_mode: "source",
    sidebar_open: true,
    sidebar_width: 300,
  };
}

/** Deterministic identity per path, so a test can name one. */
function idFor(path: string): NoteId {
  return `id:${path}` as NoteId;
}

const p = (s: string) => s as RelPath;

beforeEach(() => {
  saved.length = 0;
  stored = blankSession();
  editor.doc = null;
  vi.clearAllMocks();
  // Two tests replace `open` deliberately; without restoring it here the
  // replacement leaks into every test that runs after them.
  editor.open = vi.fn(defaultOpen);
  useTabs.getState().reset();
});

describe("opening", () => {
  it("adds a tab and makes it active", async () => {
    await useTabs.getState().openPath(p("um.md"));
    const s = useTabs.getState();
    expect(s.tabs.map((t) => t.path)).toEqual(["um.md"]);
    expect(s.activeId).toBe(idFor("um.md"));
    expect(editor.open).toHaveBeenCalledWith("um.md");
  });

  it("does not open a second tab for a note already open", async () => {
    await useTabs.getState().openPath(p("um.md"));
    await useTabs.getState().openPath(p("dois.md"));
    await useTabs.getState().openPath(p("um.md"));
    expect(useTabs.getState().tabs).toHaveLength(2);
    expect(useTabs.getState().activeId).toBe(idFor("um.md"));
  });
});

describe("leaving a tab", () => {
  it("flushes a dirty note rather than losing the buffer", async () => {
    await useTabs.getState().openPath(p("um.md"));
    editor.doc!.bufferVersion = 3; // typed, not saved

    await useTabs.getState().openPath(p("dois.md"));
    expect(editor.save).toHaveBeenCalledWith(true);
    expect(editor.keepDraft).not.toHaveBeenCalled();
  });

  it("writes a draft instead of saving when the note is in conflict", async () => {
    await useTabs.getState().openPath(p("um.md"));
    editor.doc!.bufferVersion = 3;
    editor.doc!.conflict = { size: 1 };

    await useTabs.getState().openPath(p("dois.md"));
    // Autosave is suspended for a note in conflict: the buffer goes to the
    // draft, never to the note.
    expect(editor.keepDraft).toHaveBeenCalledWith("conflict");
    expect(editor.save).not.toHaveBeenCalled();
  });

  it("does nothing for a clean note", async () => {
    await useTabs.getState().openPath(p("um.md"));
    await useTabs.getState().openPath(p("dois.md"));
    expect(editor.save).not.toHaveBeenCalled();
    expect(editor.keepDraft).not.toHaveBeenCalled();
  });
});

describe("closing", () => {
  it("activates the neighbour to the right", async () => {
    for (const n of ["um.md", "dois.md", "tres.md"]) await useTabs.getState().openPath(p(n));
    await useTabs.getState().activate(idFor("dois.md"));

    await useTabs.getState().close(idFor("dois.md"));
    expect(useTabs.getState().tabs.map((t) => t.path)).toEqual(["um.md", "tres.md"]);
    expect(useTabs.getState().activeId).toBe(idFor("tres.md"));
  });

  it("falls back to the left when there is nothing to the right", async () => {
    for (const n of ["um.md", "dois.md"]) await useTabs.getState().openPath(p(n));
    await useTabs.getState().close(idFor("dois.md"));
    expect(useTabs.getState().activeId).toBe(idFor("um.md"));
  });

  it("closing the last tab closes the document", async () => {
    await useTabs.getState().openPath(p("um.md"));
    await useTabs.getState().close(idFor("um.md"));
    expect(useTabs.getState().tabs).toHaveLength(0);
    expect(useTabs.getState().activeId).toBeNull();
    expect(editor.close).toHaveBeenCalled();
  });

  it("closing an inactive tab leaves the active one alone", async () => {
    for (const n of ["um.md", "dois.md"]) await useTabs.getState().openPath(p(n));
    await useTabs.getState().close(idFor("um.md"));
    expect(useTabs.getState().activeId).toBe(idFor("dois.md"));
  });
});

describe("the cursor", () => {
  it("is remembered per tab and handed back when the editor asks", async () => {
    await useTabs.getState().openPath(p("um.md"));
    useTabs.getState().noteCursor(idFor("um.md"), 42, 7, 640);

    const tab = useTabs.getState().tabs[0];
    expect([tab.line, tab.col, tab.scrollTop]).toEqual([42, 7, 640]);
    expect(pendingCursor(idFor("um.md"))).toEqual({ line: 42, col: 7 });
  });

  it("is ignored while a restore is in flight", async () => {
    stored = {
      ...blankSession(),
      tabs: [
        { note_id: idFor("um.md"), path: p("um.md"), line: 9, col: 3, scroll_top: 90, pinned: false },
      ],
      active_tab: idFor("um.md"),
    };
    // A freshly mounted editor reports position 1:1 as it builds; that must not
    // overwrite the position being restored.
    editor.open = vi.fn(async (path: RelPath) => {
      useTabs.getState().noteCursor(idFor(path), 1, 1, 0);
      editor.doc = { noteId: idFor(path), path, bufferVersion: 0, savedVersion: 0, conflict: null };
    });

    await useTabs.getState().restore();
    expect(pendingCursor(idFor("um.md"))).toEqual({ line: 9, col: 3 });
  });
});

describe("persistence", () => {
  it("writes tabs and the active tab in the session's shape", async () => {
    await useTabs.getState().openPath(p("um.md"));
    useTabs.getState().noteCursor(idFor("um.md"), 5, 2, 100);
    await useTabs.getState().persist();

    const last = saved[saved.length - 1];
    expect(last.active_tab).toBe(idFor("um.md"));
    expect(last.tabs).toEqual([
      { note_id: idFor("um.md"), path: "um.md", line: 5, col: 2, scroll_top: 100, pinned: false },
    ]);
    // Everything else in the session belongs to other stores and is preserved.
    expect(last.view_mode).toBe("source");
    expect(last.sidebar_width).toBe(300);
  });

  it("restores tabs, the active tab and the cursor", async () => {
    stored = {
      ...blankSession(),
      tabs: [
        { note_id: idFor("um.md"), path: p("um.md"), line: 1, col: 1, scroll_top: 0, pinned: false },
        { note_id: idFor("dois.md"), path: p("dois.md"), line: 42, col: 7, scroll_top: 640, pinned: false },
      ],
      active_tab: idFor("dois.md"),
    };

    await useTabs.getState().restore();
    const s = useTabs.getState();
    expect(s.tabs.map((t) => t.path)).toEqual(["um.md", "dois.md"]);
    expect(s.activeId).toBe(idFor("dois.md"));
    expect(editor.open).toHaveBeenCalledWith("dois.md");
    expect(pendingCursor(idFor("dois.md"))).toEqual({ line: 42, col: 7 });
  });

  it("drops a tab whose note is gone rather than failing on every click", async () => {
    stored = {
      ...blankSession(),
      tabs: [
        { note_id: idFor("sumiu.md"), path: p("sumiu.md"), line: 1, col: 1, scroll_top: 0, pinned: false },
      ],
      active_tab: idFor("sumiu.md"),
    };
    editor.open = vi.fn(async () => {
      throw new Error("not found");
    });

    await useTabs.getState().restore();
    expect(useTabs.getState().tabs).toHaveLength(0);
    expect(useTabs.getState().activeId).toBeNull();
  });

  it("an empty session leaves no tabs", async () => {
    await useTabs.getState().restore();
    expect(useTabs.getState().tabs).toHaveLength(0);
  });
});

describe("following the tree", () => {
  it("a renamed note keeps its tab and its identity", async () => {
    await useTabs.getState().openPath(p("um.md"));
    useTabs.getState().repath(idFor("um.md"), p("pasta/um.md"));
    expect(useTabs.getState().tabs[0]).toMatchObject({
      noteId: idFor("um.md"),
      path: "pasta/um.md",
    });
    expect(useTabs.getState().activeId).toBe(idFor("um.md"));
  });

  it("a deleted note loses its tab", async () => {
    for (const n of ["um.md", "dois.md"]) await useTabs.getState().openPath(p(n));
    useTabs.getState().forget([idFor("dois.md")]);
    expect(useTabs.getState().tabs.map((t) => t.path)).toEqual(["um.md"]);
    expect(useTabs.getState().activeId).toBe(idFor("um.md"));
  });
});

describe("opening at a position", () => {
  it("opens a note that is not open and lands on the line", async () => {
    await useTabs.getState().openAt(p("um.md"), 42, 7);
    expect(editor.open).toHaveBeenCalledWith("um.md");
    expect(pendingCursor(idFor("um.md"))).toEqual({ line: 42, col: 7 });
  });

  it("moves the caret in a note that is already open", async () => {
    await useTabs.getState().openPath(p("um.md"));
    const before = useTabs.getState().gotoRev;

    await useTabs.getState().openAt(p("um.md"), 12, 3);
    expect(useTabs.getState().tabs).toHaveLength(1);
    expect(pendingCursor(idFor("um.md"))).toEqual({ line: 12, col: 3 });
    // The view will not rebuild for a note it already holds, so the editor is
    // told to move — this is what makes clicking a search hit land on the line.
    expect(useTabs.getState().gotoRev).toBeGreaterThan(before);
  });

  it("switches to another open tab and lands on the line there", async () => {
    for (const n of ["um.md", "dois.md"]) await useTabs.getState().openPath(p(n));
    await useTabs.getState().openAt(p("um.md"), 5, 1);
    expect(useTabs.getState().activeId).toBe(idFor("um.md"));
    expect(pendingCursor(idFor("um.md"))).toEqual({ line: 5, col: 1 });
  });
});
