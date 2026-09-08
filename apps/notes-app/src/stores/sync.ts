import { create } from "zustand";
import * as ipc from "../ipc";
import { useEditor } from "./editor";
import { useWorkspace } from "./workspace";

/**
 * The frontend half of reconciliation (docs/ARCHITECTURE.md §8).
 *
 * The core does the deciding — `stat`, then hash, then a verdict. This store
 * does two things the core cannot: it says **which notes are dirty**, because
 * the buffers live here, and it turns the resulting events into what the
 * interface shows.
 *
 * Two clocks, for the two cases §8 names:
 *
 * - a **tick** every 300 ms, which is a channel read when nothing has happened.
 *   With the watcher's own 200 ms debounce that keeps the 0.1b criterion —
 *   *"editar no VS Code com o app aberto atualiza a aba em <1s"* — with room to
 *   spare;
 * - a **full scan** on window focus, on a manual refresh, and every 5 s when
 *   there is no watcher at all (a network mount, a kernel out of inotify
 *   watches).
 */
const TICK_MS = 300;
const POLL_MS = 5000;

interface SyncState {
  /** Why the workspace is being polled instead of watched, when it is. */
  degraded: string | null;
  start: () => Promise<void>;
  stop: () => void;
  /** A full scan, now. */
  scan: () => Promise<void>;
}

let tick: ReturnType<typeof setInterval> | null = null;
let poll: ReturnType<typeof setInterval> | null = null;
let onFocus: (() => void) | null = null;
/** One reconciliation at a time: a slow scan must not queue up behind itself. */
let running = false;

function dirty(): ipc.NoteId[] {
  const doc = useEditor.getState().doc;
  return doc && doc.bufferVersion !== doc.savedVersion ? [doc.noteId] : [];
}

async function run(all: boolean) {
  if (running) return;
  running = true;
  try {
    const r = all ? await ipc.reconcileAll(dirty()) : await ipc.reconcileTick(dirty());
    for (const e of r.events) apply(e);
  } catch {
    // A workspace being closed mid-tick is the common case here, and it is not
    // something to show anybody.
  } finally {
    running = false;
  }
}

function apply(e: ipc.CoreEvent) {
  const ws = useWorkspace.getState();
  const ed = useEditor.getState();

  switch (e.event) {
    case "fs_changed": {
      // The sidebar re-lists the directory the change was in. One level, no
      // content read — the same call the tree makes when it expands.
      void ws.refresh(parentOf(e.path));
      if (e.kind === "modified" && ed.doc && e.note_id === ed.doc.noteId) {
        // Clean buffer plus an external change: reload, keeping the cursor.
        void ed.reloadFromDisk();
      }
      if (e.kind === "removed" && ed.doc && e.note_id === ed.doc.noteId) ed.close();
      break;
    }
    case "note_conflict": {
      if (!ed.doc || e.note_id !== ed.doc.noteId) break;
      // The core suspended autosave; the draft is this side's job, because the
      // buffer lives here (docs/DECISIONS-0.1a.md D-11).
      void ed.enterConflict();
      break;
    }
    case "note_moved": {
      void ws.refresh(parentOf(e.from));
      void ws.refresh(parentOf(e.to));
      ed.repath(e.from, e.to);
      break;
    }
    case "workspace_unavailable":
      ws.fail({ code: "unavailable", root: e.root, reason: e.reason });
      break;
    case "workspace_available":
      break;
    case "watch_degraded":
      useSync.setState({ degraded: e.reason });
      break;
  }
}

function parentOf(path: ipc.RelPath): ipc.RelPath {
  const i = path.lastIndexOf("/");
  return (i < 0 ? "" : path.slice(0, i)) as ipc.RelPath;
}

export const useSync = create<SyncState>((set, get) => ({
  degraded: null,

  async start() {
    get().stop();
    try {
      // `null` means the platform is watching. A reason means it is not, and
      // the reason is shown rather than swallowed — the inotify limit comes
      // back with the `sysctl` that raises it.
      set({ degraded: await ipc.watchStart() });
    } catch (e) {
      set({ degraded: ipc.asCoreError(e).code });
    }

    tick = setInterval(() => void run(false), TICK_MS);
    // The 5 s poll runs whether or not there is a watcher: a watch can be lost
    // without saying so, and a scan that finds nothing costs one directory
    // walk of a folder the user chose.
    poll = setInterval(() => void run(true), POLL_MS);
    onFocus = () => void run(true);
    window.addEventListener("focus", onFocus);
  },

  stop() {
    if (tick) clearInterval(tick);
    if (poll) clearInterval(poll);
    if (onFocus) window.removeEventListener("focus", onFocus);
    tick = poll = null;
    onFocus = null;
  },

  async scan() {
    await run(true);
  },
}));
