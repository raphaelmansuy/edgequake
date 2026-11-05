/**
 * SPEC-183 follow-up — per-workspace dismissal state for the dropzone's "text only" notice.
 *
 * Kept out of the component so the store stays testable without a DOM and the dropzone keeps
 * one job: rendering the upload zone.
 */
"use client";

import {
  dismissTextOnlyNotice,
  getDismissedTextOnlyNoticeWorkspaces,
  getServerTextOnlyNoticeDismissed,
  subscribeTextOnlyNoticeDismiss,
} from "@/lib/documents/text-only-notice-dismiss";
import { useCallback, useSyncExternalStore } from "react";

export interface TextOnlyNoticeDismissal {
  dismissed: boolean;
  dismiss: () => void;
}

export function useTextOnlyNoticeDismissed(
  workspaceId: string | null | undefined,
): TextOnlyNoticeDismissal {
  const dismissedIds = useSyncExternalStore(
    subscribeTextOnlyNoticeDismiss,
    getDismissedTextOnlyNoticeWorkspaces,
    getServerTextOnlyNoticeDismissed,
  );
  const id = workspaceId?.trim() ?? "";
  const dismiss = useCallback(() => dismissTextOnlyNotice(id), [id]);
  return { dismissed: id ? dismissedIds.includes(id) : false, dismiss };
}

export default useTextOnlyNoticeDismissed;
