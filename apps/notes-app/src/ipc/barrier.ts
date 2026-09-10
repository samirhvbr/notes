/** One synchronous gate shared by input, stores and IPC. No editor mutation is
 * admitted between its snapshot and the verified recovery reload. */
let locked = false;
let pending = 0;
let composing = false;
export const setComposing = (value: boolean) => { composing = value; };
const listeners = new Set<() => void>();
export const isSyncLocked = () => locked;
export const subscribeBarrier = (listener: () => void) => {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
};
export function beginSyncBarrier(): boolean {
  if (locked || pending || composing) return false;
  locked = true;
  listeners.forEach(f => f());
  return true;
}
export function endSyncBarrier() {
  locked = false;
  listeners.forEach(f => f());
}
export async function tracked<T>(call: () => Promise<T>): Promise<T> {
  if (locked) throw { code: "unsupported", cap: "sync application in progress" };
  pending++;
  try { return await call(); }
  finally {
    // Include callers' response continuations, not just the transport promise.
    setTimeout(() => { pending--; }, 0);
  }
}
