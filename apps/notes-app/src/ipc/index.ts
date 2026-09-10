// The ONLY place `invoke` is called (docs/ARCHITECTURE.md §13).
//
// Every type crossing this boundary is generated from Rust by ts-rs into
// `./generated`; nothing here is hand-written, and a CI job fails when the
// committed files differ from a fresh generation. The frontend has no
// filesystem capability — every read and write below is a command.
import { invoke } from "@tauri-apps/api/core";

import type { BaseRev } from "./generated/BaseRev";
import type { CoreError } from "./generated/CoreError";
import type { DocStatus } from "./generated/DocStatus";
import type { DraftChoice } from "./generated/DraftChoice";
import type { DraftInfo } from "./generated/DraftInfo";
import type { DraftReason } from "./generated/DraftReason";
import type { ConflictChoice } from "./generated/ConflictChoice";
import type { ConflictSnapshot } from "./generated/ConflictSnapshot";
import type { Conflicts } from "./generated/Conflicts";
import type { ChangeKind } from "./generated/ChangeKind";
import type { ConflictKind } from "./generated/ConflictKind";
import type { CoreEvent } from "./generated/CoreEvent";
import type { DeleteKind } from "./generated/DeleteKind";
import type { Reconciled } from "./generated/Reconciled";
import type { Deleted } from "./generated/Deleted";
import type { Document } from "./generated/Document";
import type { Eol } from "./generated/Eol";
import type { Heading } from "./generated/Heading";
import type { Link } from "./generated/Link";
import type { LinkKind } from "./generated/LinkKind";
import type { Rendered } from "./generated/Rendered";
import type { Span } from "./generated/Span";
import type { Task } from "./generated/Task";
import type { Entry } from "./generated/Entry";
import type { NoteId } from "./generated/NoteId";
import type { OpenedNote } from "./generated/OpenedNote";
import type { RelPath } from "./generated/RelPath";
import type { SaveResult } from "./generated/SaveResult";
import type { QuickMatch } from "./generated/QuickMatch";
import type { QuickOpen } from "./generated/QuickOpen";
import type { WatchStatus } from "./generated/WatchStatus";
import type { SearchHit } from "./generated/SearchHit";
import type { SearchId } from "./generated/SearchId";
import type { SearchMode } from "./generated/SearchMode";
import type { SearchOpts } from "./generated/SearchOpts";
import type { SearchProgress } from "./generated/SearchProgress";
import type { Session } from "./generated/Session";
import type { Tab } from "./generated/Tab";
import type { Settings } from "./generated/Settings";
import type { WorkspaceInfo } from "./generated/WorkspaceInfo";
import type { WorkspaceEntry } from "./generated/WorkspaceEntry";

export type {
  BaseRev, ChangeKind, ConflictChoice, ConflictKind, ConflictSnapshot,
  Conflicts, CoreError, CoreEvent, DeleteKind, Deleted, DocStatus, DraftChoice,
  DraftInfo, DraftReason, Document, Entry, Eol, Heading, Link, LinkKind, NoteId,
  OpenedNote, QuickMatch, QuickOpen, Reconciled, RelPath, Rendered, SaveResult,
  SearchHit, SearchId, SearchMode, SearchOpts, SearchProgress, Session, Settings,
  Tab, Span, Task, WatchStatus, WorkspaceInfo, WorkspaceEntry,
};

/** Diagnostics, and the only shape here that is not generated. */
export interface EnvReport {
  os: string;
  arch: string;
  tauriVersion: string;
  session: string;
  /** The proprietary driver — the one the dmabuf workaround exists for. */
  nvidia: boolean;
  /** Reported, and deliberately not a trigger: nouveau's GBM works. */
  nouveau: boolean;
  dmabufApplied: boolean;
  dmabufExplanation: string;
  dataDir: string;
}

/** The workspace root, for listing. */
export const ROOT = "" as RelPath;

/**
 * A rejected command carries a `CoreError`, whose `code` is the contract.
 * Nothing in the UI reads a message — `code` maps to an i18n key.
 */
export function asCoreError(e: unknown): CoreError {
  if (e && typeof e === "object" && "code" in e) return e as CoreError;
  return { code: "internal", message: String(e) };
}

export const envReport = () => invoke<EnvReport>("env_report");

export const workspaceOpen = (root: string) =>
  invoke<WorkspaceInfo>("workspace_open", { root });
export const workspaceCreate = (parent: string, name: string) =>
  invoke<WorkspaceInfo>("workspace_create", { parent, name });
export const workspaceRestoreLast = () =>
  invoke<WorkspaceInfo | null>("workspace_restore_last");
export const workspaceRecent = () => invoke<WorkspaceEntry[]>("workspace_recent");
export const workspaceClose = (dirty: NoteId[]) =>
  invoke<void>("workspace_close", { dirty });

export const treeList = (dir: RelPath) => invoke<Entry[]>("tree_list", { dir });

export const noteOpen = (path: RelPath) => invoke<OpenedNote>("note_open", { path });
export const noteSave = (
  noteId: NoteId,
  text: string,
  bufferVersion: number,
  baseRev: BaseRev,
) => invoke<SaveResult>("note_save", { noteId, text, bufferVersion, baseRev });
export const noteFlush = (
  noteId: NoteId,
  text: string,
  bufferVersion: number,
  baseRev: BaseRev,
) => invoke<SaveResult>("note_flush", { noteId, text, bufferVersion, baseRev });
/** Re-read a note from disk. The caller decides when a buffer may be replaced. */
export const noteReload = (noteId: NoteId) =>
  invoke<OpenedNote>("note_reload", { noteId });

/** Forget the per-note state a closed tab no longer needs. Leaves the draft. */
export const noteClose = (noteId: NoteId) => invoke<void>("note_close", { noteId });

/** Rewrite a note's line endings, because the user asked. Keeps the old bytes. */
export const noteConvertEol = (noteId: NoteId, eol: Eol) =>
  invoke<OpenedNote>("note_convert_eol", { noteId, eol });

/**
 * Keep mine · use the disk's · save as a copy.
 *
 * **"Compare" is not a command.** It changes nothing on disk and reads two
 * strings this frontend is already holding, so it is a screen
 * (docs/ARCHITECTURE.md §17.1).
 */
export const conflictResolve = (
  noteId: NoteId,
  text: string,
  baseRev: BaseRev,
  choice: ConflictChoice,
) => invoke<OpenedNote>("conflict_resolve", { noteId, text, baseRev, choice });

/**
 * Open an `http(s)` URL in the operating system's browser.
 *
 * A link in a note never navigates the WebView and never reaches a shell: the
 * command checks the scheme again in Rust, and the capability file allows only
 * `http` and `https` (scope §8.4).
 */
export const shellOpen = (url: string) => invoke<void>("shell_open", { url });

/** Everything kept in `conflicts/`, and what it costs on disk. */
export const conflictList = () => invoke<Conflicts>("conflict_list");

export const noteCreate = (dir: RelPath, name: string) =>
  invoke<Entry>("note_create", { dir, name });
export const dirCreate = (dir: RelPath, name: string) =>
  invoke<Entry>("dir_create", { dir, name });

/**
 * Sanitized HTML for the buffer the editor is holding.
 *
 * The text is sent rather than read from disk because the preview follows what
 * is being typed. **This is the whole of the preview IR** — no AST crosses, and
 * there is no Markdown parser in this application's frontend
 * (docs/ARCHITECTURE.md §10). `Rendered.html` is safe to assign to `innerHTML`
 * for exactly that reason, and for no other.
 */
export const markdownRender = (path: RelPath, text: string) =>
  invoke<Rendered>("markdown_render", { path, text });

/** The outline, links and front-matter span, with no HTML rendered. */
export const markdownOutline = (text: string) =>
  invoke<Document>("markdown_outline", { text });

/** Turn raw HTML or remote images on for *this* workspace. */
export const markdownTrustSet = (
  rawHtml: boolean | null,
  remoteImages: boolean | null,
) => invoke<void>("markdown_trust_set", { rawHtml, remoteImages });

/**
 * Start watching the workspace. `null` means the platform is watching; a string
 * is why it is not, and is shown rather than swallowed — the inotify limit comes
 * back with the `sysctl` that raises it.
 */
export const watchStart = () => invoke<string | null>("watch_start");

/**
 * How much of the workspace the watcher has managed to cover.
 *
 * `watch_start` returns as soon as the root is watched; the rest of the tree is
 * walked on a background thread, so this is polled beside `reconcile_tick` to
 * keep the status bar honest while it fills (0.1b).
 */
export const watchStatus = () => invoke<WatchStatus>("watch_status");

/**
 * One watcher-driven tick. **The dirty list comes from here**, because the
 * buffers live in this process and the core does not hold them.
 */
export const reconcileTick = (dirty: NoteId[]) =>
  invoke<Reconciled>("reconcile_tick", { dirty });

/** A full scan: window focus, tab switch, manual refresh. */
export const reconcileAll = (dirty: NoteId[]) =>
  invoke<Reconciled>("reconcile_all", { dirty });

/** Rename in place, keeping the `NoteId` — the tab and its cursor survive. */
export const entryRename = (path: RelPath, newName: string) =>
  invoke<Entry>("entry_rename", { path, newName });

/** Move into another directory. A collision is `AlreadyExists`, naming it. */
export const entryMove = (path: RelPath, toDir: RelPath) =>
  invoke<Entry>("entry_move", { path, toDir });

/** Copy beside the original. Never overwrites; the copy is its own note. */
export const entryDuplicate = (path: RelPath) =>
  invoke<Entry>("entry_duplicate", { path });

/** Delete, and say whether it went to the bin (scope §7.7). */
export const entryDelete = (path: RelPath) => invoke<Deleted>("entry_delete", { path });

export const draftWrite = (
  noteId: NoteId,
  text: string,
  bufferVersion: number,
  baseRev: BaseRev,
  reason: DraftReason,
) => invoke<DraftInfo>("draft_write", { noteId, text, bufferVersion, baseRev, reason });
export const draftList = () => invoke<DraftInfo[]>("draft_list");
export const draftResolve = (noteId: NoteId, choice: DraftChoice) =>
  invoke<OpenedNote>("draft_resolve", { noteId, choice });

/** Fuzzy match over paths, from memory — safe on every keystroke (0.1c). */
export const quickOpen = (query: string, limit: number) =>
  invoke<QuickOpen>("quick_open", { query, limit });

/** Start a content scan. Starting one cancels the previous. */
export const searchStart = (query: string, opts: SearchOpts) =>
  invoke<SearchId>("search_start", { query, opts });

/** Drain the hits found since the last poll — the `reconcile_tick` shape. */
export const searchPoll = (id: SearchId) => invoke<SearchProgress>("search_poll", { id });

export const searchCancel = (id: SearchId) => invoke<void>("search_cancel", { id });

export const sessionGet = () => invoke<Session>("session_get");
export const sessionSave = (session: Session) => invoke<void>("session_save", { session });
export const settingsGet = () => invoke<Settings>("settings_get");
export const settingsSet = (settings: Settings) => invoke<void>("settings_set", { settings });

export type { IndexStatus } from "./generated/IndexStatus";
export type { RecentNote } from "./generated/RecentNote";
export const indexStart = (force = false) => invoke<import("./generated/IndexStatus").IndexStatus>("index_start", { force });
export const indexStatus = () => invoke<import("./generated/IndexStatus").IndexStatus>("index_status");
export const indexCancel = () => invoke<void>("index_cancel");
export const recentNotes = () => invoke<import("./generated/RecentNote").RecentNote[]>("recent_notes");

export type { ReferencePlan } from "./generated/ReferencePlan";
export const referencePreview = (from:RelPath,to:RelPath) => invoke<import("./generated/ReferencePlan").ReferencePlan>("reference_preview",{from,to});
export const referenceApply = (token:string,selected:number[]) => invoke<import("./generated/ReferenceResult").ReferenceResult>("reference_apply",{token,selected});

export type {Knowledge} from "./generated/Knowledge";
export type {Metadata} from "./generated/Metadata";
export const knowledgeGet=()=>invoke<import("./generated/Knowledge").Knowledge>("knowledge_get");
export const metadataGet=(text:string)=>invoke<import("./generated/Metadata").Metadata>("metadata_get",{text});
export const wikiCandidates=(target:string)=>invoke<RelPath[]>("wiki_candidates",{target});
export const attachmentImport=(note:RelPath,bytes:number[])=>invoke<import("./generated/Attachment").Attachment>("attachment_import",{note,bytes});
