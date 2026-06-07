import {
  isProcessingStatus,
  normalizeStatus,
} from "@/components/documents/status-badge";
import type { TaskResponse } from "@/types";

/** Pipeline phase counts derived from document statuses (SPEC-017 UI-P3-004). */
export interface PipelinePhaseCounts {
  pending: number;
  processing: number;
  completed: number;
  failed: number;
}

const EMPTY_PHASE_COUNTS: PipelinePhaseCounts = {
  pending: 0,
  processing: 0,
  completed: 0,
  failed: 0,
};

export function countDocumentsByPhase(
  statuses: Array<string | undefined | null>,
): PipelinePhaseCounts {
  if (statuses.length === 0) return { ...EMPTY_PHASE_COUNTS };

  return statuses.reduce<PipelinePhaseCounts>((acc, raw) => {
    const status = normalizeStatus(raw);

    if (isProcessingStatus(status)) {
      acc.processing += 1;
    } else if (status === "pending") {
      acc.pending += 1;
    } else if (status === "completed" || status === "indexed") {
      acc.completed += 1;
    } else if (status === "failed" || status === "cancelled") {
      acc.failed += 1;
    }
    return acc;
  }, { ...EMPTY_PHASE_COUNTS });
}

export function formatTaskType(taskType: string): string {
  return taskType
    .replace(/_/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function formatPipelineCost(cost: number): string {
  if (cost < 0.0001) return "< $0.0001";
  if (cost < 0.01) return `$${cost.toFixed(4)}`;
  return `$${cost.toFixed(3)}`;
}

export function formatDurationSeconds(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  if (seconds < 3600) {
    return `${Math.floor(seconds / 60)}m ${Math.round(seconds % 60)}s`;
  }
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

export function formatTokenCount(tokens: number): string {
  if (tokens < 1000) return tokens.toString();
  if (tokens < 1_000_000) return `${(tokens / 1000).toFixed(1)}K`;
  return `${(tokens / 1_000_000).toFixed(2)}M`;
}

export function formatThroughput(docsPerMin: number): string {
  if (docsPerMin < 0.1) return "< 0.1/min";
  if (docsPerMin < 1) return `${docsPerMin.toFixed(1)}/min`;
  return `${Math.round(docsPerMin)}/min`;
}

export function formatWaitTimeMs(waitMs: number): string {
  const seconds = Math.floor(waitMs / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) return `${minutes}m ${remainingSeconds}s`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
}

const UUID_PATTERN =
  /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi;

/** Replace UUIDs in pipeline log messages with human-readable document names. */
export function replaceUuidsInMessage(
  message: string,
  documentMap: Map<string, string>,
): string {
  return message.replace(UUID_PATTERN, (uuid) => {
    const docName = documentMap.get(uuid.toLowerCase());
    if (docName) {
      return docName.length > 30 ? `${docName.slice(0, 27)}...` : docName;
    }
    return `doc-${uuid.slice(0, 8)}`;
  });
}

export function buildDocumentNameMap(
  documents: Array<{
    id: string;
    title?: string | null;
    file_name?: string | null;
  }>,
): Map<string, string> {
  const map = new Map<string, string>();
  for (const doc of documents) {
    const displayName =
      doc.title || doc.file_name || `Document ${doc.id.slice(0, 8)}`;
    map.set(doc.id.toLowerCase(), displayName);
  }
  return map;
}

export function partitionTasksByStatus(tasks: TaskResponse[]): {
  pendingTasks: TaskResponse[];
  processingTasks: TaskResponse[];
} {
  const pendingTasks = tasks
    .filter((task) => task.status === "pending")
    .sort(
      (a, b) =>
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
    );

  const processingTasks = tasks
    .filter((task) => task.status === "processing")
    .sort(
      (a, b) =>
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
    );

  return { pendingTasks, processingTasks };
}
