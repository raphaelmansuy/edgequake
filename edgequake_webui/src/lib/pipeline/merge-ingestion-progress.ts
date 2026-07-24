/**
 * SPEC-086 LAW-23: max(store, poll) by stage rank; terminal poll wins; seed never sticky.
 * Pure merge — no React/Zustand (SRP).
 */

import { SERVER_STAGE_ORDER, normalizeRunStage } from "@/lib/pipeline/ingestion-run-view";
import type { IngestionProgress, StageProgress } from "@/types/ingestion";

const TERMINAL = new Set(["completed", "failed", "cancelled"]);

export function stageRank(stage: string): number {
  const normalized = normalizeRunStage(stage, stage);
  const idx = SERVER_STAGE_ORDER.indexOf(normalized);
  return idx >= 0 ? idx : 0;
}

function isTerminal(status: string | undefined): boolean {
  return !!status && TERMINAL.has(status);
}

function progressScore(p: IngestionProgress): number {
  const current = p.progress?.current_stage;
  const stage = p.progress?.stages?.find((s) => s.stage === current);
  const pct =
    stage?.progress ??
    p.overall_progress ??
    p.progress?.completion_percentage ??
    0;
  return Number.isFinite(pct) ? pct : 0;
}

function mergeStages(
  storeStages: StageProgress[] | undefined,
  pollStages: StageProgress[] | undefined,
): StageProgress[] {
  if (!pollStages?.length) return storeStages ?? [];
  if (!storeStages?.length) return pollStages;
  const byStage = new Map(storeStages.map((s) => [s.stage, s]));
  for (const ps of pollStages) {
    const prev = byStage.get(ps.stage);
    if (!prev || (ps.progress ?? 0) >= (prev.progress ?? 0)) {
      byStage.set(ps.stage, { ...prev, ...ps });
    }
  }
  return Array.from(byStage.values());
}

function preferPollFields(
  store: IngestionProgress,
  poll: IngestionProgress,
): IngestionProgress {
  return {
    ...store,
    ...poll,
    progress: {
      ...store.progress,
      ...poll.progress,
      chunk_progress:
        poll.progress.chunk_progress ?? store.progress.chunk_progress,
      pdf_progress: poll.progress.pdf_progress ?? store.progress.pdf_progress,
      stages: mergeStages(store.progress.stages, poll.progress.stages),
      latest_message:
        poll.progress.latest_message || store.progress.latest_message,
      current_stage:
        poll.progress.current_stage || store.progress.current_stage,
      completion_percentage:
        poll.progress.completion_percentage ??
        store.progress.completion_percentage,
    },
  };
}

/**
 * Merge store + polled progress. Never leave seed as SSOT after an advanced poll.
 */
export function mergeIngestionProgress(
  store: IngestionProgress | null | undefined,
  poll: IngestionProgress | null | undefined,
): IngestionProgress | null {
  if (!store && !poll) return null;
  if (!store) return poll!;
  if (!poll) return store;

  const storeTerm = isTerminal(store.status);
  const pollTerm = isTerminal(poll.status);

  if (pollTerm && !storeTerm) return preferPollFields(store, poll);
  if (storeTerm && !pollTerm) return store;
  if (storeTerm && pollTerm) {
    return (poll.updated_at ?? "") >= (store.updated_at ?? "")
      ? preferPollFields(store, poll)
      : store;
  }

  const rPoll = stageRank(
    normalizeRunStage(poll.progress?.current_stage, poll.status),
  );
  const rStore = stageRank(
    normalizeRunStage(store.progress?.current_stage, store.status),
  );

  if (rPoll > rStore) return preferPollFields(store, poll);
  if (rPoll < rStore) return store;

  if (progressScore(poll) > progressScore(store)) {
    return preferPollFields(store, poll);
  }
  return store;
}
