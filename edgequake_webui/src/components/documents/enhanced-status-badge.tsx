/**
 * @module EnhancedStatusBadge
 * @description Status badge with enhanced progress information from ingestion store.
 *
 * WHY: Combines document status (from API) with real-time track progress (from WebSocket)
 * to provide the most detailed and accurate progress information available.
 *
 * @implements OODA-06: PDF page-by-page progress display
 * @implements SPEC-001/Objective-A: Chunk-level progress visibility
 * @implements DRY: Reuses document-status + progress-formatter utilities
 */
'use client';

import { formatOverallProgress } from '@/lib/utils/progress-formatter';
import {
  resolveDocumentDisplayStatus,
  resolveDocumentProgressMessage,
} from '@/lib/utils/document-status';
import { useIngestionStore } from '@/stores/use-ingestion-store';
import type { Document } from '@/types';
import { useMemo } from 'react';
import { StatusBadge } from './status-badge';

interface EnhancedStatusBadgeProps {
  document: Document;
  /** Compact mode (icon only) */
  compact?: boolean;
  /** Disable tooltip (for use in other tooltips) */
  disableTooltip?: boolean;
}

/**
 * Enhanced status badge that combines document status with track progress.
 *
 * WHY: Document status from API may be stale (updated every N seconds).
 * Track progress from WebSocket ingestion store is real-time and more granular.
 *
 * Priority:
 * 1. Track progress message (from WebSocket, most detailed)
 * 2. Document stage_message / warning (from API, backend-provided)
 * 3. Document status (from API, fallback)
 */
export function EnhancedStatusBadge({
  document,
  compact = false,
  disableTooltip = false,
}: EnhancedStatusBadgeProps) {
  // SPEC-086: subscribe to track slice so poll/WS immutable updates re-render.
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

  return (
    <StatusBadge
      status={displayStatus}
      stageMessage={progressMessage}
      stageProgressValue={progressValue}
      compact={compact}
      disableTooltip={disableTooltip}
    />
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
