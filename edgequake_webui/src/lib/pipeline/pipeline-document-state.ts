/**
 * Classify document pipeline states for consistent banner + dialog messaging.
 *
 * WHY: "pending" means waiting (post-recovery or queued), not active LLM work.
 * Workers may be idle while documents still need processing.
 */

import {
  getDocumentDisplayStatus,
  isProcessingStatus,
  type DocumentStatus,
} from '@/components/documents/status-badge';
import type { Document } from '@/types';
import type { IngestionAlertMode } from './ingestion-alert-presenter';

export const WAITING_STATUSES: DocumentStatus[] = ['pending', 'queued', 'cleaning'];

export function isWaitingStatus(status: DocumentStatus): boolean {
  return WAITING_STATUSES.includes(status);
}

/** Active pipeline stage — worker should be doing real work. */
export function isActiveProcessingStatus(status: DocumentStatus): boolean {
  return isProcessingStatus(status) && !isWaitingStatus(status);
}

export interface PipelineDocumentSummary {
  activeCount: number;
  waitingCount: number;
  queuedCount: number;
  activeDocs: Document[];
  waitingDocs: Document[];
}

export function summarizePipelineDocuments(
  documents: Document[] | undefined,
  opts?: OrphanAdmissionOpts,
): PipelineDocumentSummary {
  const activeDocs: Document[] = [];
  const waitingDocs: Document[] = [];
  let queuedCount = 0;

  for (const doc of documents ?? []) {
    const status = getDocumentDisplayStatus(doc);
    // Orphan staging Uploading must not count as "Working" (SPEC-086 restart).
    if (isOrphanAdmissionShell(doc, Date.now(), opts)) {
      waitingDocs.push(doc);
      continue;
    }
    if (isActiveProcessingStatus(status)) {
      activeDocs.push(doc);
    } else if (isWaitingStatus(status)) {
      waitingDocs.push(doc);
      if (status === "queued") {
        queuedCount += 1;
      }
    }
  }

  return {
    activeCount: activeDocs.length,
    waitingCount: waitingDocs.length,
    queuedCount,
    activeDocs,
    waitingDocs,
  };
}

/** Tasks pending in queue not already represented by waiting or active documents. */
export function orphanQueuedTaskCount(
  pipelineQueuedTasks: number,
  waitingDocCount: number,
  activeDocCount = 0,
): number {
  return Math.max(0, pipelineQueuedTasks - waitingDocCount - activeDocCount);
}

/** Task counters from pipeline status APIs (basic or enhanced). */
export interface PipelineTaskStats {
  is_busy?: boolean;
  queued_tasks?: number;
  running_tasks?: number;
  pending_tasks?: number;
  processing_tasks?: number;
}

/** True when workers or the task queue can still pick up waiting documents. */
export function hasQueueCoverage(
  pipeline: PipelineTaskStats | undefined,
  pendingTaskCount: number,
  processingTaskCount: number,
): boolean {
  return (
    pendingTaskCount > 0 ||
    processingTaskCount > 0 ||
    Boolean(pipeline?.is_busy)
  );
}

/** Waiting documents with no worker/task scheduled (document ↔ task desync).
 *
 * SPEC-048: Do NOT treat a fresh upload as stuck. New docs often appear as
 * pending/queued for a few seconds before the task row is visible — that is
 * normal Queued, not "Needs attention".
 *
 * Stuck requires: no queue coverage AND (aged past grace OR recovery signal).
 *
 * SPEC-086 follow-up: staging admit shells can sit on `uploading` +
 * "Document received…" across restarts (orphan recovery used to skip
 * `staging:`). Those look "Working" because uploading is active — reclassify
 * as waiting/stuck when aged past grace.
 */
export const STUCK_GRACE_MS = 60_000;

const RECOVERY_STUCK_RE =
  /auto-recovered|no worker|needs?\s+reprocess|orphaned|server restart|please re-upload|upload interrupted/i;

const ADMISSION_SEED_RE = /document received,\s*starting processing/i;

export function isRecoveryStuckSignal(doc: Document): boolean {
  const msg = `${doc.stage_message || ""} ${doc.error_message || ""}`;
  return RECOVERY_STUCK_RE.test(msg);
}

/** How long the document has been waiting (ms). Unknown timestamps → aged. */
export function documentWaitAgeMs(doc: Document, now = Date.now()): number {
  const ts = doc.updated_at || doc.created_at;
  if (!ts) return Number.POSITIVE_INFINITY;
  const parsed = Date.parse(ts);
  if (Number.isNaN(parsed)) return Number.POSITIVE_INFINITY;
  return Math.max(0, now - parsed);
}

export type OrphanAdmissionOpts = {
  /**
   * When true, Insert may still be Pending behind busy workers — do not
   * treat aged uploading seed as Needs attention (SPEC-086 ops).
   */
  hasQueueCoverage?: boolean;
};

/**
 * Aged staging admit shell stuck on uploading with no real worker progress.
 * Fresh uploads stay under STUCK_GRACE_MS and are not flagged.
 *
 * Must NOT flag slow multipart uploads: require server admission signals
 * (`admission_staging` and/or seed copy), never progress alone.
 *
 * Must NOT false-orphan when `track_id` exists and the pipeline still has
 * queue coverage (Queued behind PDF convert).
 */
export function isOrphanAdmissionShell(
  doc: Document,
  now = Date.now(),
  opts?: OrphanAdmissionOpts,
): boolean {
  const status = getDocumentDisplayStatus(doc);
  if (
    status === "failed" ||
    status === "cancelled" ||
    status === "completed" ||
    status === "indexed" ||
    status === "partial_failure" ||
    status === "partial_success"
  ) {
    return false;
  }
  if (doc.failure_code === "server_restart_interrupted") {
    return true;
  }
  const stage = (doc.current_stage || status || "").toLowerCase();
  if (stage !== "uploading") {
    return false;
  }
  if (documentWaitAgeMs(doc, now) < STUCK_GRACE_MS) {
    return false;
  }
  const msg = `${doc.stage_message || ""} ${doc.error_message || ""}`;
  // Server admission shell only — client optimistic "Queued…" / slow HTTP must not match.
  const looksLikeAdmission =
    Boolean(doc.admission_staging) || ADMISSION_SEED_RE.test(msg);
  if (!looksLikeAdmission) {
    return false;
  }
  // Queued-behind-busy: keep Working/Queued, not Needs attention.
  if (doc.track_id && opts?.hasQueueCoverage) {
    return false;
  }
  return true;
}

/**
 * Failed / interrupted staging shells need re-upload, not Retry Failed / reprocess.
 */
export function needsReuploadNotReprocess(doc: Document): boolean {
  if (doc.failure_code === "server_restart_interrupted") {
    return true;
  }
  const msg = `${doc.stage_message || ""} ${doc.error_message || ""}`;
  if (/please re-upload|orphaned staging|upload interrupted/i.test(msg)) {
    return true;
  }
  const status = getDocumentDisplayStatus(doc);
  return Boolean(doc.admission_staging) && status === "failed";
}

export function detectStuckDocuments(
  summary: PipelineDocumentSummary,
  hasCoverage: boolean,
  now = Date.now(),
): Document[] {
  if (summary.waitingCount === 0) {
    return [];
  }
  const orphanOpts: OrphanAdmissionOpts = { hasQueueCoverage: hasCoverage };
  // Server-signaled orphans stay stuck even while other jobs run.
  // Age+seed alone must NOT force stuck when hasCoverage (queued-behind-busy).
  const forceStuck = summary.waitingDocs.filter(
    (doc) =>
      isRecoveryStuckSignal(doc) ||
      isOrphanAdmissionShell(doc, now, orphanOpts),
  );
  if (hasCoverage) {
    return forceStuck;
  }
  return summary.waitingDocs.filter((doc) => {
    if (
      isRecoveryStuckSignal(doc) ||
      isOrphanAdmissionShell(doc, now, orphanOpts)
    ) {
      return true;
    }
    // Fresh upload / just-queued: keep amber Queued, never red Needs attention
    if (documentWaitAgeMs(doc, now) < STUCK_GRACE_MS) {
      return false;
    }
    // SPEC-048: aged waiting with track_id still prefers Queued (task may lag
    // counters). Orphan staging Uploading is handled above.
    if (doc.track_id) {
      return false;
    }
    return true;
  });
}

/** Unified banner + dialog pipeline UI state (document truth + task queue). */
export interface PipelineUiState {
  activeDocCount: number;
  waitingDocCount: number;
  processingTaskCount: number;
  pendingTaskCount: number;
  isActivelyProcessing: boolean;
  /** @deprecated Prefer alertMode / isQueuedOnly — true when queued, not stuck */
  isWaitingOnly: boolean;
  isQueuedOnly: boolean;
  isStuck: boolean;
  stuckDocCount: number;
  stuckDocs: Document[];
  alertMode: IngestionAlertMode;
  showPipelineIndicator: boolean;
}

function resolveAlertMode(
  summary: PipelineDocumentSummary,
  isActivelyProcessing: boolean,
  isStuck: boolean,
  isQueuedOnly: boolean,
  stuckDocCount: number,
): IngestionAlertMode {
  // Real work + orphan staging shells → mixed (not pure Working).
  if (isActivelyProcessing && stuckDocCount > 0) {
    return "mixed";
  }
  if (isActivelyProcessing && summary.waitingCount > 0) {
    return "mixed";
  }
  if (isActivelyProcessing) {
    return "working";
  }
  if (isStuck) {
    return "stuck";
  }
  if (isQueuedOnly) {
    return "queued";
  }
  // No active / stuck / queued signal — caller must hide the indicator.
  return "queued";
}

/**
 * Single source for pipeline header, banner, and dialog modes.
 * Documents in pending/queued win over idle task statistics.
 */
export function resolvePipelineUiState(
  documents: Document[] | undefined,
  pipeline?: PipelineTaskStats,
): PipelineUiState {
  const pendingTaskCount =
    pipeline?.pending_tasks ?? pipeline?.queued_tasks ?? 0;
  const processingTaskCount =
    pipeline?.processing_tasks ?? pipeline?.running_tasks ?? 0;
  const queueCoverage = hasQueueCoverage(
    pipeline,
    pendingTaskCount,
    processingTaskCount,
  );
  const orphanOpts: OrphanAdmissionOpts = { hasQueueCoverage: queueCoverage };
  // Re-summarize with coverage so aged seed behind busy workers stays Working.
  const summary = summarizePipelineDocuments(documents, orphanOpts);

  const orphanQueued = orphanQueuedTaskCount(
    pendingTaskCount,
    summary.waitingCount,
    summary.activeCount,
  );
  const waitingDocCount = summary.waitingCount + orphanQueued;

  // First principle: "Processing N document(s)" requires document evidence when
  // the list is loaded and fully terminal. Stale `processing_tasks` / `is_busy`
  // must NOT keep the working banner after every doc is ingested.
  // Empty list still trusts task counters (docs may lag the queue briefly).
  const docsFullyIdle =
    (documents?.length ?? 0) > 0 &&
    summary.activeCount === 0 &&
    summary.waitingCount === 0;
  const isActivelyProcessing =
    summary.activeCount > 0 ||
    (processingTaskCount > 0 && !docsFullyIdle);

  const stuckDocs = detectStuckDocuments(summary, queueCoverage);
  const isStuck = !isActivelyProcessing && stuckDocs.length > 0;
  const isQueuedOnly =
    !isActivelyProcessing && waitingDocCount > 0 && !isStuck;

  const alertMode = resolveAlertMode(
    summary,
    isActivelyProcessing,
    isStuck,
    isQueuedOnly,
    stuckDocs.length,
  );

  // Prefer document count; fall back to running tasks when list lags the queue.
  // When fully idle, never surface a stale task count as "Processing N".
  const displayActiveCount = isActivelyProcessing
    ? summary.activeCount > 0
      ? summary.activeCount
      : processingTaskCount
    : 0;

  return {
    activeDocCount: displayActiveCount,
    waitingDocCount,
    processingTaskCount,
    pendingTaskCount,
    isActivelyProcessing,
    isWaitingOnly: isQueuedOnly,
    isQueuedOnly,
    isStuck,
    stuckDocCount: stuckDocs.length,
    stuckDocs,
    alertMode,
    showPipelineIndicator: isActivelyProcessing || isStuck || isQueuedOnly,
  };
}
