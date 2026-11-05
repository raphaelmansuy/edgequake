/**
 * @vitest-environment jsdom
 *
 * SPEC-183 follow-up — a dismissed text-only notice must survive a refresh, and must not
 * silence a workspace the person has not seen.
 */
import {
  dismissTextOnlyNotice,
  getDismissedTextOnlyNoticeWorkspaces,
  getServerTextOnlyNoticeDismissed,
  isTextOnlyNoticeDismissed,
  resetTextOnlyNoticeDismissals,
  subscribeTextOnlyNoticeDismiss,
  TEXT_ONLY_NOTICE_DISMISS_KEY,
} from "@/lib/documents/text-only-notice-dismiss";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

describe("text-only notice dismissal", () => {
  beforeEach(() => {
    localStorage.clear();
    resetTextOnlyNoticeDismissals();
  });

  afterEach(() => {
    localStorage.clear();
    resetTextOnlyNoticeDismissals();
    vi.restoreAllMocks();
  });

  it("remembers a dismissal under a prefixed key", () => {
    dismissTextOnlyNotice("ws-1");

    expect(isTextOnlyNoticeDismissed("ws-1")).toBe(true);
    expect(JSON.parse(localStorage.getItem(TEXT_ONLY_NOTICE_DISMISS_KEY) ?? "[]")).toEqual([
      "ws-1",
    ]);
    // BR0728 — every WebUI storage key is prefixed, so a scoped clear can find it.
    expect(TEXT_ONLY_NOTICE_DISMISS_KEY.startsWith("edgequake-")).toBe(true);
  });

  it("does not silence a workspace that was never dismissed", () => {
    dismissTextOnlyNotice("ws-1");

    expect(isTextOnlyNoticeDismissed("ws-2")).toBe(false);
    expect(isTextOnlyNoticeDismissed(null)).toBe(false);
    expect(isTextOnlyNoticeDismissed(undefined)).toBe(false);
    expect(isTextOnlyNoticeDismissed("  ")).toBe(false);
  });

  it("returns the same snapshot until something changes, so React does not loop", () => {
    const first = getDismissedTextOnlyNoticeWorkspaces();
    expect(getDismissedTextOnlyNoticeWorkspaces()).toBe(first);

    dismissTextOnlyNotice("ws-1");
    const second = getDismissedTextOnlyNoticeWorkspaces();
    expect(second).not.toBe(first);
    expect(second).toEqual(["ws-1"]);
  });

  it("keeps the server snapshot empty so no notice is hidden on a machine we cannot see", () => {
    dismissTextOnlyNotice("ws-1");

    expect(getServerTextOnlyNoticeDismissed()).toEqual([]);
  });

  it("notifies subscribers once per new dismissal", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeTextOnlyNoticeDismiss(listener);

    dismissTextOnlyNotice("ws-1");
    dismissTextOnlyNotice("ws-1");
    dismissTextOnlyNotice("ws-2");

    // The repeat is ignored: a second click on the same notice is not a state change.
    expect(listener).toHaveBeenCalledTimes(2);
    unsubscribe();
  });

  it("ignores a dismissal with no workspace, which could not be remembered", () => {
    dismissTextOnlyNotice("");
    dismissTextOnlyNotice(null);
    dismissTextOnlyNotice(undefined);

    expect(getDismissedTextOnlyNoticeWorkspaces()).toEqual([]);
    expect(localStorage.getItem(TEXT_ONLY_NOTICE_DISMISS_KEY)).toBeNull();
  });

  it("treats an unreadable payload as nothing dismissed", async () => {
    localStorage.setItem(TEXT_ONLY_NOTICE_DISMISS_KEY, "{not json");

    // WHY a fresh module: the store reads storage once per page load, which is exactly what a
    // refresh does. Testing the read path needs a new instance, not a second write.
    vi.resetModules();
    const fresh = await import("@/lib/documents/text-only-notice-dismiss");
    expect(fresh.getDismissedTextOnlyNoticeWorkspaces()).toEqual([]);
  });

  it("ignores non-string entries in a hand-edited payload", async () => {
    localStorage.setItem(TEXT_ONLY_NOTICE_DISMISS_KEY, JSON.stringify(["ws-1", 42, null, ""]));

    vi.resetModules();
    const fresh = await import("@/lib/documents/text-only-notice-dismiss");
    expect(fresh.getDismissedTextOnlyNoticeWorkspaces()).toEqual(["ws-1"]);
  });
});
