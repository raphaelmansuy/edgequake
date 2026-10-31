/**
 * SPEC-086: one progress presenter for all formats (ActiveRuns-style stepper).
 * PDF page detail is an optional nested slot under converting — not a second product.
 *
 * Cancelled/stopping: honest orange terminals with frozen progress (never Failed).
 */

"use client";

import { ServerStageStepper } from "@/components/documents/server-stage-stepper";
import { RunCaption, RunDetailsToggle } from "@/components/documents/run-caption";
import { RunMeterRow } from "@/components/documents/run-meter-row";
import { Button } from "@/components/ui/button";
import {
  formatQueueChrome,
  formatRunHeadline,
  mapWireStageToPhase,
  shouldNestPdfPageMeter,
  shouldShowOverallMeter,
  stageDisplayName,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import { resolveCaptionProgress } from "@/lib/pipeline/phase-segments";
import type { RagProfile } from "@/constants/rag-profile";
import {
  buildStageTimeline,
  formatStepDetailLine,
} from "@/lib/pipeline/stage-timeline";
import { X } from "lucide-react";
import { useState, type ReactNode } from "react";

export { shouldNestPdfPageMeter, shouldShowOverallMeter };

export interface IngestionRunCardProps {
  run: IngestionRunView;
  /** Nested detail (e.g. PDF page N/M) — only while converting. */
  nestedDetail?: ReactNode;
  compact?: boolean;
  /** Cancel in-flight run (ActiveRuns / upload parity). */
  onCancel?: () => void;
  /**
   * Dismiss a terminal card (orphan failed shell delete, or cancelled ack hide).
   */
  onDismiss?: () => void;
  className?: string;
  /**
   * SPEC-180: the workspace profile. A `normal` workspace ingests chunks and vectors only,
   * so its card never shows an extraction phase. Defaults to `graph` for older call sites.
   */
  profile?: RagProfile;
  "data-testid"?: string;
}

/** Orphan failed attention OR compact cancelled ack get Dismiss. */
export function canDismissTerminalRun(
  run: Pick<IngestionRunView, "stage" | "stageStatus">,
  hasDismissHandler: boolean,
): boolean {
  if (!hasDismissHandler) return false;
  if (run.stage === "cancelled" || run.stageStatus === "cancelled") return true;
  return run.stageStatus === "failed" || run.stage === "failed";
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

/** Cancelled Working cards get Dismiss (local AR suppress — not document delete). */
export function canDismissCancelledRun(
  run: Pick<IngestionRunView, "stage" | "stageStatus">,
  hasDismissHandler: boolean,
): boolean {
  return (
    hasDismissHandler &&
    (run.stageStatus === "cancelled" || run.stage === "cancelled")
  );
}

/** True when every N/M in the sentence is finished (4/4, not 3/5). */
export function fractionCountsSettled(text: string): boolean {
  const pairs = [...text.matchAll(/(\d+)\s*\/\s*(\d+)/g)];
  if (pairs.length === 0) return false;
  return pairs.every((match) => {
    const total = Number(match[2]);
    return total > 0 && Number(match[1]) >= total;
  });
}

/** Prefer the ledger/stage headline when it already carries the same counts. */
export function visibleStatusLine(
  headline: string,
  stepLabel: string | undefined,
  stepDetail: string | null,
): string {
  if (!stepDetail) return headline;
  const nums = stepDetail.match(/\d+/g) ?? [];
  if (nums.length > 0 && nums.every((n) => headline.includes(n))) {
    return headline;
  }
  return stepLabel ? `${stepLabel} · ${stepDetail}` : stepDetail;
}

function normalizeStatus(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "");
}

/** True when a backend message only restates a line already on the card. */
export function statusLineRepeats(
  message: string | null | undefined,
  lines: Array<string | null | undefined>,
): boolean {
  const body = normalizeStatus(message ?? "");
  if (body.length < 4) return false;
  return lines.some((line) => {
    const other = normalizeStatus(line ?? "");
    if (other.length < 4) return false;
    return body === other || other.includes(body) || body.includes(other);
  });
}

function isCancelTerminal(
  run: Pick<IngestionRunView, "stage" | "stageStatus">,
): boolean {
  return (
    run.stageStatus === "cancelled" ||
    run.stage === "cancelled" ||
    run.stageStatus === "stopping" ||
    run.stage === "stopping"
  );
}

export function IngestionRunCard({
  run,
  nestedDetail,
  compact = false,
  onCancel,
  onDismiss,
  className,
  profile = "graph",
  "data-testid": testId,
}: IngestionRunCardProps) {
  // SPEC-099: compact cards hide verbose message until expanded
  const [detailsOpen, setDetailsOpen] = useState(false);
  const timeline = buildStageTimeline(run);
  const admission = timeline.admissionPhase;
  const isAdmission = Boolean(admission);
  const cancelTerminal = isCancelTerminal(run);
  const overallPct = Math.round(timeline.overallProgress01 * 100);
  const stagePct =
    typeof timeline.stageProgress01 === "number"
      ? Math.round(timeline.stageProgress01 * 100)
      : undefined;
  const hasStageCounts = Boolean(run.counts && run.counts.total > 0);
  // LAW-IS2: stage N/M is the one primary meter; overall is only the fallback.
  const determinateStagePct = hasStageCounts ? stagePct : undefined;
  const caption = resolveCaptionProgress({
    stagePct: determinateStagePct,
    overallPct,
    ledger: run.runProgress,
    activePhase: mapWireStageToPhase(run.stage),
  });
  // Live runs: the phase strip IS the bar and the headline sits in the caption.
  const captionMode = !isAdmission && !cancelTerminal;
  // LAW-IS2: nest only when list SSOT lacks page/figure counts (no second bar).
  const showPdfDetail =
    Boolean(nestedDetail) && shouldNestPdfPageMeter(run);
  const canCancel =
    Boolean(onCancel) &&
    !isAdmission &&
    !cancelTerminal &&
    run.stage !== "completed" &&
    run.stage !== "failed";
  const canDismissFailed = canDismissFailedRun(run, Boolean(onDismiss));
  const canDismissCancelled = canDismissCancelledRun(run, Boolean(onDismiss));
  const canDismiss = canDismissFailed || canDismissCancelled;

  const headlineClass = cancelTerminal
    ? run.stageStatus === "cancelled" || run.stage === "cancelled"
      ? "text-xs tabular-nums text-orange-700 dark:text-orange-300"
      : "text-xs tabular-nums text-orange-700/80 dark:text-orange-300/80"
    : "text-xs tabular-nums text-sky-700 dark:text-sky-300";

  const ledgerHeadline = cancelTerminal
    ? stageDisplayName(run.stage, run.sourceType)
    : isAdmission
      ? formatQueueChrome(run) || formatRunHeadline(run)
      : formatRunHeadline(run).replace(` · ${run.filename}`, "");
  const activeStep = timeline.steps.find(
    (step) =>
      step.status === "active" ||
      step.status === "failed" ||
      step.status === "cancelled",
  );
  const stepDetail = formatStepDetailLine(activeStep?.detail);
  const stepLinePaints =
    captionMode &&
    Boolean(stepDetail?.includes("/")) &&
    activeStep?.status !== "cancelled";
  const headlineText = stepLinePaints
    ? visibleStatusLine(ledgerHeadline, activeStep?.label, stepDetail)
    : ledgerHeadline;
  const extraMessage =
    run.message &&
    !statusLineRepeats(run.message, [
      headlineText,
      ledgerHeadline,
      stepDetail,
      activeStep ? `${activeStep.label} · ${stepDetail ?? ""}` : null,
    ])
      ? run.message
      : null;

  return (
    <div
      className={
        className ??
        (compact
          ? cancelTerminal
            ? "min-w-0 space-y-1 overflow-hidden rounded-md border border-orange-200/70 bg-orange-50/30 px-2 py-1.5 dark:border-orange-900/50 dark:bg-orange-950/20"
            : "min-w-0 space-y-1 overflow-hidden rounded-md border border-border/60 bg-background/90 px-2 py-1.5"
          : cancelTerminal
            ? "min-w-0 space-y-2 overflow-hidden rounded-md border border-orange-200/80 bg-orange-50/40 p-2.5 shadow-sm dark:border-orange-900/50 dark:bg-orange-950/20"
            : "min-w-0 space-y-2 overflow-hidden rounded-md border border-border/80 bg-background p-2.5 shadow-sm")
      }
      data-testid={testId ?? "spec086-ingestion-run-card"}
      data-document-id={run.documentId}
      data-stage={run.stage}
      data-stage-status={run.stageStatus}
      data-source-type={run.sourceType}
      data-mode={run.mode ?? "full"}
      data-admission={cancelTerminal ? "cancelled" : (admission ?? "running")}
      data-compact={compact ? "true" : "false"}
    >
      <div className="flex min-w-0 items-center justify-between gap-2 text-sm">
        <span className="min-w-0 truncate font-medium text-foreground">
          {run.filename}
        </span>
        <div className="flex shrink-0 items-center gap-2">
          {compact && extraMessage ? (
            <RunDetailsToggle
              open={detailsOpen}
              onToggle={() => setDetailsOpen((open) => !open)}
            />
          ) : null}
          {!captionMode ? (
            <span className={headlineClass} data-testid="spec048-run-headline">
              {headlineText}
            </span>
          ) : null}
          {canCancel ? (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-7 gap-1 border-border/60 px-2 text-xs text-muted-foreground hover:text-foreground"
              onClick={onCancel}
              data-testid="spec086-run-cancel"
            >
              <X className="h-3 w-3" aria-hidden="true" />
              Cancel
            </Button>
          ) : null}
          {canDismiss ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs text-muted-foreground"
              onClick={onDismiss}
              title={
                canDismissCancelled
                  ? "Hide this cancelled run from Active Runs. The document stays in the list."
                  : "Remove this failed upload. Re-upload the file to try again."
              }
              data-testid="spec086-run-dismiss"
            >
              Dismiss
            </Button>
          ) : null}
        </div>
      </div>

      {/* IS3: 4-phase strip by default (wire chips on Pipeline dialog). */}
      <ServerStageStepper
        run={run}
        variant="phases"
        profile={profile}
        headlineText={cancelTerminal ? undefined : headlineText}
        phaseProgress={captionMode ? { stagePct: determinateStagePct } : undefined}
        hideStepDetail={stepLinePaints}
      />

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
      ) : cancelTerminal ? (
        <div className="space-y-1.5" data-testid="spec086-cancel-progress-frozen">
          <RunMeterRow
            testId="spec048-overall-progress"
            pctTestId="spec048-run-overall-pct"
            label="Overall (frozen)"
            pct={overallPct}
            ariaLabel="Overall progress (frozen)"
            barClassName="h-1.5"
            indicatorClassName="[&_[data-slot=progress-indicator]]:bg-orange-400/70"
          />
        </div>
      ) : (
        <RunCaption
          headlineText={headlineText}
          pct={caption.pct}
          estimated={caption.estimated}
          overallPct={overallPct}
          srLabel={`This stage${
            timeline.stageCountsLabel ? ` · ${timeline.stageCountsLabel}` : ""
          }`}
          stepDetail={
            stepLinePaints && activeStep
              ? {
                  stage: activeStep.id,
                  failed: activeStep.status === "failed",
                }
              : undefined
          }
          countComplete={fractionCountsSettled(headlineText)}
        />
      )}

      {showPdfDetail ? (
        <div data-testid="spec086-pdf-converting-detail" className="pt-0.5">
          {nestedDetail}
        </div>
      ) : null}

      {extraMessage && (!compact || detailsOpen) ? (
        <p
          className="text-xs text-muted-foreground line-clamp-2"
          data-testid="spec086-run-message"
        >
          {extraMessage}
        </p>
      ) : null}

      {run.mode && run.mode !== "full" ? (
        <div
          className="text-xs text-muted-foreground"
          data-testid="spec048-run-mode"
        >
          Reprocess mode: {run.mode}
        </div>
      ) : null}

      {/* IS3: optional cost chip when spend is non-zero. */}
      {typeof run.costUsd === "number" && run.costUsd > 0 ? (
        <div
          className="text-xs tabular-nums text-muted-foreground"
          data-testid="spec091-run-cost"
        >
          Cost so far ${run.costUsd.toFixed(2)}
        </div>
      ) : null}
    </div>
  );
}

export default IngestionRunCard;
