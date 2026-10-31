/**
 * Pure model for the segmented phase progress (Admit / Prepare / Extract /
 * Materialize). SPEC-155: each segment fill comes from the typed run-progress
 * ledger when present — never from a single regex-parsed counter that flips
 * units mid-phase.
 */

import {
  PHASE_STRIP_ORDER,
  mapWireStageToPhase,
  type IngestionPhaseId,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import {
  findPhase,
  formatPhaseSummary,
  phaseFillPct,
  stripPhaseToLedger,
  type RunProgress,
} from "@/lib/pipeline/run-progress";
import type { ProfilePhase, ProfilePhaseId } from "@/lib/pipeline/profile-phases";

export type PhaseSegmentStatus = "done" | "active" | "pending" | "failed";

/** A real, tiny value (1%) must still be visible as a sliver. */
export const MIN_VISIBLE_FILL_PCT = 4;

export interface PhaseSegment {
  phase: IngestionPhaseId;
  status: PhaseSegmentStatus;
  /** 0–100 paint width of the segment fill. */
  fillPct: number;
  /** Active phase without a determinate N/M: animated, not a fake number. */
  indeterminate: boolean;
  /** Done-segment summary for tooltip / aria (e.g. "92 pages, 12 figures"). */
  summary?: string | null;
}

export function phaseSegmentStatus(
  phase: IngestionPhaseId,
  active: IngestionPhaseId,
  failed: boolean,
): PhaseSegmentStatus {
  const ai = PHASE_STRIP_ORDER.indexOf(active);
  const pi = PHASE_STRIP_ORDER.indexOf(phase);
  if (failed && pi === ai) return "failed";
  if (pi < ai) return "done";
  if (pi === ai) return "active";
  return "pending";
}

/** Clamp to 0–100 and keep a visible sliver for small non-zero values. */
export function visibleFillPct(pct: number): number {
  if (!Number.isFinite(pct) || pct <= 0) return 0;
  return Math.min(100, Math.max(MIN_VISIBLE_FILL_PCT, pct));
}

/**
 * Build four segments. When `run.runProgress` is present its per-phase fills
 * win; otherwise fall back to the legacy stagePct for the active segment only.
 */
export function buildPhaseSegments(
  run: Pick<IngestionRunView, "stage" | "stageStatus" | "runProgress">,
  stagePct: number | undefined,
): PhaseSegment[] {
  const active = mapWireStageToPhase(run.stage);
  const failed = run.stageStatus === "failed" || run.stage === "failed";
  const ledger = run.runProgress ?? null;

  return PHASE_STRIP_ORDER.map((phase) => {
    const status = phaseSegmentStatus(phase, active, failed);
    const ledgerPhaseId = stripPhaseToLedger(phase);
    const ledgerPhase =
      ledger && ledgerPhaseId ? findPhase(ledger, ledgerPhaseId) : undefined;
    const summary = formatPhaseSummary(ledgerPhase);

    if (status === "done" || status === "failed") {
      return {
        phase,
        status,
        fillPct: 100,
        indeterminate: false,
        summary,
      };
    }
    if (status === "pending") {
      return { phase, status, fillPct: 0, indeterminate: false, summary: null };
    }

    // Active
    if (ledger && ledgerPhaseId) {
      const fill = phaseFillPct(ledgerPhase);
      if (ledgerPhase && (ledgerPhase.tasks?.length ?? 0) > 0) {
        return {
          phase,
          status,
          fillPct: visibleFillPct(fill),
          indeterminate: false,
          summary,
        };
      }
      // Ledger present but no tasks yet — indeterminate pulse.
      return {
        phase,
        status,
        fillPct: 0,
        indeterminate: true,
        summary,
      };
    }

    // Legacy fallback (no ledger).
    return typeof stagePct === "number"
      ? {
          phase,
          status,
          fillPct: visibleFillPct(stagePct),
          indeterminate: false,
          summary: null,
        }
      : { phase, status, fillPct: 0, indeterminate: true, summary: null };
  });
}

/**
 * A phase segment for one profile's strip (SPEC-180).
 *
 * `buildPhaseSegments` above keeps the fixed four-phase shape for older call sites; this one
 * describes the phases the profile actually has, so classical RAG never shows "Extract".
 */
export interface ProfilePhaseSegment {
  phase: ProfilePhaseId;
  /** Profile label ("Index" for both, "Extract" only for GraphRAG). */
  label: string;
  /** One sentence about what this phase produces for this profile. */
  hint: string;
  status: PhaseSegmentStatus;
  fillPct: number;
  indeterminate: boolean;
  summary?: string | null;
}

export interface CaptionProgress {
  /** Percentage painted next to the headline. */
  pct: number;
  /** True when it is the overall estimate (no determinate stage N/M). */
  estimated: boolean;
}

/**
 * Build one segment per phase the profile really has.
 *
 * Fill comes from the typed ledger when the phase maps to one; the active phase without a
 * determinate ledger falls back to the legacy stage percentage, then to an indeterminate
 * pulse. A done phase is full; a pending one is empty.
 */
export function buildProfileSegments(
  run: Pick<IngestionRunView, "stage" | "stageStatus" | "runProgress">,
  stagePct: number | undefined,
  opts: { phases: readonly ProfilePhase[]; activeIndex: number },
): ProfilePhaseSegment[] {
  const failed = run.stageStatus === "failed" || run.stage === "failed";
  const ledger = run.runProgress ?? null;

  return opts.phases.map((phase, index) => {
    const status: PhaseSegmentStatus =
      failed && index === opts.activeIndex
        ? "failed"
        : index < opts.activeIndex
          ? "done"
          : index === opts.activeIndex
            ? "active"
            : "pending";

    const base = { phase: phase.id, label: phase.label, hint: phase.hint };
    // The ledger speaks the fixed four-phase vocabulary. `index` is the profile name for the
    // same final work (embed + store), so it maps to the ledger's `materialize`.
    const ledgerPhaseId = stripPhaseToLedger(
      phase.id === "index" ? "materialize" : phase.id,
    );
    const ledgerPhase =
      ledger && ledgerPhaseId ? findPhase(ledger, ledgerPhaseId) : undefined;
    const summary = formatPhaseSummary(ledgerPhase);

    if (status === "done" || status === "failed") {
      return { ...base, status, fillPct: 100, indeterminate: false, summary };
    }
    if (status === "pending") {
      return { ...base, status, fillPct: 0, indeterminate: false, summary: null };
    }

    if (ledger && ledgerPhaseId) {
      const fill = phaseFillPct(ledgerPhase);
      if (ledgerPhase && (ledgerPhase.tasks?.length ?? 0) > 0) {
        return {
          ...base,
          status,
          fillPct: visibleFillPct(fill),
          indeterminate: false,
          summary,
        };
      }
      return { ...base, status, fillPct: 0, indeterminate: true, summary };
    }

    return typeof stagePct === "number"
      ? {
          ...base,
          status,
          fillPct: visibleFillPct(stagePct),
          indeterminate: false,
          summary: null,
        }
      : { ...base, status, fillPct: 0, indeterminate: true, summary: null };
  });
}

/** Stage N/M wins; otherwise fall back to the overall estimate (LAW-IS2). */
export function resolveCaptionProgress(input: {
  stagePct: number | undefined;
  overallPct: number;
  /** When the active ledger phase has determinate tasks, prefer its fill. */
  ledger?: RunProgress | null;
  activePhase?: IngestionPhaseId;
}): CaptionProgress {
  if (input.ledger && input.activePhase) {
    const lid = stripPhaseToLedger(input.activePhase);
    if (lid) {
      const phase = findPhase(input.ledger, lid);
      if (phase && (phase.tasks?.length ?? 0) > 0) {
        return { pct: phaseFillPct(phase), estimated: false };
      }
    }
  }
  return typeof input.stagePct === "number"
    ? { pct: input.stagePct, estimated: false }
    : { pct: input.overallPct, estimated: true };
}
