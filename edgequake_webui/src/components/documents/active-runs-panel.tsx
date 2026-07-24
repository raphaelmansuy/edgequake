/**
 * SPEC-048 / SPEC-086: Active runs panel — shared IngestionRunCard presenter.
 * PDF converting page detail nests under the card (not a second upload product).
 *
 * SPEC-086 dual-run UX: never mix orphan failed shells under "Active runs"
 * beside a live PDF — partition Working/Queued vs Needs attention.
 */

"use client";

import { useQueryClient } from "@tanstack/react-query";
import { IngestionRunCard } from "@/components/documents/ingestion-run-card";
import { PdfUploadProgress } from "@/components/documents/pdf-upload-progress";
import { Button } from "@/components/ui/button";
import { cancelTask } from "@/lib/api/edgequake";
import type { IngestionRunView } from "@/lib/pipeline/ingestion-run-view";

interface ActiveRunsPanelProps {
  runs: IngestionRunView[];
  /** Delete/remove a failed attention shell (orphan staging re-upload class). */
  onDismissFailed?: (documentId: string) => void;
}

export function isOrphanFailedAttention(run: IngestionRunView): boolean {
  return (
    run.stageStatus === "failed" &&
    /re-upload|interrupted|orphaned staging|document received,\s*starting processing|prior interrupted/i.test(
      run.message,
    )
  );
}

function isLiveWorkingOrQueued(run: IngestionRunView): boolean {
  if (isOrphanFailedAttention(run)) return false;
  // LAW-28: keep Stopping… / Cancelled on ActiveRuns until list drops them.
  if (run.stage === "stopping" || run.stage === "cancelled") return true;
  return (
    run.stageStatus === "active" ||
    run.stageStatus === "pending" ||
    (Boolean(run.trackId) &&
      run.stage !== "completed" &&
      run.stage !== "failed" &&
      run.stageStatus !== "failed")
  );
}

/** Split live work from orphan attention shells (testable SSOT). */
export function partitionActiveRuns(runs: IngestionRunView[]): {
  working: IngestionRunView[];
  attention: IngestionRunView[];
} {
  const working: IngestionRunView[] = [];
  const attention: IngestionRunView[] = [];
  for (const run of runs) {
    if (isOrphanFailedAttention(run)) {
      attention.push(run);
    } else if (isLiveWorkingOrQueued(run)) {
      working.push(run);
    }
  }
  return { working, attention };
}

function workingSectionTitle(working: IngestionRunView[]): string {
  const anyWorking = working.some((r) => r.stageStatus === "active");
  if (anyWorking) {
    return working.length > 1 ? "Active runs" : "Active run";
  }
  return working.length > 1 ? "Queued runs" : "Queued run";
}

function renderRunCard(
  run: IngestionRunView,
  onDismissFailed: ((documentId: string) => void) | undefined,
  onCancelTrack: ((trackId: string) => void) | undefined,
) {
  return (
    <IngestionRunCard
      key={run.documentId}
      run={run}
      data-testid="spec048-active-run-card"
      onCancel={
        run.trackId &&
        run.stageStatus !== "failed" &&
        run.stage !== "stopping" &&
        run.stage !== "cancelled" &&
        onCancelTrack
          ? () => onCancelTrack(run.trackId!)
          : undefined
      }
      onDismiss={
        isOrphanFailedAttention(run) && onDismissFailed
          ? () => onDismissFailed(run.documentId)
          : undefined
      }
      nestedDetail={
        run.sourceType === "pdf" &&
        run.stage === "converting" &&
        run.trackId ? (
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
}: ActiveRunsPanelProps) {
  const queryClient = useQueryClient();
  const onCancelTrack = (trackId: string) => {
    void cancelTask(trackId)
      .catch(() => {
        /* terminal cancelled may still be on KV */
      })
      .finally(() => {
        // SPEC-086 ops: surface ui_phase=stopping / cancelled without waiting
        // for the next slow list poll.
        void queryClient.invalidateQueries({ queryKey: ["documents"] });
        void queryClient.invalidateQueries({ queryKey: ["tasks"] });
        void queryClient.invalidateQueries({ queryKey: ["pipeline-status"] });
      });
  };
  const { working, attention } = partitionActiveRuns(runs);
  if (working.length === 0 && attention.length === 0) return null;

  const dismissAll = () => {
    if (!onDismissFailed) return;
    for (const run of attention) {
      onDismissFailed(run.documentId);
    }
  };

  return (
    <div
      className="space-y-4 rounded-lg border border-sky-200/80 bg-sky-50/40 p-3 dark:border-sky-900 dark:bg-sky-950/20"
      data-testid="spec048-active-runs-panel"
    >
      {working.length > 0 && (
        <section
          className="space-y-3"
          data-testid="spec048-active-runs-working"
        >
          <div className="flex items-baseline justify-between gap-2">
            <div className="text-sm font-medium tracking-tight">
              {workingSectionTitle(working)}
            </div>
            <div className="text-xs tabular-nums text-muted-foreground">
              {working.length}
            </div>
          </div>
          {working.map((run) =>
            renderRunCard(run, onDismissFailed, onCancelTrack),
          )}
        </section>
      )}

      {attention.length > 0 && (
        <section
          className="space-y-3 rounded-md border border-amber-200/80 bg-amber-50/50 p-2.5 dark:border-amber-900/60 dark:bg-amber-950/30"
          data-testid="spec086-needs-attention"
        >
          <div className="flex items-center justify-between gap-2">
            <div className="min-w-0">
              <div className="text-sm font-medium tracking-tight">
                Needs attention
              </div>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Prior interrupted upload(s) — dismiss and re-upload. Not part of
                the current active run.
              </p>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <span className="text-xs tabular-nums text-muted-foreground">
                {attention.length}
              </span>
              {onDismissFailed && (
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
            renderRunCard(run, onDismissFailed, onCancelTrack),
          )}
        </section>
      )}
    </div>
  );
}

export default ActiveRunsPanel;
