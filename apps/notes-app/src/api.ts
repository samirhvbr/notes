// Typed wrappers over the Tauri commands.
//
// SCOPE §2.5: the frontend never touches the filesystem. Every read and write
// below is a call into the Rust core, which validates the resolved path against
// the workspace root on every operation. There is no fs capability granted to
// this webview — see src-tauri/capabilities/default.json.
import { invoke } from "@tauri-apps/api/core";

export type SessionKind = "wayland" | "x11" | "other" | "unknown";

/** What the process decided about its own environment at startup. */
export interface SpikeEnv {
  os: string;
  arch: string;
  tauriVersion: string;
  /** Linux display server, as reported by XDG_SESSION_TYPE / WAYLAND_DISPLAY. */
  session: SessionKind;
  /** Whether an NVIDIA kernel module was visible at startup. */
  nvidia: boolean;
  /** Whether WEBKIT_DISABLE_DMABUF_RENDERER was set by us, and why or why not. */
  dmabufWorkaround: string;
  /** Where per-workspace state is kept. Never inside the user's folder. */
  appDataDir: string;
}

export interface NoteEntry {
  name: string;
  relPath: string;
  size: number;
}

export interface WorkspaceInfo {
  root: string;
  /** True when this workspace came back from persisted state rather than a picker. */
  restored: boolean;
  entries: NoteEntry[];
}

export const spikeEnv = () => invoke<SpikeEnv>("spike_env");

/** Adopt a folder as the active workspace and persist the choice. */
export const openWorkspace = (path: string) =>
  invoke<WorkspaceInfo>("open_workspace", { path });

/**
 * Re-open the workspace persisted by a previous run. Returns null when there is
 * none. An error here is the 0.0 acceptance question on mobile, so it is
 * surfaced rather than swallowed.
 */
export const restoreWorkspace = () =>
  invoke<WorkspaceInfo | null>("restore_workspace");

export const readNote = (relPath: string) =>
  invoke<string>("read_note", { relPath });

export const writeNote = (relPath: string, contents: string) =>
  invoke<number>("write_note", { relPath, contents });
