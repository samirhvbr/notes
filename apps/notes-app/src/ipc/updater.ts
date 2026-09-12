import { invoke } from "@tauri-apps/api/core";

export interface UpdateResult { supported: boolean; version: string | null; notes: string | null }
export const checkUpdate = () => invoke<UpdateResult>("update_check");
// Installation owns the input barrier, so it must not go through the wrapper
// that rejects other commands while that barrier is held.
export const installUpdate = () => invoke<void>("update_install");
