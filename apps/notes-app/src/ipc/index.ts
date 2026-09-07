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
import type { Entry } from "./generated/Entry";
import type { NoteId } from "./generated/NoteId";
import type { OpenedNote } from "./generated/OpenedNote";
import type { RelPath } from "./generated/RelPath";
import type { SaveResult } from "./generated/SaveResult";
import type { Session } from "./generated/Session";
import type { Settings } from "./generated/Settings";
import type { WorkspaceInfo } from "./generated/WorkspaceInfo";
import type { WorkspaceEntry } from "./generated/WorkspaceEntry";

export type {
  BaseRev, CoreError, DocStatus, DraftChoice, DraftInfo, DraftReason, Entry,
  NoteId, OpenedNote, RelPath, SaveResult, Session, Settings, WorkspaceInfo,
  WorkspaceEntry,
};

/** Diagnostics, and the only shape here that is not generated. */
export interface EnvReport {
  os: string;
  arch: string;
  tauriVersion: string;
  session: string;
  nvidia: boolean;
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
export const noteCreate = (dir: RelPath, name: string) =>
  invoke<Entry>("note_create", { dir, name });
export const dirCreate = (dir: RelPath, name: string) =>
  invoke<Entry>("dir_create", { dir, name });

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

export const sessionGet = () => invoke<Session>("session_get");
export const sessionSave = (session: Session) => invoke<void>("session_save", { session });
export const settingsGet = () => invoke<Settings>("settings_get");
export const settingsSet = (settings: Settings) => invoke<void>("settings_set", { settings });
