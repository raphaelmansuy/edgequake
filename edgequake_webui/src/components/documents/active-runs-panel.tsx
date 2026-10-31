/**
 * SPEC-048 / SPEC-086: Active runs panel — shared IngestionRunCard presenter.
 * PDF converting page detail nests under the card (not a second upload product).
 *
 * SPEC-086 dual-run UX: never mix orphan failed shells under "Active runs"
 * beside a live PDF — partition Working/Queued vs Needs attention.
 *
 * Cancelled terminals: first-class orange Cancelled (never Failed); 12s TTL +
 * durable Dismiss via sessionStorage (document row remains under Cancelled).
 *
 * SPEC-155: Working section is collapsible (summary header + persisted state).
 * Needs-attention / stalled cards are never collapsed away.
 */

"use client";

import { useEffect, useReducer, useRef, useState } from "react";
import { ChevronDown, ChevronRight, X } from "lucide-react";
import { IngestionRunCard } from "@/components/documents/ingestion-run-card";
import { PdfUploadProgress } from "@/components/documents/pdf-upload-progress";
import { StalledRunCard } from "@/components/documents/stalled-run-card";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { useCancelDocument } from "@/hooks/use-cancel-document";
import { useWorkspaceCapabilities } from "@/hooks/use-workspace-capabilities";
import type { RagProfile } from "@/constants/rag-profile";
import {
  cancelledRetentionDeadlines,
  createCancelledObservationClock,
  filterWorkingRunsForRetention,
  isCancelledRun,
  workingSectionTitleForRuns,
} from "@/lib/pipeline/active-runs-retention";
import {
  loadDismissedCancelledIds,
  persistDismissedCancelledId,
  pruneDismissedCancelledIds,
  rememberCancelledFromStage,
} from "@/lib/pipeline/cancelled-active-run-dismiss";
import {
  isOrphanFailedAttention,
  isStalledAttention,
  partitionActiveRuns,
} from "@/lib/pipeline/active-runs-partition";
import {
  shouldNestPdfPageMeter,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import {
  readWorkingCollapsed,
  summarizeWorkingRuns,
  writeWorkingCollapsed,
} from "@/lib/documents/intake-strip-state";

// Re-export partition SSOT for existing test / call-site imports.
export {
  hasPanelVisibleActiveRuns,
  isOrphanFailedAttention,
  isStalledAttention,
  isLiveWorkingOrQueued,
  partitionActiveRuns,
} from "@/lib/pipeline/active-runs-partition";

interface ActiveRunsPanelProps {
  runs: IngestionRunView[];
  /** Delete/remove a failed attention shell (orphan staging re-upload class). */
  onDismissFailed?: (documentId: string) => void;
  /** Restart a stalled run (opens the Reprocess choice upstream). */
  onReprocess?: (documentId: string) => void;
}

/** Copy for the Needs-attention section, matched to what is actually in it. */
export function attentionHint(attention: IngestionRunView[]): string {
  const stalled = attention.filter(isStalledAttention).length;
  if (stalled === 0) {
    return "Prior interrupted upload(s) — dismiss and re-upload. Not part of the current active run.";
  }
  if (stalled === attention.length) {
    return "These runs stopped reporting progress — the worker has probably gone away. Cancel to release them, or Reprocess to try again.";
  }
  return "Some runs stopped reporting progress, others are interrupted uploads. Cancel stalled runs or dismiss and re-upload.";
}

/** A run the user can still stop: live or queued, not already winding down. */
export function isCancellableRun(run: IngestionRunView): boolean {
  return (
    run.stageStatus !== "failed" &&
    run.stageStatus !== "stopping" &&
    run.stageStatus !== "cancelled" &&
    run.stage !== "stopping" &&
    run.stage !== "cancelled" &&
    run.stage !== "completed"
  );
}

interface RunCardHandlers {
  onDismissFailed?: (documentId: string) => void;
  onDismissCancelled?: (documentId: string) => void;
  onCancelRun: (run: IngestionRunView) => void;
  onReprocess?: (documentId: string) => void;
  cancellingIds: ReadonlySet<string>;
  /** SPEC-180: which ingest phases exist for this workspace. */
  profile: RagProfile;
}

/** Honest section title — never "Queued run" for cancelled-only. */
export function workingSectionTitle(working: IngestionRunView[]): string {
  if (working.length === 0) return "Active run";
  const allCancelled = working.every(
    (r) => r.stage === "cancelled" || r.stageStatus === "cancelled",
  );
  if (allCancelled) {
    return "Cancelled";
  }
  const allStoppingOrCancelled = working.every(
    (r) =>
      r.stage === "stopping" ||
      r.stage === "cancelled" ||
      r.stageStatus === "cancelled" ||
      r.stageStatus === "stopping",
  );
  const anyStopping = working.some(
    (r) => r.stage === "stopping" || r.stageStatus === "stopping",
  );
  if (allStoppingOrCancelled && anyStopping) {
    return "Stopping…";
  }
  const anyWorking = working.some((r) => r.stageStatus === "active");
  if (anyWorking) {
    return working.length > 1 ? "Active runs" : "Active run";
  }
  return working.length > 1 ? "Queued runs" : "Queued run";
}

function renderRunCard(run: IngestionRunView, h: RunCardHandlers) {
  const cancel = isCancellableRun(run) ? () => h.onCancelRun(run) : undefined;

  if (isStalledAttention(run)) {
    return (
      <StalledRunCard
        key={run.documentId}
        run={run}
        onCancel={cancel}
        onReprocess={
          h.onReprocess ? () => h.onReprocess?.(run.documentId) : undefined
        }
        isCancelling={h.cancellingIds.has(run.documentId)}
      />
    );
  }

  const dismissCancelled =
    isCancelledRun(run) && h.onDismissCancelled
      ? () => h.onDismissCancelled?.(run.documentId)
      : undefined;
  const dismissFailed =
    isOrphanFailedAttention(run) && h.onDismissFailed
      ? () => h.onDismissFailed?.(run.documentId)
      : undefined;

  return (
    <IngestionRunCard
      key={run.documentId}
      run={run}
      compact
      profile={h.profile}
      data-testid="spec048-active-run-card"
      onCancel={cancel}
      onDismiss={dismissCancelled ?? dismissFailed}
      nestedDetail={
        // LAW-IS2 / F-IS-06: second progress product only when list lacks page counts.
        shouldNestPdfPageMeter(run) && run.trackId ? (
          <PdfUploadProgress
            trackId={run.trackId}
            filename={run.filename}
            compact
            nested
          />
        ) : undefined
      }
    />
  );
}

export function ActiveRunsPanel({
  runs,
  onDismissFailed,
  onReprocess,
}: ActiveRunsPanelProps) {
  const cancelDocument = useCancelDocument();
  // SPEC-180: a normal workspace ingests chunks and vectors, so its run card has no
  // extraction phase. Read the capability, never the profile name.
  const caps = useWorkspaceCapabilities();
  const profile: RagProfile = caps.extracts_graph ? "graph" : "normal";
  const [cancellingIds, setCancellingIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const clockRef = useRef(createCancelledObservationClock());
  const [dismissedCancelledIds, setDismissedCancelledIds] = useState(() =>
    loadDismissedCancelledIds(),
  );
  // Force re-render when cancelled TTL expires.
  const [, bumpRetention] = useReducer((n: number) => n + 1, 0);
  // Density-first: collapsed by default so Documents keep the viewport.
  const [workingCollapsed, setWorkingCollapsed] = useState(true);
  useEffect(() => {
    // Hydrate from localStorage after mount (SSR default stays collapsed).
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setWorkingCollapsed(readWorkingCollapsed());
  }, []);

  const onCancelRun = (run: IngestionRunView) => {
    const id = run.documentId;
    if (cancellingIds.has(id)) return;
    setCancellingIds((prev) => new Set(prev).add(id));
    void cancelDocument({ documentId: id, trackId: run.trackId }).finally(() =>
      setCancellingIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      }),
    );
  };

  // Cache freeze stage while Stopping / Cancelled so refresh stays honest.
  useEffect(() => {
    for (const run of runs) {
      if (
        run.cancelledAtStage &&
        (run.stageStatus === "stopping" ||
          run.stageStatus === "cancelled" ||
          run.stage === "stopping" ||
          run.stage === "cancelled")
      ) {
        rememberCancelledFromStage(run.documentId, run.cancelledAtStage);
      }
    }
  }, [runs]);

  const { working: rawWorking, attention } = partitionActiveRuns(runs);
  const working = filterWorkingRunsForRetention(rawWorking, {
    clock: clockRef.current,
    dismissedCancelledIds,
  });
  const summary = summarizeWorkingRuns(working);

  // Delay until each in-window cancelled card expires. Stable primitive deps —
  // never sync-bump when remaining <= 0 (that + fresh `runs` arrays loops).
  const cancelledTtlDeadlines = cancelledRetentionDeadlines(rawWorking, {
    clock: clockRef.current,
    dismissedCancelledIds,
  });
  const cancelledTtlScheduleKey = cancelledTtlDeadlines
    .map((e) => `${e.id}:${e.deadline}`)
    .join("|");

  useEffect(() => {
    if (cancelledTtlDeadlines.length === 0) return;
    const timers = cancelledTtlDeadlines.map((entry) => {
      const remaining = Math.max(1, entry.deadline - Date.now());
      return setTimeout(() => bumpRetention(), remaining);
    });
    return () => {
      for (const t of timers) clearTimeout(t);
    };
    // Schedule key is the stable identity; deadlines array is from this render.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- key encodes deadlines
  }, [cancelledTtlScheduleKey]);

  // Prune durable dismiss when docs leave cancelled (e.g. reprocess).
  useEffect(() => {
    const cancelledIds = new Set(
      runs.filter(isCancelledRun).map((r) => r.documentId),
    );
    setDismissedCancelledIds((prev) => {
      const pruned = pruneDismissedCancelledIds(cancelledIds);
      if (
        prev.size === pruned.size &&
        [...prev].every((id) => pruned.has(id))
      ) {
        return prev;
      }
      return pruned;
    });
  }, [runs]);

  if (working.length === 0 && attention.length === 0) return null;

  const onDismissCancelled = (documentId: string) => {
    const next = persistDismissedCancelledId(documentId);
    setDismissedCancelledIds(next);
  };

  const dismissAll = () => {
    if (!onDismissFailed) return;
    for (const run of attention.filter(isOrphanFailedAttention)) {
      onDismissFailed(run.documentId);
    }
  };

  const toggleWorkingCollapsed = () => {
    setWorkingCollapsed((prev) => {
      const next = !prev;
      writeWorkingCollapsed(next);
      return next;
    });
  };

  const avgPct =
    summary.avgProgress01 != null
      ? Math.round(summary.avgProgress01 * 100)
      : null;

  // Primary cancellable run for the collapsed one-line Cancel affordance.
  const primaryCancellable = working.find(isCancellableRun);

  return (
    <div
      className="space-y-1.5 rounded-lg border border-sky-200/80 bg-sky-50/40 p-1.5 dark:border-sky-900 dark:bg-sky-950/20"
      data-testid="spec048-active-runs-panel"
      data-density="compact"
      data-working-collapsed={workingCollapsed ? "true" : "false"}
    >
      {working.length > 0 && (
        <section
          className="space-y-1"
          data-testid="spec048-active-runs-working"
        >
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              className="flex min-w-0 flex-1 items-center gap-1.5 rounded-md text-left hover:bg-sky-100/60 dark:hover:bg-sky-900/40 px-1.5 py-1"
              onClick={toggleWorkingCollapsed}
              aria-expanded={!workingCollapsed}
              data-testid="documents-intake-toggle"
            >
              {workingCollapsed ? (
                <ChevronRight
                  className="h-3.5 w-3.5 shrink-0 text-muted-foreground"
                  aria-hidden="true"
                />
              ) : (
                <ChevronDown
                  className="h-3.5 w-3.5 shrink-0 text-muted-foreground"
                  aria-hidden="true"
                />
              )}
              <div className="min-w-0 flex-1">
                <div className="flex min-w-0 items-baseline gap-1.5">
                  <span className="shrink-0 text-xs font-medium tracking-tight">
                    {workingSectionTitleForRuns(working)}
                  </span>
                  {workingCollapsed ? (
                    <span
                      className="truncate text-xs text-muted-foreground"
                      data-testid="documents-intake-summary"
                    >
                      {summary.text}
                    </span>
                  ) : null}
                </div>
              </div>
            </button>
            {workingCollapsed && avgPct != null ? (
              <div
                className="flex w-24 shrink-0 items-center gap-1.5"
                aria-hidden="true"
              >
                <Progress
                  value={avgPct}
                  className="h-1.5 flex-1 bg-sky-200 dark:bg-sky-900 [&>[data-slot=progress-indicator]]:bg-sky-600 dark:[&>[data-slot=progress-indicator]]:bg-sky-400"
                />
                <span className="w-8 text-right text-xs tabular-nums text-muted-foreground">
                  {avgPct}%
                </span>
              </div>
            ) : null}
            {working.length > 1 ? (
              <div
                className="rounded-full bg-sky-100 px-1.5 text-xs font-medium tabular-nums text-sky-800 dark:bg-sky-950 dark:text-sky-200"
                aria-label={`${working.length} runs`}
              >
                {working.length}
              </div>
            ) : null}
            {workingCollapsed && primaryCancellable ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-7 shrink-0 gap-1 border-border/60 px-2 text-xs text-muted-foreground hover:text-foreground"
                onClick={() => onCancelRun(primaryCancellable)}
                disabled={cancellingIds.has(primaryCancellable.documentId)}
                data-testid="spec086-run-cancel"
              >
                <X className="h-3 w-3" aria-hidden="true" />
                Cancel
              </Button>
            ) : null}
          </div>
          {!workingCollapsed
            ? working.map((run) =>
                renderRunCard(run, {
                  onDismissFailed,
                  onDismissCancelled,
                  onCancelRun,
                  onReprocess,
                  cancellingIds,
                  profile,
                }),
              )
            : null}
        </section>
      )}

      {attention.length > 0 && (
        <section
          className="space-y-1.5 rounded-md border border-amber-200/80 bg-amber-50/50 p-2 dark:border-amber-900/60 dark:bg-amber-950/30"
          data-testid="spec086-needs-attention"
        >
          <div className="flex items-center justify-between gap-2">
            <div className="min-w-0">
              <div className="text-sm font-medium tracking-tight">
                Needs attention
              </div>
              <p
                className="mt-0.5 text-xs text-muted-foreground"
                data-testid="spec155-attention-hint"
              >
                {attentionHint(attention)}
              </p>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <span className="text-xs tabular-nums text-muted-foreground">
                {attention.length}
              </span>
              {onDismissFailed && attention.some(isOrphanFailedAttention) && (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="h-7 text-xs"
                  data-testid="spec086-dismiss-all-attention"
                  onClick={dismissAll}
                >
                  Dismiss all
                </Button>
              )}
            </div>
          </div>
          {attention.map((run) =>
            renderRunCard(run, {
              onDismissFailed,
              onCancelRun,
              onReprocess,
              cancellingIds,
              profile,
            }),
          )}
        </section>
      )}
    </div>
  );
}

export default ActiveRunsPanel;
