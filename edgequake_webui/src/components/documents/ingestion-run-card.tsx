/**
 * SPEC-086: one progress presenter for all formats (ActiveRuns-style stepper).
 * PDF page detail is an optional nested slot under converting — not a second product.
 */

"use client";

import { ServerStageStepper } from "@/components/documents/server-stage-stepper";
import { Progress } from "@/components/ui/progress";
import {
  stageDisplayName,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import { buildStageTimeline } from "@/lib/pipeline/stage-timeline";
import type { ReactNode } from "react";

export interface IngestionRunCardProps {
  run: IngestionRunView;
  /** Nested detail (e.g. PDF page N/M) — only while converting. */
  nestedDetail?: ReactNode;
  compact?: boolean;
  /** Cancel in-flight run (ActiveRuns / upload parity). */
  onCancel?: () => void;
  /**
   * Dismiss a terminal failed attention card (removes failed admission shell).
   * SPEC-086: orphan staging "please re-upload" had Cancel disabled and no exit.
   */
  onDismiss?: () => void;
  className?: string;
  "data-testid"?: string;
}

/** Failed attention runs get Dismiss (not Cancel). */
export function canDismissFailedRun(
  run: Pick<IngestionRunView, "stage" | "stageStatus">,
  hasDismissHandler: boolean,
): boolean {
  return (
    hasDismissHandler &&
    (run.stageStatus === "failed" || run.stage === "failed")
  );
}

export function IngestionRunCard({
  run,
  nestedDetail,
  compact = false,
  onCancel,
  onDismiss,
  className,
  "data-testid": testId,
}: IngestionRunCardProps) {
  const timeline = buildStageTimeline(run);
  const admission = timeline.admissionPhase;
  const isAdmission = Boolean(admission);
  const overallPct = Math.round(timeline.overallProgress01 * 100);
  const stagePct =
    typeof timeline.stageProgress01 === "number"
      ? Math.round(timeline.stageProgress01 * 100)
      : undefined;
  const hasStageCounts = Boolean(run.counts && run.counts.total > 0);
  const showPdfDetail =
    Boolean(nestedDetail) &&
    run.sourceType === "pdf" &&
    run.stage === "converting";
  const canCancel =
    Boolean(onCancel) &&
    !isAdmission &&
    run.stage !== "completed" &&
    run.stage !== "failed" &&
    run.stage !== "cancelled" &&
    run.stage !== "stopping";
  const canDismiss = canDismissFailedRun(run, Boolean(onDismiss));

  return (
    <div
      className={
        className ??
        (compact
          ? "space-y-1.5"
          : "space-y-2 rounded-md border border-border/80 bg-background p-2.5 shadow-sm")
      }
      data-testid={testId ?? "spec086-ingestion-run-card"}
      data-document-id={run.documentId}
      data-stage={run.stage}
      data-source-type={run.sourceType}
      data-mode={run.mode ?? "full"}
      data-admission={admission ?? "running"}
    >
      <div className="flex items-center justify-between gap-2 text-sm">
        <span className="truncate font-medium text-foreground">
          {run.filename}
        </span>
        <div className="flex shrink-0 items-center gap-2">
          <span
            className="text-xs tabular-nums text-sky-700 dark:text-sky-300"
            data-testid="spec048-run-headline"
          >
            {run.counts
              ? `${stageDisplayName(run.stage, run.sourceType)} · ${run.counts.current}/${run.counts.total}`
              : stageDisplayName(run.stage, run.sourceType)}
          </span>
          {canCancel ? (
            <button
              type="button"
              className="text-xs text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
              onClick={onCancel}
              data-testid="spec086-run-cancel"
            >
              Cancel
            </button>
          ) : null}
          {canDismiss ? (
            <button
              type="button"
              className="text-xs text-muted-foreground hover:text-foreground underline-offset-2 hover:underline"
              onClick={onDismiss}
              title="Remove this failed upload. Re-upload the file to try again."
              data-testid="spec086-run-dismiss"
            >
              Dismiss
            </button>
          ) : null}
        </div>
      </div>

      <ServerStageStepper run={run} />

      {showPdfDetail ? (
        <div data-testid="spec086-pdf-converting-detail" className="pt-0.5">
          {nestedDetail}
        </div>
      ) : null}

      {isAdmission ? (
        <div
          className="h-1.5 w-full overflow-hidden rounded bg-muted"
          data-testid="spec048-run-progress-indeterminate"
          data-admission={admission ?? undefined}
        >
          <div
            className={
              admission === "cleaning"
                ? "h-full w-1/3 animate-pulse rounded bg-rose-400/70"
                : "h-full w-1/3 animate-pulse rounded bg-amber-400/70"
            }
          />
        </div>
      ) : (
        <div className="space-y-1.5">
          {hasStageCounts && typeof stagePct === "number" ? (
            <div className="space-y-0.5" data-testid="spec048-stage-progress">
              <div className="flex items-center justify-between gap-2 text-[10px] text-muted-foreground">
                <span>
                  This stage
                  {timeline.stageCountsLabel
                    ? ` · ${timeline.stageCountsLabel}`
                    : ""}
                </span>
                <span className="tabular-nums">{stagePct}%</span>
              </div>
              <Progress
                value={stagePct}
                className="h-1.5 [&_[data-slot=progress-indicator]]:bg-sky-500"
              />
            </div>
          ) : (
            <div
              className="h-1.5 w-full overflow-hidden rounded bg-muted"
              data-testid="spec048-run-progress-indeterminate"
            >
              <div className="h-full w-1/3 animate-pulse rounded bg-sky-400/70" />
            </div>
          )}

          <div className="space-y-0.5" data-testid="spec048-overall-progress">
            <div className="flex items-center justify-between gap-2 text-[10px] text-muted-foreground">
              <span>Overall (est.)</span>
              <span
                className="tabular-nums"
                data-testid="spec048-run-overall-pct"
              >
                {overallPct}%
              </span>
            </div>
            <Progress
              value={overallPct}
              className="h-1 [&_[data-slot=progress-indicator]]:bg-sky-400/80"
            />
          </div>
        </div>
      )}

      {run.message ? (
        <p
          className="text-[11px] text-muted-foreground line-clamp-2"
          data-testid="spec086-run-message"
        >
          {run.message}
        </p>
      ) : null}

      {run.mode && run.mode !== "full" ? (
        <div
          className="text-[11px] text-muted-foreground"
          data-testid="spec048-run-mode"
        >
          Reprocess mode: {run.mode}
        </div>
      ) : null}
    </div>
  );
}

export default IngestionRunCard;
