/**
 * SPEC-183 follow-up — a dismissed "text only" notice stays dismissed, per workspace.
 *
 * WHY this is not component state: the sentence is a standing fact about the workspace
 * (`embedding_modalities` is immutable), so re-showing it on every visit is the noise the
 * dismissal exists to stop. Remembering it per workspace means switching to a text-only
 * workspace you have not seen yet still explains itself, while the one you dismissed stays quiet.
 *
 * WHY `useSyncExternalStore` and not an effect: reading `localStorage` during render breaks the
 * server render, and reading it in an effect shows the notice for one frame and then removes it.
 * A snapshot read is correct on both sides, and the server snapshot is "nothing dismissed".
 */

/** Prefixed per BR0728 (every WebUI storage key starts with `edgequake-`). */
export const TEXT_ONLY_NOTICE_DISMISS_KEY = "edgequake-text-only-notice-dismissed";

/** Referentially stable empty snapshot: `useSyncExternalStore` re-renders forever otherwise. */
const EMPTY: readonly string[] = Object.freeze([]);

const listeners = new Set<() => void>();

let dismissedIds: readonly string[] = EMPTY;
let loaded = false;

function getStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    // Private mode or a blocked origin: the notice still dismisses for this session.
    return null;
  }
}

/** Load once per module instance; a corrupt payload is treated as "nothing dismissed". */
function ensureLoaded(): void {
  if (loaded) return;
  loaded = true;
  try {
    const raw = getStorage()?.getItem(TEXT_ONLY_NOTICE_DISMISS_KEY);
    if (!raw) return;
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return;
    const ids = parsed.filter((v): v is string => typeof v === "string" && v.length > 0);
    if (ids.length > 0) dismissedIds = Object.freeze(ids);
  } catch {
    /* unreadable storage or bad JSON — start empty rather than fail */
  }
}

function emit(): void {
  for (const listener of listeners) listener();
}

function persist(): void {
  try {
    getStorage()?.setItem(TEXT_ONLY_NOTICE_DISMISS_KEY, JSON.stringify(dismissedIds));
  } catch {
    /* quota / private mode — the in-memory snapshot still holds it for this session */
  }
}

export function subscribeTextOnlyNoticeDismiss(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Dismissed workspace ids. The same array instance is returned until something changes. */
export function getDismissedTextOnlyNoticeWorkspaces(): readonly string[] {
  ensureLoaded();
  return dismissedIds;
}

/** Server render: nothing was dismissed on a machine we cannot see. */
export function getServerTextOnlyNoticeDismissed(): readonly string[] {
  return EMPTY;
}

export function isTextOnlyNoticeDismissed(workspaceId: string | null | undefined): boolean {
  const id = workspaceId?.trim();
  if (!id) return false;
  return getDismissedTextOnlyNoticeWorkspaces().includes(id);
}

export function dismissTextOnlyNotice(workspaceId: string | null | undefined): void {
  const id = workspaceId?.trim();
  // No workspace means no key to remember it under. Dismissing would be forgotten on reload,
  // which is worse than not offering it.
  if (!id) return;
  ensureLoaded();
  if (dismissedIds.includes(id)) return;
  dismissedIds = Object.freeze([...dismissedIds, id]);
  persist();
  emit();
}

/** Test helper and the reset path for a cleared workspace list. */
export function resetTextOnlyNoticeDismissals(): void {
  dismissedIds = EMPTY;
  loaded = true;
  try {
    getStorage()?.removeItem(TEXT_ONLY_NOTICE_DISMISS_KEY);
  } catch {
    /* ignore */
  }
  emit();
}
