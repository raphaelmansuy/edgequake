/**
 * @module progress-event-normalizer
 * @description Single wire → domain adapter for WebSocket progress events (SPEC-149 LAW-149-6).
 *
 * Rust emits serde tagged `{ "type": "...", "data": { ... } }`.
 * Some legacy flat shapes are accepted for forward compatibility.
 */

import type {
  ChunkFailureEvent,
  ChunkProgressEvent,
  PdfPageProgressEvent,
  StageTransitionEvent,
  WebSocketProgressMessage,
} from "@/types/ingestion";

type RawRecord = Record<string, unknown>;

function asRecord(value: unknown): RawRecord | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as RawRecord)
    : null;
}

function num(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function str(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

/**
 * Normalize a parsed JSON WebSocket frame into a typed progress message.
 * Returns null when the frame cannot be recognized.
 */
export function normalizeProgressEvent(
  raw: unknown,
): WebSocketProgressMessage | null {
  const root = asRecord(raw);
  if (!root) return null;

  const type = str(root.type);
  if (!type) return null;

  const data = asRecord(root.data) ?? root;

  switch (type) {
    case "heartbeat":
    case "Heartbeat":
      return {
        type: type === "heartbeat" ? "heartbeat" : "Heartbeat",
        timestamp: str(data.timestamp) ?? new Date().toISOString(),
        server_time: str(data.server_time) ?? str(data.timestamp) ?? "",
      };

    case "Connected":
      return {
        type: "Connected",
        timestamp: str(data.timestamp) ?? new Date().toISOString(),
        server_version: str(data.server_version),
        // message lives in data on the wire; keep optional for consumers
      };

    case "SubscribedAck":
      return {
        type: "SubscribedAck",
        data: {
          accepted: Array.isArray(data.accepted)
            ? data.accepted.filter((v): v is string => typeof v === "string")
            : [],
          requested: num(data.requested) ?? 0,
        },
      };

    case "StatusSnapshot":
      return {
        type: "StatusSnapshot",
        timestamp: str(root.timestamp) ?? new Date().toISOString(),
        active_tasks: Array.isArray(data.active_tasks)
          ? (data.active_tasks as StatusSnapshotEventActive[])
          : [],
        // Preserve backend snapshot fields for debugging / future UI
        is_busy: data.is_busy as boolean | undefined,
        job_name: data.job_name as string | null | undefined,
        processed_documents: num(data.processed_documents),
        total_documents: num(data.total_documents),
      } as WebSocketProgressMessage;

    case "PdfPageProgress":
      return normalizePdfPageProgress(data);

    case "ChunkProgress":
      return {
        type: "ChunkProgress",
        data: {
          document_id: str(data.document_id) ?? "",
          task_id: str(data.task_id) ?? "",
          chunk_index: num(data.chunk_index) ?? 0,
          total_chunks: num(data.total_chunks) ?? 0,
          chunk_preview: str(data.chunk_preview) ?? "",
          time_ms: num(data.time_ms) ?? 0,
          eta_seconds: num(data.eta_seconds) ?? 0,
          tokens_in: num(data.tokens_in) ?? 0,
          tokens_out: num(data.tokens_out) ?? 0,
          cost_usd: num(data.cost_usd) ?? 0,
        },
      } satisfies ChunkProgressEvent;

    case "StageTransition":
      return {
        type: "StageTransition",
        data: {
          document_id: str(data.document_id) ?? "",
          task_id: str(data.task_id) ?? "",
          stage: str(data.stage) ?? "uploading",
          stage_message: str(data.stage_message) ?? "",
          stage_progress:
            data.stage_progress === null || data.stage_progress === undefined
              ? null
              : num(data.stage_progress),
        },
      } satisfies StageTransitionEvent;

    case "ChunkFailure":
      return {
        type: "ChunkFailure",
        data: {
          document_id: str(data.document_id) ?? "",
          task_id: str(data.task_id) ?? "",
          chunk_index: num(data.chunk_index) ?? 0,
          total_chunks: num(data.total_chunks) ?? 0,
          error_message: str(data.error_message) ?? "",
          was_timeout: Boolean(data.was_timeout),
          retry_attempts: num(data.retry_attempts) ?? 0,
        },
      } satisfies ChunkFailureEvent;

    case "GraphStorageProgress":
      return {
        type: "GraphStorageProgress",
        data: {
          track_id: str(data.track_id) ?? "",
          document_id: str(data.document_id) ?? "",
          sub_phase: str(data.sub_phase) ?? "",
          sub_phase_label: str(data.sub_phase_label) ?? "",
          entities_processed: num(data.entities_processed) ?? 0,
          entities_total: num(data.entities_total) ?? 0,
          entities_created: num(data.entities_created) ?? 0,
          entities_updated: num(data.entities_updated) ?? 0,
          relationships_processed: num(data.relationships_processed) ?? 0,
          relationships_total: num(data.relationships_total) ?? 0,
          relationships_created: num(data.relationships_created) ?? 0,
          relationships_updated: num(data.relationships_updated) ?? 0,
          elapsed_ms: num(data.elapsed_ms) ?? 0,
          eta_ms:
            data.eta_ms === null || data.eta_ms === undefined
              ? null
              : num(data.eta_ms) ?? null,
        },
      };

    case "ProgressSnapshot":
      return {
        type: "ProgressSnapshot",
        data,
      };

    case "DeletionStarted":
    case "DeletionPhase":
    case "DeletionCompleted":
    case "DeletionFailed":
    case "BulkDeletionStarted":
    case "BulkDeletionItemProgress":
    case "BulkDeletionCompleted":
    case "BulkDeletionFailed":
      return {
        type,
        data,
      } as WebSocketProgressMessage;

    case "ingestion_started":
    case "stage_started":
    case "stage_progress":
    case "stage_completed":
    case "ingestion_completed":
    case "ingestion_failed":
      return raw as WebSocketProgressMessage;

    case "Message":
      return {
        type: "Message",
        data: {
          level: str(data.level) ?? "info",
          message: str(data.message) ?? "",
          timestamp: str(data.timestamp) ?? new Date().toISOString(),
        },
      };

    default:
      return null;
  }
}

type StatusSnapshotEventActive = {
  track_id: string;
  document_id: string;
  status: string;
  progress: number;
};

function normalizePdfPageProgress(data: RawRecord): PdfPageProgressEvent {
  const pageNum = num(data.page_num) ?? num(data.current_page) ?? 0;
  const totalPages = num(data.total_pages) ?? 0;
  const explicitCompleted = num(data.completed_pages);
  const explicitProgress = num(data.progress);
  // Legacy backends without completed_pages: prefer progress-derived count,
  // then physical page_num. Never force 0 when progress exists.
  const completedPages =
    explicitCompleted ??
    (explicitProgress !== undefined && totalPages > 0
      ? Math.round(
          (explicitProgress > 1 ? explicitProgress / 100 : explicitProgress) *
            totalPages,
        )
      : pageNum);

  let progress = explicitProgress;
  if (progress === undefined) {
    progress =
      totalPages > 0
        ? Math.min(1, Math.max(0, completedPages / totalPages))
        : 0;
  } else if (progress > 1) {
    progress = progress / 100;
  }
  // Clamp at the normalization boundary so typed consumers never see >1 / <0.
  progress = Math.min(1, Math.max(0, progress));

  return {
    type: "PdfPageProgress",
    timestamp: new Date().toISOString(),
    data: {
      document_id:
        str(data.document_id) ?? str(data.pdf_id) ?? "",
      task_id: str(data.task_id) ?? "",
      current_page: pageNum,
      total_pages: totalPages,
      completed_pages: completedPages,
      progress,
      phase: str(data.phase),
      success: data.success as boolean | undefined,
      error: str(data.error) ?? undefined,
      markdown_len: num(data.markdown_len),
    },
  };
}
