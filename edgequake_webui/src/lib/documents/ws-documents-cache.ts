/**
 * WebSocket → React Query documents cache helpers.
 *
 * WHY: During Ollama extraction, ChunkProgress fires very frequently. Full-list
 * invalidateQueries(['documents']) every ~400ms freezes /documents (main-thread
 * re-filter of up to 500 rows + API fan-out). Patch the matching row in cache
 * for stage updates; ignore high-frequency chunk ticks for list invalidation.
 */

import type { Document } from "@/types";
import type { QueryClient } from "@tanstack/react-query";
import { normalizeProgress01 } from "@/lib/documents/status-domain";

/** High-frequency events that must not trigger documents-list refetch. */
const LIST_NOISE_TYPES = new Set([
  "ChunkProgress",
  "ChunkFailure",
  "cost_update",
  "heartbeat",
  "Heartbeat",
  "Connected",
]);

/** Structural / terminal events that need a full list refresh (debounced). */
const FULL_INVALIDATE_TYPES = new Set([
  "ingestion_started",
  "ingestion_completed",
  "ingestion_failed",
  "StatusSnapshot",
  "stage_completed",
]);

/** Events that should patch row fields in the documents cache. */
const CACHE_PATCH_TYPES = new Set([
  "ingestion_started",
  "stage_started",
  "stage_progress",
  "stage_completed",
  "StageTransition",
  "ingestion_completed",
  "ingestion_failed",
  "PdfPageProgress",
  "StatusSnapshot",
]);

export interface ProgressCacheMessage {
  type?: string;
  track_id?: string;
  document_id?: string;
  stage?: string;
  progress?: number;
  message?: string;
  data?: {
    document_id?: string;
    task_id?: string;
    current_page?: number;
    total_pages?: number;
    completed_pages?: number;
    progress?: number;
    phase?: string;
    chunk_index?: number;
    total_chunks?: number;
    stage?: string;
    stage_message?: string;
    stage_progress?: number | null;
  };
}

export function isListNoiseProgressEvent(type: string | undefined): boolean {
  return type != null && LIST_NOISE_TYPES.has(type);
}

export function shouldPatchDocumentsCache(type: string | undefined): boolean {
  return type != null && CACHE_PATCH_TYPES.has(type);
}

export function shouldInvalidateDocumentsList(
  type: string | undefined,
): boolean {
  return type != null && FULL_INVALIDATE_TYPES.has(type);
}

function resolveTrackId(message: ProgressCacheMessage): string | undefined {
  return message.track_id || message.data?.task_id;
}

function resolveDocumentId(message: ProgressCacheMessage): string | undefined {
  return message.document_id || message.data?.document_id;
}

function humanizePdfPhase(phase: string | undefined): string {
  if (!phase) return "";
  switch (phase) {
    case "extraction":
    case "extracting":
    case "extracted":
    case "ocr":
    case "rendering":
      return "";
    case "extraction_error":
      return " — page error";
    case "resumed":
      return " — resumed";
    case "group_complete":
      return " — rendering";
    default:
      if (phase.startsWith("group_partial_")) return " — continuing";
      // Hide raw internal phase tokens from the documents list.
      if (/^[a-z0-9_]+$/i.test(phase) && phase.length < 24) return "";
      return ` — ${phase}`;
  }
}

function patchFieldsFromMessage(
  message: ProgressCacheMessage,
  existing?: Document,
): Partial<Document> {
  const type = message.type;
  const fields: Partial<Document> = {};

  // Terminal state is absorbing — late PdfPageProgress must not rewind it.
  const existingStatus = existing?.status;
  if (
    type === "PdfPageProgress" &&
    (existingStatus === "completed" || existingStatus === "failed")
  ) {
    return fields;
  }

  if (type === "PdfPageProgress" && message.data) {
    const { current_page, total_pages, completed_pages, progress, phase, task_id } =
      message.data;
    if (task_id) fields.track_id = task_id;
    fields.status = "processing";
    fields.current_stage = "converting";
    // SPEC-120: display_status must not remain "queued" or the badge ignores
    // current_stage (getDocumentDisplayStatus prefers display_status).
    fields.display_status = "converting";
    fields.ui_phase = "running";

    const done =
      typeof completed_pages === "number"
        ? completed_pages
        : typeof current_page === "number"
          ? current_page
          : undefined;

    // Reject decreasing completed_pages / progress vs cached values.
    const prevDone = existing?.stage_message?.match(/(\d+)\//)?.[1];
    const prevCompleted = prevDone ? Number(prevDone) : undefined;
    if (
      typeof done === "number" &&
      typeof prevCompleted === "number" &&
      done < prevCompleted &&
      existing?.current_stage === "converting"
    ) {
      return fields;
    }

    if (typeof progress === "number") {
      let normalized = normalizeProgress01(progress);
      if (typeof normalized === "number") {
        // Active converting never reports 100% — terminal event owns 1.0.
        if (normalized >= 1) normalized = 0.99;
        const prevProgress = existing?.stage_progress;
        if (
          typeof prevProgress === "number" &&
          normalized + 1e-9 < prevProgress &&
          existing?.current_stage === "converting"
        ) {
          // Keep prior monotonic progress; still allow message refresh below.
        } else {
          fields.stage_progress = normalized;
        }
      }
    }
    if (
      typeof done === "number" &&
      typeof total_pages === "number" &&
      total_pages > 0
    ) {
      const phaseLabel = humanizePdfPhase(phase);
      fields.stage_message = `Converting ${done}/${total_pages} pages${phaseLabel}`;
    }
    return fields;
  }

  const trackId = resolveTrackId(message);
  if (trackId) fields.track_id = trackId;

  if (type === "StageTransition" && message.data) {
    fields.status = "processing";
    fields.track_id = message.data.task_id ?? fields.track_id;
    if (message.data.stage) {
      fields.current_stage = message.data.stage;
      fields.display_status = message.data.stage;
      fields.ui_phase = "running";
    }
    if (message.data.stage_message) {
      fields.stage_message = message.data.stage_message;
    }
    if (typeof message.data.stage_progress === "number") {
      let normalized = normalizeProgress01(message.data.stage_progress);
      if (typeof normalized === "number") {
        if (normalized >= 1) normalized = 0.99;
        fields.stage_progress = normalized;
      }
    }
    return fields;
  }

  if (type === "ingestion_started") {
    fields.status = "processing";
    if (message.stage) fields.current_stage = message.stage;
  } else if (type === "stage_started" || type === "stage_progress") {
    fields.status = "processing";
    if (message.stage) fields.current_stage = message.stage;
    if (typeof message.progress === "number") {
      let normalized = normalizeProgress01(message.progress);
      if (typeof normalized === "number") {
        if (normalized >= 1) normalized = 0.99;
        fields.stage_progress = normalized;
      }
    }
    if (message.message) fields.stage_message = message.message;
  } else if (type === "stage_completed") {
    fields.status = "processing";
    if (message.stage) fields.current_stage = message.stage;
    // Stage complete within an active ingestion is still active overall.
    fields.stage_progress = 0.99;
  } else if (type === "ingestion_completed") {
    fields.status = "completed";
    fields.current_stage = undefined;
    fields.stage_progress = 1;
    fields.stage_message = "Completed";
  } else if (type === "ingestion_failed") {
    fields.status = "failed";
    if (message.stage) fields.current_stage = message.stage;
    if (message.message) fields.stage_message = message.message;
  }

  return fields;
}

function documentMatchesMessage(
  doc: Document,
  message: ProgressCacheMessage,
): boolean {
  const trackId = resolveTrackId(message);
  const documentId = resolveDocumentId(message);
  if (trackId && doc.track_id === trackId) return true;
  if (documentId && doc.id === documentId) return true;
  return false;
}

type DocumentsQueryData = {
  items: Document[];
  [key: string]: unknown;
};

function applyPatchToDocumentsQueries(
  queryClient: QueryClient,
  message: ProgressCacheMessage,
  patchFor: (doc: Document) => Partial<Document>,
): number {
  let patched = 0;
  const queries = queryClient.getQueriesData<DocumentsQueryData>({
    queryKey: ["documents"],
  });

  for (const [key, data] of queries) {
    if (!data?.items?.length) continue;
    let changed = false;
    const items = data.items.map((doc) => {
      if (!documentMatchesMessage(doc, message)) return doc;
      const patch = patchFor(doc);
      if (Object.keys(patch).length === 0) return doc;
      changed = true;
      patched += 1;
      return { ...doc, ...patch };
    });
    if (changed) {
      queryClient.setQueryData(key, { ...data, items });
    }
  }
  return patched;
}

/**
 * Patch matching documents in all cached `['documents', ...]` queries.
 * Returns number of rows patched.
 */
export function patchDocumentsCacheFromProgress(
  queryClient: QueryClient,
  message: ProgressCacheMessage,
): number {
  if (!shouldPatchDocumentsCache(message.type)) return 0;

  if (message.type === "StatusSnapshot") {
    const snapshot = message as ProgressCacheMessage & {
      active_tasks?: Array<{
        track_id: string;
        document_id: string;
        status: string;
        progress: number;
      }>;
    };
    let patched = 0;
    for (const task of snapshot.active_tasks ?? []) {
      patched += applyPatchToDocumentsQueries(
        queryClient,
        {
          type: "stage_progress",
          track_id: task.track_id,
          document_id: task.document_id,
        },
        () => {
          let progress = normalizeProgress01(task.progress) ?? task.progress;
          if (typeof progress === "number" && progress >= 1) progress = 0.99;
          return {
            track_id: task.track_id,
            status: "processing",
            current_stage: "processing",
            stage_progress: progress,
            stage_message: task.status,
          };
        },
      );
    }
    return patched;
  }

  return applyPatchToDocumentsQueries(queryClient, message, (doc) =>
    patchFieldsFromMessage(message, doc),
  );
}

/** Debounce window for rare full-list invalidation (ms). */
export const DOCUMENTS_INVALIDATE_DEBOUNCE_MS = 1500;

/** Safety-net full list refetch while ingest is active (ms). */
export const DOCUMENTS_SAFETY_NET_INVALIDATE_MS = 5000;
