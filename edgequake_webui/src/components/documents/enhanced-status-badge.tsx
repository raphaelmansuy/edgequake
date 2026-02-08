/**
 * @module EnhancedStatusBadge
 * @description Status badge with enhanced progress information from ingestion store.
 * 
 * WHY: Combines document status (from API) with real-time track progress (from WebSocket)
 * to provide the most detailed and accurate progress information available.
 * 
 * @implements OODA-06: PDF page-by-page progress display
 * @implements SPEC-001/Objective-A: Chunk-level progress visibility
 * @implements DRY: Reuses formatOverallProgress utility
 */
'use client';

import { formatOverallProgress } from '@/lib/utils/progress-formatter';
import { useIngestionStore } from '@/stores/use-ingestion-store';
import type { Document } from '@/types';
import { useMemo } from 'react';
import { StatusBadge, getDocumentDisplayStatus } from './status-badge';

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
 * 2. Document stage_message (from API, backend-provided)
 * 3. Document status (from API, fallback)
 */
export function EnhancedStatusBadge({ 
  document, 
  compact = false,
  disableTooltip = false,
}: EnhancedStatusBadgeProps) {
  // Get track from ingestion store if available
  const getTrack = useIngestionStore((state) => state.getTrack);
  const track = useMemo(
    () => (document.track_id ? getTrack(document.track_id) : undefined),
    [document.track_id, getTrack]
  );

  // Determine best status to display
  const displayStatus = useMemo(() => {
    const baseStatus = getDocumentDisplayStatus(document);
    
    // Priority 0: If document has error_message, show as failed
    // WHY: Type guard prevents TypeError when backend sends {} instead of string
    if (
      document.error_message && 
      typeof document.error_message === 'string' && 
      document.error_message.trim() !== ''
    ) {
      // Note: Error message is displayed in UI via progressMessage, no need to log
      return 'failed';
    }
    
    // WHY: Fix confusing status where stage is "complete" but status shows previous stage
    // If stage_message says "complete" but status is still on that stage, show as transitioning
    if (document.stage_message) {
      const msg = document.stage_message.toLowerCase();
      
      // PDF conversion complete → should show next stage (Chunking)
      if (baseStatus === 'converting' && (msg.includes('complete') || msg.includes('extracted'))) {
        return 'chunking'; // Transition to next stage
      }
      
      // Chunking complete → should show next stage (Extracting)
      if (baseStatus === 'chunking' && msg.includes('complete')) {
        return 'extracting';
      }
      
      // Extracting complete → should show next stage (Embedding)
      if (baseStatus === 'extracting' && msg.includes('complete')) {
        return 'embedding';
      }
      
      // Embedding complete → should show next stage (Storing)
      if (baseStatus === 'embedding' && msg.includes('complete')) {
        return 'storing';
      }
    }
    
    return baseStatus;
  }, [document]);

  // Determine best progress message to display
  const progressMessage = useMemo(() => {
    // Priority 0: Error message (highest priority)
    // WHY: Type guard prevents TypeError when backend sends {} instead of string
    if (
      document.error_message && 
      typeof document.error_message === 'string' && 
      document.error_message.trim() !== ''
    ) {
      return `Error: ${document.error_message}`;
    }
    
    // Priority 1: Track progress message (most detailed, real-time)
    if (track) {
      const trackMessage = formatOverallProgress(track);
      if (trackMessage) {
        return trackMessage;
      }
    }

    // Priority 2: Document stage_message (backend-provided)
    if (document.stage_message) {
      return document.stage_message;
    }

    // Priority 3: No custom message (StatusBadge will show default)
    return undefined;
  }, [track, document.stage_message, document.error_message]);

  // Get progress value (0.0 to 1.0)
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
  const getTrack = useIngestionStore((state) => state.getTrack);
  
  return useMemo(() => {
    // Priority 1: Track progress message
    if (document.track_id) {
      const track = getTrack(document.track_id);
      if (track) {
        const trackMessage = formatOverallProgress(track);
        if (trackMessage) {
          return trackMessage;
        }
      }
    }

    // Priority 2: Document stage_message
    if (document.stage_message) {
      return document.stage_message;
    }

    return undefined;
  }, [document.track_id, document.stage_message, getTrack]);
}
