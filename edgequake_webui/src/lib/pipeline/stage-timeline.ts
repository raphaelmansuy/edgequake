/**
 * SPEC-048: Stage timeline SSOT — per-step status + detail progress.
 *
 * Projects IngestionRunView → ordered steps with:
 *   pending | active | done | skipped | failed
 *
 * Edge cases covered:
 * - Non-PDF: converting skipped
 * - mode=merge: stages before merging skipped
 * - mode=entities: uploading/converting skipped
 * - gleaning / summarizing / preprocessing: first-class steps
 * - queued admission: no server step active
 * - failed: failed step marked; prior done; later pending
 * - completed: all applicable steps done
 */

import {
  SERVER_STAGE_ORDER,
  stageDisplayName,
  type IngestionCountUnit,
  type IngestionRunMode,
  type IngestionRunStage,
  type IngestionRunView,
} from "./ingestion-run-view";

export type StageStepStatus =
  | "pending"
  | "active"
  | "done"
  | "skipped"
  | "failed";

export interface StageStepDetail {
  current?: number;
  total?: number;
  unit?: IngestionCountUnit | string;
  progress01?: number;
  message?: string;
}

export interface StageTimelineStep {
  id: IngestionRunStage;
  label: string;
  status: StageStepStatus;
  detail?: StageStepDetail;
}

export type AdmissionPhase = "cleaning" | "queued" | null;

export interface StageTimeline {
  steps: StageTimelineStep[];
  activeStepId: IngestionRunStage | null;
  /** True when waiting for a free worker (post-cleanup admission). */
  admissionQueued: boolean;
  /** True while sync-in-HTTP graph cleanup is in flight. */
  admissionCleaning: boolean;
  /** Unified admission phase for presenters (cleaning | queued | null). */
  admissionPhase: AdmissionPhase;
  /**
   * Weighted overall estimate 0–1 (FP-03/04/10).
   * Never 1.0 until terminal completed; admission → 0.
   */
  overallProgress01: number;
  /** True when overall % is an estimate (always, except terminal). */
  overallIsEstimate: boolean;
  /** Within-stage determinate fraction when counts/progress known. */
  stageProgress01?: number;
  /** Prefer showing N/M for the active stage over inventing %. */
  stageCountsLabel?: string;
}

/**
 * Relative wall-clock weights (FP-10: longer stages need more bar share).
 * Extracting + embedding dominate real ingest time.
 */
export const STAGE_WEIGHTS: Record<string, number> = {
  uploading: 1,
  converting: 4,
  preprocessing: 1,
  chunking: 2,
  extracting: 28,
  gleaning: 8,
  merging: 12,
  summarizing: 6,
  embedding: 18,
  storing: 5,
};

function stageWeight(id: string): number {
  return STAGE_WEIGHTS[id] ?? 1;
}

/** Clamp in-stage fraction: never claim a stage is 100% until we leave it. */
export function clampStageFraction(frac: number | undefined): number {
  if (typeof frac !== "number" || !Number.isFinite(frac) || frac <= 0) {
    return 0.02; // stage started, unknown depth
  }
  return Math.min(0.99, Math.max(0, frac));
}

function resolveActiveFraction(detail?: StageStepDetail): number {
  if (
    typeof detail?.current === "number" &&
    typeof detail?.total === "number" &&
    detail.total > 0
  ) {
    return clampStageFraction(detail.current / detail.total);
  }
  if (typeof detail?.progress01 === "number") {
    return clampStageFraction(detail.progress01);
  }
  return 0.02;
}

/**
 * First-principles overall progress:
 *   Σ (weight_i × progress_i) / Σ weight_i
 * where progress_i ∈ {0 pending, frac active, 1 done}, skipped excluded.
 * Cap at 0.99 until the run is actually completed.
 */
export function computeWeightedOverallProgress(
  steps: StageTimelineStep[],
  options: {
    isComplete: boolean;
    admissionQueued: boolean;
    admissionCleaning?: boolean;
  },
): { overall01: number; stageProgress01?: number; stageCountsLabel?: string } {
  if (options.isComplete) {
    return { overall01: 1 };
  }
  if (options.admissionQueued || options.admissionCleaning) {
    return { overall01: 0 };
  }

  let weightSum = 0;
  let progressSum = 0;
  let stageProgress01: number | undefined;
  let stageCountsLabel: string | undefined;

  for (const step of steps) {
    if (step.status === "skipped" || step.id === "completed") continue;
    const w = stageWeight(step.id);
    weightSum += w;
    if (step.status === "done") {
      progressSum += w;
    } else if (step.status === "active" || step.status === "failed") {
      const frac = resolveActiveFraction(step.detail);
      progressSum += w * frac;
      stageProgress01 = frac;
      stageCountsLabel = formatStepDetailLine(step.detail) ?? undefined;
    }
  }

  if (weightSum <= 0) {
    return { overall01: 0 };
  }

  // Never paint 100% before terminal complete (trust axiom)
  const overall01 = Math.min(0.99, progressSum / weightSum);
  return { overall01, stageProgress01, stageCountsLabel };
}

/** Processing stages in wire order (excludes admission queued + terminal). */
export const PROCESSING_STAGES: IngestionRunStage[] = [
  "uploading",
  "converting",
  "preprocessing",
  "chunking",
  "extracting",
  "gleaning",
  "merging",
  "summarizing",
  "embedding",
  "storing",
];

const STAGE_RANK: Record<string, number> = Object.fromEntries(
  SERVER_STAGE_ORDER.map((s, i) => [s, i]),
);

function rank(stage: string): number {
  return STAGE_RANK[stage.toLowerCase()] ?? -1;
}

/** Expected count unit for a stage (honest empty when unknown). */
export function expectedUnitForStage(
  stage: string,
): IngestionCountUnit | undefined {
  switch (stage.toLowerCase()) {
    case "converting":
    case "preprocessing":
      return "pages";
    case "chunking":
    case "extracting":
    case "gleaning":
    case "embedding":
      return "chunks";
    case "merging":
    case "summarizing":
      return "entities";
    case "storing":
      return "relationships";
    default:
      return undefined;
  }
}

function shouldSkipConverting(
  sourceType: IngestionRunView["sourceType"],
): boolean {
  return sourceType !== "pdf";
}

function shouldSkipForMode(
  step: IngestionRunStage,
  mode: IngestionRunMode | undefined,
): boolean {
  if (!mode || mode === "full") return false;
  const r = rank(step);
  if (mode === "merge") {
    // Merge-only reuses graph snapshot → jump to merging
    return r >= 0 && r < rank("merging");
  }
  if (mode === "entities") {
    // Entities-only reuses markdown → skip upload/convert
    return step === "uploading" || step === "converting";
  }
  return false;
}

function applicableSteps(run: IngestionRunView): IngestionRunStage[] {
  return PROCESSING_STAGES.filter((step) => {
    // SPEC-086 ops: omit converting entirely for non-PDF (no "Converting PDF").
    if (step === "converting" && shouldSkipConverting(run.sourceType)) {
      return false;
    }
    return true;
  }).concat(["completed"]);
}

function formatDetail(run: IngestionRunView): StageStepDetail | undefined {
  const hasCounts = Boolean(run.counts);
  const hasProgress = typeof run.progress01 === "number";
  const stageLabel = stageDisplayName(run.stage, run.sourceType);
  const hasMessage =
    Boolean(run.message) &&
    run.message !== stageLabel &&
    !run.message.toLowerCase().startsWith(stageLabel.toLowerCase());

  if (!hasCounts && !hasProgress && !hasMessage) {
    // Still expose expected unit so UI can show "… chunks" indeterminate hint
    const unit = expectedUnitForStage(run.stage);
    return unit ? { unit, message: run.message } : { message: run.message };
  }

  return {
    current: run.counts?.current,
    total: run.counts?.total,
    unit: run.counts?.unit ?? expectedUnitForStage(run.stage),
    progress01: run.progress01,
    message: run.message,
  };
}

export function formatStepDetailLine(detail?: StageStepDetail): string | null {
  if (!detail) return null;
  if (
    typeof detail.current === "number" &&
    typeof detail.total === "number" &&
    detail.total > 0
  ) {
    const unit = detail.unit ? ` ${detail.unit}` : "";
    const pct =
      typeof detail.progress01 === "number"
        ? ` · ${Math.round(detail.progress01 * 100)}%`
        : "";
    return `${detail.current}/${detail.total}${unit}${pct}`;
  }
  if (typeof detail.progress01 === "number" && detail.progress01 > 0) {
    return `${Math.round(detail.progress01 * 100)}%`;
  }
  if (detail.message && detail.message.trim()) {
    // Prefer short message without repeating stage label noise
    const msg = detail.message.trim();
    if (msg.length <= 80) return msg;
    return `${msg.slice(0, 77)}…`;
  }
  return null;
}

/**
 * Build per-step timeline for a run.
 */
export function buildStageTimeline(run: IngestionRunView): StageTimeline {
  // Admission only — never treat an in-flight stage as Queued/Cleaning
  const admissionCleaning = run.stage === "cleaning";
  const admissionQueued = run.stage === "queued";
  const admissionPhase: AdmissionPhase = admissionCleaning
    ? "cleaning"
    : admissionQueued
      ? "queued"
      : null;
  const isAdmission = admissionCleaning || admissionQueued;

  const steps = applicableSteps(run);
  const current = run.stage;
  const currentRank = rank(current);
  const isFailed = run.stageStatus === "failed" || current === "failed";
  const isComplete =
    run.stageStatus === "complete" || current === "completed";

  const detail = !isAdmission ? formatDetail(run) : undefined;

  const timelineSteps: StageTimelineStep[] = steps.map((step) => {
    const skipped =
      (step === "converting" && shouldSkipConverting(run.sourceType)) ||
      shouldSkipForMode(step, run.mode);

    if (skipped) {
      return {
        id: step,
        label: stageDisplayName(step, run.sourceType),
        status: "skipped" as const,
      };
    }

    if (isComplete) {
      return {
        id: step,
        label: stageDisplayName(step, run.sourceType),
        status: "done" as const,
      };
    }

    if (isAdmission) {
      return {
        id: step,
        label: stageDisplayName(step, run.sourceType),
        status: "pending" as const,
      };
    }

    if (isFailed) {
      // Prefer marking the failed stage; if wire says only "failed", mark storing-ish last active as failed
      const failedId =
        current === "failed"
          ? // unknown which step blew up — mark nothing active; surface failed chip via completed slot
            null
          : current;
      if (failedId && step === failedId) {
        return {
          id: step,
          label: stageDisplayName(step, run.sourceType),
          status: "failed" as const,
          detail,
        };
      }
      const stepRank = rank(step);
      if (failedId && stepRank < rank(failedId)) {
        return {
          id: step,
          label: stageDisplayName(step, run.sourceType),
          status: "done" as const,
        };
      }
      if (step === "completed") {
        return {
          id: step,
          label: "Failed",
          status: "failed" as const,
          detail,
        };
      }
      return {
        id: step,
        label: stageDisplayName(step, run.sourceType),
        status: "pending" as const,
      };
    }

    // Active / done / pending along happy path
    if (step === current) {
      return {
        id: step,
        label: stageDisplayName(step, run.sourceType),
        status: "active" as const,
        detail,
      };
    }

    const stepRank = rank(step);
    if (currentRank >= 0 && stepRank >= 0 && stepRank < currentRank) {
      return {
        id: step,
        label: stageDisplayName(step, run.sourceType),
        status: "done" as const,
      };
    }

    return {
      id: step,
      label: stageDisplayName(step, run.sourceType),
      status: "pending" as const,
    };
  });

  // First-principles weighted overall (never fake 100% mid-flight)
  const {
    overall01,
    stageProgress01,
    stageCountsLabel,
  } = computeWeightedOverallProgress(timelineSteps, {
    isComplete,
    admissionQueued,
    admissionCleaning,
  });

  const activeStepId =
    timelineSteps.find((s) => s.status === "active" || s.status === "failed")
      ?.id ?? null;

  return {
    steps: timelineSteps,
    activeStepId,
    admissionQueued,
    admissionCleaning,
    admissionPhase,
    overallProgress01: overall01,
    overallIsEstimate: !isComplete,
    stageProgress01,
    stageCountsLabel,
  };
}
