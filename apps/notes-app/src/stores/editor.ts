import { isSyncLocked } from "../ipc/barrier";
import { create } from "zustand";
import * as ipc from "../ipc";
import type {
  BaseRev, ConflictChoice, CoreError, DocStatus, DraftInfo, Eol, NoteId, OpenedNote, RelPath,
} from "../ipc";

/**
 * One open note.
 *
 * `bufferVersion` is monotonic per note and is the whole of the stale-save
 * guard: a save is only allowed to paint the tab clean when the version it
 * returns still equals the current one. An old save landing after new
 * keystrokes therefore cannot mark the buffer saved.
 */
export interface OpenDoc {
  noteId: NoteId;
  path: RelPath;
  text: string;
  baseRev: BaseRev;
  readOnly: OpenedNote["read_only"];
  bufferVersion: number;
  savedVersion: number;
  status: DocStatus;
  /** Bumped when the text was replaced from **outside** the editor — a reload
   *  after an external change. The editor watches it to swap the document
   *  while keeping the cursor and the undo history; ordinary typing never
   *  touches it. */
  externalRev: number;
  conflict: BaseRev | null;
  draft: DraftInfo | null;
  lastError: CoreError | null;
}

interface EditorState {
  doc: OpenDoc | null;
  autosaveMs: number;

  open: (path: RelPath) => Promise<void>;
  edit: (text: string) => void;
  save: (flush?: boolean) => Promise<void>;
  keepDraft: (reason: "stale" | "exit" | "conflict") => Promise<void>;
  resolveDraft: (restore: boolean) => Promise<void>;
  /** Keep mine · use the disk's · save as a copy. The core keeps whichever
   *  version this does not choose (docs/ARCHITECTURE.md §4.3). */
  resolveConflict: (choice: ConflictChoice) => Promise<void>;
  /** Rewrite this note's line endings, because the user asked. */
  convertEol: (eol: Eol) => Promise<void>;
  /** Take what is on disk into a **clean** buffer, keeping the cursor.
   *  Scope §12: *"disco mudou, buffer limpo → recarrega, preserva cursor."* */
  reloadFromDisk: () => Promise<void>;
  /** The core has suspended autosave for this note. Persist the buffer — the
   *  core cannot, because it does not hold it (DECISIONS-0.1a.md D-11). */
  enterConflict: () => Promise<void>;
  /** Follow a rename or a move the application performed.
   *
   *  The `NoteId` did not change — the core updated the registry directly and
   *  identity correlation was never involved — so the buffer, the cursor and
   *  the dirty state stay exactly as they were and only the path moves. That is
   *  scope §17's *"rename via app não reseta aba/cursor/id"* on this side of the
   *  IPC. */
  repath: (from: RelPath, to: RelPath) => void;
  close: () => void;
  setAutosave: (ms: number) => void;
}

let debounce: ReturnType<typeof setTimeout> | null = null;
/** At most one save per document is in flight (ARCHITECTURE.md §5). */
let inFlight = false;

function fromOpened(o: OpenedNote): OpenDoc {
  return {
    noteId: o.note_id,
    path: o.path,
    text: o.text,
    baseRev: o.base_rev,
    readOnly: o.read_only,
    bufferVersion: 0,
    savedVersion: 0,
    externalRev: 0,
    status: o.read_only ? "read_only" : "saved",
    conflict: null,
    draft: o.draft,
    lastError: null,
  };
}

export const useEditor = create<EditorState>((set, get) => ({
  doc: null,
  autosaveMs: 750,

  setAutosave: (ms) => set({ autosaveMs: ms }),

  async open(path) {
    if (debounce) clearTimeout(debounce);
    try {
      set({ doc: fromOpened(await ipc.noteOpen(path)) });
    } catch (e) {
      set((s) => ({ doc: s.doc && { ...s.doc, lastError: ipc.asCoreError(e), status: "error" } }));
      throw e;
    }
  },

  edit(text) {
    const doc = get().doc;
    if (isSyncLocked() || !doc || doc.readOnly) return;
    const next: OpenDoc = {
      ...doc,
      text,
      bufferVersion: doc.bufferVersion + 1,
      status: doc.conflict ? "conflict" : "pending",
    };
    set({ doc: next });

    if (debounce) clearTimeout(debounce);
    debounce = setTimeout(() => {
      // While a note is in conflict, autosave is suspended and the edits go to
      // the draft instead — never to the note (ARCHITECTURE.md §5).
      void (get().doc?.conflict ? get().keepDraft("conflict") : get().save());
    }, get().autosaveMs);
  },

  async save(flush = false) {
    const doc = get().doc;
    if (isSyncLocked() || !doc || doc.readOnly || doc.conflict || inFlight) return;
    if (doc.bufferVersion === doc.savedVersion && !flush) return;

    const sending = doc.bufferVersion;
    inFlight = true;
    set({ doc: { ...doc, status: "writing" } });
    try {
      const call = flush ? ipc.noteFlush : ipc.noteSave;
      const r = await call(doc.noteId, doc.text, sending, doc.baseRev);
      set((s) => {
        const d = s.doc;
        if (!d || d.noteId !== doc.noteId) return s;
        if (r.result === "saved") {
          // Clean only when the version that came back is still the current
          // one; otherwise the user has typed since and the tab stays dirty.
          const current = r.buffer_version === d.bufferVersion;
          return {
            doc: {
              ...d,
              baseRev: r.base_rev,
              savedVersion: r.buffer_version,
              status: current ? "saved" : "pending",
              lastError: null,
            },
          };
        }
        if (r.result === "conflict") {
          return { doc: { ...d, conflict: r.disk_rev, status: "conflict" } };
        }
        return {
          doc: { ...d, status: "error", lastError: { code: "io", op: "write", path: d.path, kind: r.kind } },
        };
      });
    } catch (e) {
      set((s) => ({ doc: s.doc && { ...s.doc, status: "error", lastError: ipc.asCoreError(e) } }));
    } finally {
      inFlight = false;
    }
  },

  async keepDraft(reason) {
    const doc = get().doc;
    if (!doc) return;
    try {
      const info = await ipc.draftWrite(doc.noteId, doc.text, doc.bufferVersion, doc.baseRev, reason);
      set((s) => ({ doc: s.doc && { ...s.doc, draft: info } }));
    } catch (e) {
      set((s) => ({ doc: s.doc && { ...s.doc, lastError: ipc.asCoreError(e) } }));
    }
  },

  async resolveDraft(restore) {
    const doc = get().doc;
    if (!doc) return;
    const opened = await ipc.draftResolve(doc.noteId, restore ? "restore" : "discard");
    set({ doc: { ...fromOpened(opened), bufferVersion: restore ? 1 : 0 } });
  },

  async resolveConflict(choice) {
    const doc = get().doc;
    if (!doc) return;
    if (debounce) clearTimeout(debounce);
    try {
      const opened = await ipc.conflictResolve(doc.noteId, doc.text, doc.baseRev, choice);
      set({ doc: fromOpened(opened) });
    } catch (e) {
      // `use_disk` on a note that was deleted externally is the user accepting
      // the deletion: the core has kept the buffer in `conflicts/` and there is
      // nothing left for the tab to show.
      if (ipc.asCoreError(e).code === "not_found") {
        set({ doc: null });
        return;
      }
      throw e;
    }
  },

  async convertEol(eol) {
    const doc = get().doc;
    if (!doc) return;
    if (debounce) clearTimeout(debounce);
    set({ doc: fromOpened(await ipc.noteConvertEol(doc.noteId, eol)) });
  },

  async reloadFromDisk() {
    const doc = get().doc;
    if (!doc) return;
    // Only a clean buffer is replaced. A dirty one is a conflict, and the core
    // has already said so by sending a different event.
    if (doc.bufferVersion !== doc.savedVersion) return;
    try {
      const fresh = await ipc.noteReload(doc.noteId);
      // A reload must not replace typing or navigation that happened during IPC.
      if (get().doc !== doc) return;
      set({
        doc: {
          ...fromOpened(fresh),
          externalRev: doc.externalRev + 1,
          draft: doc.draft,
        },
      });
    } catch (e) {
      set((s) => ({ doc: s.doc && { ...s.doc, lastError: ipc.asCoreError(e) } }));
    }
  },

  async enterConflict() {
    const doc = get().doc;
    if (!doc || doc.conflict) return;
    if (debounce) clearTimeout(debounce);
    set({ doc: { ...doc, conflict: doc.baseRev, status: "conflict" } });
    await get().keepDraft("conflict");
  },

  repath(from, to) {
    const doc = get().doc;
    if (!doc) return;
    if (doc.path === from) {
      set({ doc: { ...doc, path: to } });
      return;
    }
    // The note was inside a folder that moved.
    if (doc.path.startsWith(`${from}/`)) {
      set({ doc: { ...doc, path: (to + doc.path.slice(from.length)) as RelPath } });
    }
  },

  close() {
    const doc = get().doc;
    if (debounce) clearTimeout(debounce);
    if (doc) void ipc.noteClose(doc.noteId).catch(() => {});
    set({ doc: null });
  },
}));


/** Install a verified reload only into the exact frozen clean buffer. */
export function acceptSyncReload(before: OpenDoc | null, fresh: OpenedNote[]): boolean {
  if (useEditor.getState().doc !== before) return false;
  if (!before) return true;
  const note = fresh.find(n => n.note_id === before.noteId);
  if (!note || before.bufferVersion !== before.savedVersion) return false;
  useEditor.setState({ doc: { ...fromOpened(note), externalRev: before.externalRev + 1 } });
  return true;
}
