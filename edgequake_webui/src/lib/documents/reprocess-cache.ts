/**
 * Shared reprocess cache helpers (SPEC-054 progress SSOT).
 *
 * WHY (DRY): Single-doc and bulk reprocess must bind the same optimistic fields
 * and progress key. Drift caused bulk panels to subscribe to batch `reprocess_*`
 * or a stale completed track_id → flash-then-dismiss.
 *
 * Provisional admit (kill dead zone):
 *   Before the slow POST /documents/reprocess returns, we seed
 *   `reprocess_pending_{documentId}` + pin processing fields so the 2s documents
 *   poll cannot restore Completed during graph cleanup.
 */

import type { ReprocessFailedResponse } from "@/lib/api/edgequake";
import { resolveProgressTrackId } from "@/lib/upload/progress-track-id";
import type { Document } from "@/types";
import type { QueryClient } from "@tanstack/react-query";

/** Fields applied at Confirm (matches backend early admit → cleaning). */
export const REPROCESS_OPTIMISTIC_FIELDS = {
  status: "processing",
  current_stage: "cleaning",
  stage_message: "Removing prior knowledge graph…",
  stage_progress: 0,
  error_message: undefined,
} as const;

/**
 * Fields after HTTP success binds a live task_id.
 * Cleanup already finished server-side → true worker-admission `queued`.
 */
export const REPROCESS_POST_ADMIT_FIELDS = {
  status: "processing",
  current_stage: "queued",
  stage_message: "Waiting for a free worker…",
  stage_progress: 0,
  error_message: undefined,
} as const;

type ReprocessPatchFields = {
  status: string;
  current_stage: string;
  stage_message: string;
  stage_progress: number;
  error_message?: undefined;
};

/** Client-only progress key while waiting for POST /documents/reprocess. */
export const REPROCESS_PENDING_PREFIX = "reprocess_pending_";

export function provisionalReprocessTrackId(documentId: string): string {
  return `${REPROCESS_PENDING_PREFIX}${documentId}`;
}

export function isProvisionalReprocessTrackId(
  trackId: string | null | undefined,
): boolean {
  return Boolean(trackId?.startsWith(REPROCESS_PENDING_PREFIX));
}

/** Server batch correlation id (`reprocess_YYYYMMDD_…`) — not a progress SSOT key. */
export function isReprocessBatchTrackId(
  trackId: string | null | undefined,
): boolean {
  if (!trackId?.startsWith("reprocess_")) return false;
  return !isProvisionalReprocessTrackId(trackId);
}

/**
 * True when the key is safe to pass to ProgressPanelRow / IngestionRunCard.
 * Provisional client keys and batch `reprocess_*` ids have no progress seed.
 */
export function isPollableReprocessProgressTrackId(
  trackId: string | null | undefined,
): boolean {
  if (!trackId?.trim()) return false;
  if (isProvisionalReprocessTrackId(trackId)) return false;
  if (isReprocessBatchTrackId(trackId)) return false;
  return true;
}

/**
 * Resolve the track id shown in ProgressPanelRow for a reprocess entry.
 *
 * While the entry is still provisional, keep Queuing UI even if the documents
 * poll already shows early-admit batch `reprocess_*`. Only prefer serverTrack
 * when it is a live task progress key (e.g. pdf_processing-…).
 */
export function resolveReprocessPanelTrackId(
  entryTrackId: string,
  serverTrackId: string | null | undefined,
): string {
  if (isProvisionalReprocessTrackId(entryTrackId)) {
    return entryTrackId;
  }
  if (isPollableReprocessProgressTrackId(serverTrackId)) {
    return serverTrackId as string;
  }
  return entryTrackId;
}

/** Static Queuing row: provisional client key or non-pollable batch reprocess_*. */
export function shouldShowReprocessQueuingPanel(
  trackId: string | null | undefined,
): boolean {
  return (
    isProvisionalReprocessTrackId(trackId) || isReprocessBatchTrackId(trackId)
  );
}

/**
 * Document ids whose session panel is still provisional Queuing (not cleaning).
 * ActiveRuns stays visible when stage is `cleaning` so the stepper narrates cleanup.
 */
export function documentIdsWithQueuingSession(
  entries: ReadonlyArray<{ documentId: string; trackId: string }>,
  stagesByDocId?: ReadonlyMap<string, string | null | undefined>,
): Set<string> {
  return new Set(
    entries
      .filter((e) => {
        if (!shouldShowReprocessQueuingPanel(e.trackId)) return false;
        const stage = (stagesByDocId?.get(e.documentId) || "").toLowerCase();
        // Prefer one honest narrative: show ActiveRuns during cleaning.
        if (stage === "cleaning") return false;
        return true;
      })
      .map((e) => e.documentId),
  );
}

/**
 * Exclude ActiveRuns for docs still showing a provisional Queuing session panel.
 * Cleaning-stage runs are kept (dual-UI: session row + ActiveRuns agree).
 */
export function filterRunsExcludingQueuingSession<
  T extends { documentId: string; stage?: string },
>(runs: ReadonlyArray<T>, queuingDocIds: ReadonlySet<string>): T[] {
  if (queuingDocIds.size === 0) return [...runs];
  return runs.filter((r) => {
    if (!queuingDocIds.has(r.documentId)) return true;
    return (r.stage || "").toLowerCase() === "cleaning";
  });
}

/** Stable toast id for admit-in-flight feedback. */
export function admitQueuingToastId(documentId: string): string {
  return `admit-queuing-${documentId}`;
}

/** Pinned fields that polls must not overwrite with terminal server status. */
export type ReprocessPinSnapshot = {
  status: string;
  current_stage: string;
  stage_message: string;
  stage_progress: number;
  track_id: string;
  error_message?: undefined;
};

const reprocessPins = new Map<string, ReprocessPinSnapshot>();

/** Full document shells for upload admit — re-injected if a poll drops the row. */
const pinnedDocumentShells = new Map<string, Document>();

/** Timers for deferred unpin after live task bind. */
const deferredUnpinTimers = new Map<string, ReturnType<typeof setTimeout>>();

function pinSnapshot(
  trackId: string,
  fields: ReprocessPatchFields = REPROCESS_OPTIMISTIC_FIELDS,
): ReprocessPinSnapshot {
  return {
    ...fields,
    track_id: trackId,
  };
}

/**
 * Pin an optimistic upload/reprocess document shell so list polls cannot drop
 * or terminal-overwrite the row until unpin / deferred unpin.
 * Preserves the shell's stage fields (uploads must not inherit reprocess cleaning).
 */
export function pinDocumentShell(doc: Document): void {
  const trackId =
    doc.track_id?.trim() || provisionalReprocessTrackId(doc.id);
  pinnedDocumentShells.set(doc.id, { ...doc, track_id: trackId });
  reprocessPins.set(doc.id, {
    status: String(doc.status || "processing"),
    current_stage: String(doc.current_stage || "uploading"),
    stage_message: String(doc.stage_message || ""),
    stage_progress:
      typeof doc.stage_progress === "number" ? doc.stage_progress : 0,
    track_id: trackId,
    error_message: undefined,
  });
}

/**
 * Pin document ids so list refetch cannot restore terminal status.
 * Pass a concrete track_id, or the pending prefix to derive per-doc provisional ids.
 */
export function pinReprocessDocuments(
  documentIds: string | Iterable<string>,
  trackId: string,
): void {
  const ids =
    typeof documentIds === "string"
      ? [documentIds]
      : Array.from(documentIds);
  const usePerDocProvisional =
    trackId === REPROCESS_PENDING_PREFIX ||
    (ids.length > 1 && isProvisionalReprocessTrackId(trackId));
  for (const id of ids) {
    const boundTrackId = usePerDocProvisional
      ? provisionalReprocessTrackId(id)
      : trackId;
    reprocessPins.set(id, pinSnapshot(boundTrackId));
  }
}

/** Update the pinned track_id after the API returns a real progress key. */
export function updateReprocessPinTrackId(
  documentId: string,
  trackId: string,
): void {
  const existing = reprocessPins.get(documentId);
  if (!existing) {
    reprocessPins.set(documentId, pinSnapshot(trackId));
    return;
  }
  reprocessPins.set(documentId, { ...existing, track_id: trackId });
}

export function unpinReprocessDocuments(
  documentIds: string | Iterable<string>,
): void {
  const ids =
    typeof documentIds === "string"
      ? [documentIds]
      : Array.from(documentIds);
  for (const id of ids) {
    reprocessPins.delete(id);
    pinnedDocumentShells.delete(id);
    const timer = deferredUnpinTimers.get(id);
    if (timer !== undefined) {
      clearTimeout(timer);
      deferredUnpinTimers.delete(id);
    }
  }
}

export function isReprocessPinned(documentId: string): boolean {
  return reprocessPins.has(documentId);
}

/** Test helper: clear all pins between cases. */
export function clearReprocessPinsForTests(): void {
  for (const timer of deferredUnpinTimers.values()) {
    clearTimeout(timer);
  }
  deferredUnpinTimers.clear();
  reprocessPins.clear();
  pinnedDocumentShells.clear();
}

const TERMINAL_STATUSES = new Set([
  "completed",
  "indexed",
  "failed",
  "cancelled",
  "partial_failure",
  "partial_success",
]);

function isTerminalStatus(status: string | undefined): boolean {
  return TERMINAL_STATUSES.has((status || "").toLowerCase());
}

/** Strip `staging:` so pin/list identity matches admit document_id. */
export function bareDocumentId(id: string): string {
  return id.startsWith("staging:") ? id.slice("staging:".length) : id;
}

/** True when a server list row already represents this pinned upload shell. */
export function serverRowCoversPinnedShell(
  doc: Document,
  pinnedId: string,
  shell: Document,
): boolean {
  if (doc.id === pinnedId) return true;
  if (doc.id === `staging:${pinnedId}`) return true;
  if (bareDocumentId(doc.id) === pinnedId) return true;
  const pinTrack = shell.track_id;
  return Boolean(pinTrack && doc.track_id === pinTrack);
}

type DocumentsQueryData = { items?: Document[] } | undefined;

/**
 * Re-apply pinned processing fields when a poll/refetch returns terminal status.
 * When the server is already non-terminal with a live task_id, drop the pin
 * (honest server state). Otherwise keep pin until deferred unpin / abort.
 */
export function protectPinnedDocumentsInQueryData<T extends DocumentsQueryData>(
  data: T,
): T {
  if (
    (!data?.items && pinnedDocumentShells.size === 0) ||
    (reprocessPins.size === 0 && pinnedDocumentShells.size === 0)
  ) {
    return data;
  }
  let changed = false;
  let items = (data?.items ?? []).map((doc) => {
    const pin = reprocessPins.get(doc.id);
    if (!pin) return doc;
    if (!isTerminalStatus(doc.status)) {
      if (isPollableReprocessProgressTrackId(doc.track_id)) {
        // Server honest with seeded progress key — safe to release pin early.
        reprocessPins.delete(doc.id);
        pinnedDocumentShells.delete(doc.id);
        return doc;
      }
      if (
        doc.track_id &&
        !isProvisionalReprocessTrackId(doc.track_id) &&
        pin.track_id !== doc.track_id
      ) {
        reprocessPins.set(doc.id, { ...pin, track_id: doc.track_id });
      }
      return doc;
    }
    changed = true;
    return {
      ...doc,
      status: pin.status,
      current_stage: pin.current_stage,
      stage_message: pin.stage_message,
      stage_progress: pin.stage_progress,
      track_id: pin.track_id,
      error_message: pin.error_message,
    };
  });

  // Re-inject upload/reprocess shells dropped by a stale poll.
  // Match bare id, staging:{id} alias, or same track_id (SPEC-086 dual-run).
  for (const [id, shell] of [...pinnedDocumentShells.entries()]) {
    if (items.some((d) => serverRowCoversPinnedShell(d, id, shell))) {
      pinnedDocumentShells.delete(id);
      reprocessPins.delete(id);
      changed = true;
      continue;
    }
    const pin = reprocessPins.get(id);
    const reinjected: Document = pin
      ? {
          ...shell,
          status: pin.status as Document["status"],
          current_stage: pin.current_stage,
          stage_message: pin.stage_message,
          stage_progress: pin.stage_progress,
          track_id: pin.track_id,
          error_message: pin.error_message,
        }
      : shell;
    items = [reinjected, ...items];
    changed = true;
  }

  if (!changed) return data;
  return { ...(data as object), items } as T;
}

/**
 * Resolve the progress/WS subscription key from a reprocess API response.
 * Prefer per-document task_id, then top-level task_id, never batch alone.
 */
export function resolveReprocessProgressTrackId(
  response: ReprocessFailedResponse,
  documentId: string,
): string {
  const perDocTaskId = response.document_task_ids?.find(
    (entry) => entry.document_id === documentId,
  )?.task_id;
  return (
    resolveProgressTrackId({
      task_id: perDocTaskId ?? response.task_id,
      track_id: response.track_id,
    }) ?? response.track_id
  );
}

/** Format skip_reasons for toast descriptions. */
export function formatReprocessSkipReasons(
  skipReasons: Record<string, number> | undefined,
): string {
  if (!skipReasons) return "";
  return Object.entries(skipReasons)
    .map(([reason, count]) => `${reason} (${count})`)
    .join(", ");
}

/**
 * Patch one or more documents in all `["documents"]` queries.
 * Default fields = early-admit cleaning; pass post-admit fields after HTTP success.
 * When `trackId` is set, also binds the progress SSOT key.
 */
export function patchDocumentsReprocessOptimistic(
  queryClient: QueryClient,
  documentIds: string | Iterable<string>,
  trackId?: string,
  fields: ReprocessPatchFields = REPROCESS_OPTIMISTIC_FIELDS,
): void {
  const ids =
    typeof documentIds === "string"
      ? new Set([documentIds])
      : new Set(documentIds);

  queryClient.setQueriesData(
    { queryKey: ["documents"] },
    (oldData: DocumentsQueryData) => {
      if (!oldData?.items) return oldData;
      return {
        ...oldData,
        items: oldData.items.map((doc: Document) =>
          ids.has(doc.id)
            ? {
                ...doc,
                ...fields,
                ...(trackId
                  ? {
                      track_id: isProvisionalReprocessTrackId(trackId)
                        ? provisionalReprocessTrackId(doc.id)
                        : trackId,
                    }
                  : {}),
              }
            : doc,
        ),
      };
    },
  );
}

/**
 * Sync admit: pin + optimistic processing + provisional track_id.
 * Call before any await so the first paint shows non-terminal feedback.
 */
export function beginProvisionalReprocess(
  queryClient: QueryClient,
  documentIds: string | Iterable<string>,
): Map<string, string> {
  const ids =
    typeof documentIds === "string"
      ? Array.from([documentIds])
      : Array.from(documentIds);
  const provisionalByDoc = new Map<string, string>();
  for (const id of ids) {
    const provisional = provisionalReprocessTrackId(id);
    provisionalByDoc.set(id, provisional);
    reprocessPins.set(id, pinSnapshot(provisional));
  }
  patchDocumentsReprocessOptimistic(
    queryClient,
    ids,
    REPROCESS_PENDING_PREFIX,
  );
  return provisionalByDoc;
}

/**
 * Restore one document row from a cancelQueries snapshot (skip / per-doc error).
 */
export function restoreDocumentFromSnapshots(
  queryClient: QueryClient,
  documentId: string,
  previousDocuments: ReadonlyArray<readonly [readonly unknown[], unknown]>,
): void {
  for (const [queryKey, data] of previousDocuments) {
    const prevItems = (data as DocumentsQueryData)?.items;
    const prevDoc = prevItems?.find((d) => d.id === documentId);
    if (!prevDoc) continue;
    queryClient.setQueryData(queryKey, (current: DocumentsQueryData) => {
      if (!current?.items) return current;
      return {
        ...current,
        items: current.items.map((d) => (d.id === documentId ? prevDoc : d)),
      };
    });
  }
}

/**
 * Abort an in-flight admit: unpin, restore list row, caller dismisses panel.
 */
export function abortProvisionalReprocess(
  queryClient: QueryClient,
  documentId: string,
  previousDocuments?: ReadonlyArray<readonly [readonly unknown[], unknown]>,
): void {
  unpinReprocessDocuments(documentId);
  if (previousDocuments) {
    restoreDocumentFromSnapshots(queryClient, documentId, previousDocuments);
  }
}

/** Delay used so list refetch does not clobber optimistic processing with stale data. */
export const REPROCESS_INVALIDATE_DELAY_MS = 2000;

/** Keep pin after bind until server is honest or this delay elapses. */
export function scheduleDeferredUnpin(documentId: string): void {
  const existing = deferredUnpinTimers.get(documentId);
  if (existing !== undefined) clearTimeout(existing);
  const timer = setTimeout(() => {
    deferredUnpinTimers.delete(documentId);
    unpinReprocessDocuments(documentId);
  }, REPROCESS_INVALIDATE_DELAY_MS);
  deferredUnpinTimers.set(documentId, timer);
}

/** Test helper: flush deferred unpin timers. */
export function clearDeferredUnpinTimersForTests(): void {
  for (const timer of deferredUnpinTimers.values()) {
    clearTimeout(timer);
  }
  deferredUnpinTimers.clear();
}

/**
 * Apply a successful reprocess response to the documents cache.
 * Returns the progress track id, or null when the doc was not requeued.
 *
 * Pin is kept until deferred unpin (or protectPinned sees honest server state)
 * so a stale Completed poll cannot flash terminal between bind and invalidate.
 */
export function applyReprocessSuccessToCache(
  queryClient: QueryClient,
  documentId: string,
  response: ReprocessFailedResponse,
): string | null {
  if ((response.requeued ?? 0) === 0) {
    return null;
  }
  const progressTrackId = resolveReprocessProgressTrackId(response, documentId);
  // Refresh pin to post-admit queued (cleanup finished) + live track_id.
  reprocessPins.set(documentId, {
    ...REPROCESS_POST_ADMIT_FIELDS,
    track_id: progressTrackId,
  });
  patchDocumentsReprocessOptimistic(
    queryClient,
    documentId,
    progressTrackId,
    REPROCESS_POST_ADMIT_FIELDS,
  );
  scheduleDeferredUnpin(documentId);
  return progressTrackId;
}

export function scheduleDocumentsInvalidate(queryClient: QueryClient): void {
  setTimeout(() => {
    queryClient.invalidateQueries({ queryKey: ["documents"] });
    queryClient.invalidateQueries({ queryKey: ["pipeline-status"] });
  }, REPROCESS_INVALIDATE_DELAY_MS);
}
