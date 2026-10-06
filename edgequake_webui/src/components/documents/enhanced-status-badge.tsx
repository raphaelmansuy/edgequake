/**
 * @module EnhancedStatusBadge
 * @description StatusCell: pipeline status ⊕ serving fence (SPEC-099 LAW-099-3).
 *
 * WHY: Combines document status (from API) with real-time track progress (from WebSocket)
 * to provide the most detailed and accurate progress information available.
 * Fence is secondary — never a peer emerald success pill.
 *
 * @implements OODA-06: PDF page-by-page progress display
 * @implements SPEC-001/Objective-A: Chunk-level progress visibility
 * @implements SPEC-091 IS3 / LD-09: query_ready fence (semantics unchanged)
 * @implements SPEC-099 F-099-02: composite StatusCell
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { documentStalledForMs } from '@/lib/documents/document-run-state';
import { formatSilence } from '@/lib/pipeline/run-liveness';
import { formatOverallProgress } from '@/lib/utils/progress-formatter';
import {
  resolveDocumentDisplayStatus,
  resolveDocumentProgressMessage,
} from '@/lib/utils/document-status';
import { useIngestionStore } from '@/stores/use-ingestion-store';
import type { Document } from '@/types';
import { AlertTriangle } from 'lucide-react';
import { useMemo } from 'react';
import { StatusBadge } from './status-badge';

const PIPELINE_LABELS: Record<string, string> = {
  completed: 'Completed',
  indexed: 'Indexed',
  partial_success: 'Partial',
};

function pipelineLabel(status: string): string {
  return PIPELINE_LABELS[status] ?? status;
}

/**
 * Fence fragment for SPEC-091 selectors — not a peer success Badge.
 * Rendered inside the composite StatusCell when query_ready is boolean.
 */
export function ServingFenceBadge({
  queryReady,
  visuallyHidden = false,
}: {
  queryReady: boolean | null | undefined;
  /** Keep SPEC-091 selectors without duplicating visible "Ready" in compact cells. */
  visuallyHidden?: boolean;
}) {
  if (typeof queryReady !== 'boolean') return null;
  if (queryReady) {
    return (
      <span
        data-testid="spec091-serving-fence-badge"
        data-query-ready="true"
        className={visuallyHidden ? 'sr-only' : undefined}
      >
        Ready
      </span>
    );
  }
  return (
    <span
      data-testid="spec091-serving-fence-badge"
      data-query-ready="false"
      title="Indexed but not yet queryable (serving fence)"
    >
      not queryable
    </span>
  );
}

/** SPEC-155: silent "processing" is Stalled — a spinner would be a lie. */
function StalledStatusBadge({ silentMs }: { silentMs: number }) {
  const detail = `No progress for ${formatSilence(silentMs)}`;
  return (
    <div className="inline-flex flex-wrap items-center gap-1" data-testid="status-cell" data-stalled="true">
      <Badge
        variant="outline"
        className="max-w-full min-w-0 cursor-default gap-1 truncate border-amber-400 text-amber-700 dark:text-amber-400"
        title={detail}
        aria-label={`Stalled. ${detail}`}
        data-testid="status-badge"
      >
        <AlertTriangle className="h-3 w-3 shrink-0" aria-hidden="true" />
        Stalled
      </Badge>
    </div>
  );
}

interface EnhancedStatusBadgeProps {
  document: Document;
  /** Compact mode (icon only) */
  compact?: boolean;
  /** Disable tooltip (for use in other tooltips) */
  disableTooltip?: boolean;
}

/**
 * Composite StatusCell — one visual cell for pipeline ⊕ fence.
 *
 * | query_ready | paint |
 * | true | Completed · Ready (one cell) |
 * | false | Indexed · not queryable (amber secondary) |
 * | null/undefined | pipeline StatusBadge only |
 */
export function EnhancedStatusBadge({
  document,
  compact = false,
  disableTooltip = false,
}: EnhancedStatusBadgeProps) {
  const track = useIngestionStore((state) =>
    document.track_id ? state.tracks.get(document.track_id) : undefined,
  );

  const displayStatus = useMemo(
    () => resolveDocumentDisplayStatus(document),
    [document],
  );

  const progressMessage = useMemo(() => {
    const trackMessage = track ? formatOverallProgress(track) : undefined;
    return resolveDocumentProgressMessage(document, trackMessage);
  }, [track, document]);

  const progressValue = useMemo(() => {
    if (track) {
      return track.overall_progress / 100;
    }
    if (document.stage_progress !== undefined) {
      return document.stage_progress;
    }
    return undefined;
  }, [track, document.stage_progress]);

  const stalledMs = documentStalledForMs(document);
  const queryReady = document.query_ready;
  const hasFence = typeof queryReady === 'boolean';

  // Idle/completed fence composition — single cell, no peer pills.
  if (hasFence) {
    const label = queryReady
      ? `${pipelineLabel(displayStatus)} · Ready`
      : 'Indexed · not queryable';
    const ariaLabel = queryReady
      ? `${pipelineLabel(displayStatus)}, Ready for query`
      : 'Indexed, not yet queryable';
    const tone = queryReady
      ? 'text-emerald-700 border-emerald-400 dark:text-emerald-400'
      : 'text-amber-700 border-amber-400 dark:text-amber-400';

    return (
      <Badge
        variant="outline"
        className={`max-w-full min-w-0 gap-1 truncate cursor-default ${tone}`}
        data-testid="status-cell"
        aria-label={ariaLabel}
        title={
          queryReady
            ? label
            : 'Indexed but not yet queryable (serving fence)'
        }
      >
        {queryReady ? (
          compact ? (
            <span data-testid="status-badge">
              Ready
              <ServingFenceBadge queryReady={true} visuallyHidden />
            </span>
          ) : (
            <>
              <span data-testid="status-badge">{pipelineLabel(displayStatus)}</span>
              <span aria-hidden className="opacity-60">
                ·
              </span>
              <ServingFenceBadge queryReady={true} />
            </>
          )
        ) : compact ? (
          <>
            <span data-testid="status-badge">Indexed</span>
            <ServingFenceBadge queryReady={false} />
          </>
        ) : (
          <>
            <span data-testid="status-badge">Indexed</span>
            <span aria-hidden className="opacity-60">
              ·
            </span>
            <ServingFenceBadge queryReady={false} />
          </>
        )}
        {/* Visually compose full label for screen readers via aria-label; keep text scannable */}
        <span className="sr-only">{label}</span>
      </Badge>
    );
  }

  if (stalledMs !== null) {
    return <StalledStatusBadge silentMs={stalledMs} />;
  }

  return (
    <div className="inline-flex flex-wrap items-center gap-1" data-testid="status-cell">
      <StatusBadge
        status={displayStatus}
        stageMessage={progressMessage}
        stageProgressValue={progressValue}
        compact={compact}
        disableTooltip={disableTooltip}
      />
    </div>
  );
}

/**
 * Hook to get enhanced progress message for a document.
 *
 * Useful when you need just the message text without the badge component.
 */
export function useEnhancedProgressMessage(document: Document): string | undefined {
  const track = useIngestionStore((state) =>
    document.track_id ? state.tracks.get(document.track_id) : undefined,
  );

  return useMemo(() => {
    const trackMessage = track ? formatOverallProgress(track) : undefined;
    return resolveDocumentProgressMessage(document, trackMessage);
  }, [document, track]);
}
