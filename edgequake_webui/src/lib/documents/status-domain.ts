/**
 * Pure document-status domain.
 *
 * This module owns wire normalization and status classification. It deliberately
 * has no React, icon, storage, or task-presentation dependencies.
 */

export const DOCUMENT_STATUSES = [
  "cleaning",
  "deleting",
  "queued",
  "pending",
  "uploading",
  "converting",
  "preprocessing",
  "chunking",
  "embedding",
  "re_embedding",
  "storing",
  "projecting",
  "processing",
  "indexing",
  "extracting",
  "gleaning",
  "merging",
  "summarizing",
  "completed",
  "indexed",
  "partial_success",
  "failed",
  "partial_failure",
  "delete_failed",
  "cancelled",
  "stopping",
  "cancelling",
  "held",
  "dead_letter",
] as const;

export type DocumentStatus = (typeof DOCUMENT_STATUSES)[number];

const KNOWN_STATUSES = new Set<string>(DOCUMENT_STATUSES);
const PROCESSING_STATUSES = new Set<DocumentStatus>([
  "cleaning",
  "deleting",
  "queued",
  "pending",
  "processing",
  "uploading",
  "converting",
  "preprocessing",
  "chunking",
  "extracting",
  "gleaning",
  "merging",
  "summarizing",
  "embedding",
  "re_embedding",
  "storing",
  "projecting",
  "indexing",
  "stopping",
  "cancelling",
]);
const TERMINAL_STATUSES = new Set<DocumentStatus>([
  "completed",
  "indexed",
  "failed",
  "partial_failure",
  "partial_success",
  "delete_failed",
  "cancelled",
  "dead_letter",
]);

export interface DocumentStatusInput {
  track_id?: string | null;
  current_stage?: string | null;
  status?: string | null;
  display_status?: string | null;
  ui_phase?: string | null;
}

export interface DocumentStatusOptions {
  cancelRequested?: boolean;
}

export function normalizeStatus(
  status: string | undefined | null,
): DocumentStatus {
  if (!status) return "pending";
  const normalized = status.trim().toLowerCase();
  if (KNOWN_STATUSES.has(normalized)) return normalized as DocumentStatus;
  if (normalized.includes("process")) return "processing";
  return "pending";
}

export function isProcessingStatus(status: DocumentStatus): boolean {
  return PROCESSING_STATUSES.has(status);
}

export function isTerminalStatus(status: DocumentStatus): boolean {
  return TERMINAL_STATUSES.has(status);
}

/** Stable backend-compatible rank for monotonic run merges. */
export function documentStageRank(
  stage: string | null | undefined,
): number {
  switch (normalizeStatus(stage)) {
    case "cleaning":
    case "queued":
    case "pending":
    case "uploading":
    case "held":
      return 10;
    case "converting":
      return 20;
    case "processing":
    case "preprocessing":
      return 30;
    case "chunking":
      return 40;
    case "extracting":
      return 50;
    case "gleaning":
      return 60;
    case "merging":
      return 70;
    case "summarizing":
      return 80;
    case "embedding":
    case "re_embedding":
      return 90;
    case "storing":
    case "indexing":
      return 100;
    case "projecting":
      return 105;
    case "completed":
    case "indexed":
    case "partial_success":
    case "failed":
    case "partial_failure":
    case "delete_failed":
    case "cancelled":
    case "dead_letter":
      return 110;
    case "deleting":
      return 108;
    default:
      return 0;
  }
}

export function isWaitingDocumentStage(
  stage: string | null | undefined,
): boolean {
  return documentStageRank(stage) <= 10;
}

export function isActiveDocumentStage(
  stage: string | null | undefined,
): boolean {
  const rank = documentStageRank(stage);
  return rank > 10 && rank < 110;
}

/**
 * Resolve document-owned status only. Task `presentation.badge` is intentionally
 * excluded: held/dead-letter are task states and must not redefine a document.
 *
 * Authority (cancel dual-SSOT):
 * 1. cancelRequested / ui_phase=stopping → stopping
 * 2. Terminal status OR ui_phase=terminal OR terminal display_status → that terminal
 * 3. Else display_status (in-flight fine grain)
 * 4. Else current_stage / status
 */
export function getDocumentDisplayStatus(
  doc: DocumentStatusInput,
  options: DocumentStatusOptions = {},
): DocumentStatus {
  if (options.cancelRequested) return "stopping";
  if (doc.ui_phase?.toLowerCase() === "stopping") return "stopping";

  const legacy = normalizeStatus(doc.status);
  const display = doc.display_status
    ? normalizeStatus(doc.display_status)
    : null;
  const uiTerminal = doc.ui_phase?.toLowerCase() === "terminal";

  // Intentional terminals beat stale in-flight display_status (embedding lag).
  if (isTerminalStatus(legacy)) return legacy;
  if (uiTerminal && display && isTerminalStatus(display)) return display;
  if (uiTerminal && isTerminalStatus(legacy)) return legacy;
  if (display && isTerminalStatus(display)) return display;

  if (display) return display;
  return doc.current_stage ? normalizeStatus(doc.current_stage) : legacy;
}

/** Accept legacy percentages at boundaries, but expose only a finite 0..1 value. */
export function normalizeProgress01(
  value: number | null | undefined,
): number | undefined {
  if (typeof value !== "number" || !Number.isFinite(value)) return undefined;
  const fraction = value > 1 ? value / 100 : value;
  return Math.min(1, Math.max(0, fraction));
}

export function normalizeDocumentStageProgress<
  T extends { stage_progress?: number | null },
>(document: T): T {
  const normalized = normalizeProgress01(document.stage_progress);
  if (normalized === document.stage_progress) return document;
  if (normalized === undefined) {
    const { stage_progress: _invalid, ...rest } = document;
    return rest as T;
  }
  return { ...document, stage_progress: normalized };
}
