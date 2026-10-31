/**
 * SPEC-048: Server-aligned stage stepper with per-step detail progress.
 *
 * Renders full UnifiedStage timeline (skip/fail/active/done) and shows
 * countable detail on the active (or failed) step.
 */

"use client";

import { AdmissionPhaseRow } from "@/components/documents/admission-phase-row";
import {
  PhaseStrip,
  type PhaseStripProgress,
} from "@/components/documents/phase-strip";
import { cn } from "@/lib/utils";
import type { IngestionRunView } from "@/lib/pipeline/ingestion-run-view";
import type { RagProfile } from "@/constants/rag-profile";
import {
  buildStageTimeline,
  formatStepDetailLine,
  type StageStepStatus,
} from "@/lib/pipeline/stage-timeline";

interface ServerStageStepperProps {
  run: IngestionRunView;
  /** When true, hide skipped converting for non-PDF (still shown muted by default). */
  hideSkipped?: boolean;
  /**
   * `phases` (default): SPEC-091 IS3 4-phase strip for ActiveRuns.
   * `wire`: full UnifiedStage chips (Pipeline / Details).
   */
  variant?: "phases" | "wire";
  /**
   * Headline already painted by the parent card. When it already contains the
   * step detail sentence, the detail box is kept for assistive tech but not
   * painted a second time.
   */
  headlineText?: string;
  /**
   * `phases` variant only: paint the strip as the progress bar itself
   * (segmented) instead of chips with a separate meter.
   */
  phaseProgress?: PhaseStripProgress;
  /**
   * Caption mode paints the status sentence itself (one line, one percent).
   * Skip the second detail row so "3/5 pages" is not repeated under the strip.
   */
  hideStepDetail?: boolean;
  /** SPEC-180: which phases exist. A `normal` workspace has no extraction phase. */
  profile?: RagProfile;
  className?: string;
}

/** True when the visible headline already says everything the detail box would. */
export function isDetailRedundantWithHeadline(
  headlineText: string | undefined,
  detailLine: string | null | undefined,
): boolean {
  if (!headlineText || !detailLine) return false;
  const normalize = (s: string) => s.toLowerCase().replace(/[\s·]+/g, "");
  const headline = normalize(headlineText);
  const detail = normalize(detailLine);
  if (!headline || !detail) return false;
  // Detail is often "<headline> · 3%": the percentage lives in the meter row.
  return headline.includes(detail) || detail.includes(headline);
}

function statusClasses(status: StageStepStatus): string {
  switch (status) {
    case "done":
      return "text-emerald-700 dark:text-emerald-400";
    case "active":
      return "bg-sky-100 text-sky-800 dark:bg-sky-950 dark:text-sky-200";
    case "failed":
      return "bg-rose-100 text-rose-800 dark:bg-rose-950 dark:text-rose-200";
    case "cancelled":
      return "bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-200";
    case "skipped":
      return "text-muted-foreground/50 line-through decoration-muted-foreground/40";
    default:
      return "text-muted-foreground";
  }
}

function dotClasses(status: StageStepStatus): string {
  switch (status) {
    case "done":
      return "bg-emerald-500";
    case "active":
      return "bg-sky-500 animate-pulse";
    case "failed":
      return "bg-rose-500";
    case "cancelled":
      return "bg-orange-500";
    case "skipped":
      return "bg-muted-foreground/25";
    default:
      return "bg-muted-foreground/40";
  }
}

export function ServerStageStepper({
  run,
  hideSkipped = false,
  variant = "phases",
  headlineText,
  phaseProgress,
  hideStepDetail = false,
  profile = "graph",
  className,
}: ServerStageStepperProps) {
  const timeline = buildStageTimeline(run);
  const steps = hideSkipped
    ? timeline.steps.filter((s) => s.status !== "skipped")
    : timeline.steps;
  const active = steps.find(
    (s) =>
      s.status === "active" ||
      s.status === "failed" ||
      s.status === "cancelled",
  );
  const detailLine = formatStepDetailLine(active?.detail);
  const detailRedundant = isDetailRedundantWithHeadline(
    headlineText,
    active ? `${active.label} ${detailLine ?? ""}` : detailLine,
  );
  const admissionPhase = timeline.admissionPhase;
  const isCancelTerminal =
    run.stageStatus === "cancelled" ||
    run.stage === "cancelled" ||
    run.stageStatus === "stopping" ||
    run.stage === "stopping";

  return (
    <div
      className={cn("min-w-0 space-y-2", className)}
      data-testid="spec048-server-stage-stepper"
      data-stage={run.stage}
      data-admission={admissionPhase ?? "running"}
      data-overall-progress={timeline.overallProgress01.toFixed(3)}
      data-variant={variant}
    >
      {admissionPhase ? (
        <AdmissionPhaseRow
          phase={admissionPhase}
          stageMessage={run.message}
          variant="pill"
        />
      ) : null}

      {variant === "phases" ? (
        <PhaseStrip
          run={run}
          progress={phaseProgress}
          profile={profile}
          className="min-w-0"
        />
      ) : (
      <div className="flex flex-wrap items-center gap-1.5 text-xs">
        {steps.map((step) => (
          <span
            key={step.id}
            className={cn(
              "inline-flex items-center gap-1 rounded px-1.5 py-0.5",
              statusClasses(step.status),
            )}
            data-testid={`spec048-stage-${step.id}`}
            data-state={step.status}
            title={
              step.status === "skipped"
                ? `${step.label} (skipped)`
                : step.label
            }
          >
            <span
              className={cn("h-1.5 w-1.5 rounded-full", dotClasses(step.status))}
            />
            {step.label}
            {step.status === "skipped" ? (
              <span className="sr-only">skipped</span>
            ) : null}
          </span>
        ))}
      </div>
      )}

      {active && detailLine && !isCancelTerminal && !hideStepDetail ? (
        <div
          className={cn(
            "flex items-baseline justify-between gap-3 text-xs tabular-nums",
            active.status === "failed"
              ? "rounded-md border border-rose-200 bg-rose-50/80 px-2 py-1.5 text-rose-800 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-200"
              : "text-foreground",
          )}
          data-testid="spec048-step-detail"
          data-stage={active.id}
          data-redundant={detailRedundant ? "true" : "false"}
        >
          <p className="min-w-0 truncate">
            <span className="font-medium">{active.label}</span>
            <span className="mx-1.5 text-muted-foreground">·</span>
            <span className="text-muted-foreground">{detailLine}</span>
          </p>
        </div>
      ) : null}
    </div>
  );
}

export default ServerStageStepper;
