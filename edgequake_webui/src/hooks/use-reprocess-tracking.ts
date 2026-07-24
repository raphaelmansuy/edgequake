/**
 * @module useReprocessTracking
 * @description Tracks in-progress reprocess operations so DocumentManager can
 * show upload-parity progress panels per reprocessed document.
 *
 * WHY (First Principles):
 *   Fresh upload → UploadingFile { trackId: <task-uuid>, isPdf } → ProgressPanelRow
 *   Reprocess     → ReprocessEntry { documentId, trackId: "reprocess_...", isPdf, mode }
 *
 *   Critical bug fixed here:
 *   - POST /documents/reprocess returns `track_id = "reprocess_YYYYMMDD_..."` (batch ID)
 *   - After 2s the worker OVERWRITES `document.track_id` with the actual task UUID
 *   - Old code keyed off the batch ID → pruneTerminalReprocessEntries never found the
 *     document (wrong track_id) → panels never dismissed
 *   - Old code passed the batch ID to progress UI → no WS events → blank panel
 *
 *   Fix: store `documentId` (stable, never changes) and use it for:
 *     1. Document lookup in pruneTerminalReprocessEntries
 *     2. Deriving the LIVE track_id from the documents cache in the render layer
 *
 * Design (SRP / DIP):
 *   - This hook owns ONLY the state for active reprocess progress entries.
 *   - It does NOT own rendering (ProgressPanelRow is rendered by the caller).
 *   - Cleanup is driven by the documents list from the existing useDocumentQueries.
 *
 * @implements SPEC-051: Reprocess progress parity with fresh upload.
 */
'use client';

import {
    getDocumentDisplayStatus,
    isTerminalStatus,
} from '@/components/documents/status-badge';
import { isProvisionalReprocessTrackId } from '@/lib/documents/reprocess-cache';
import type { Document } from '@/types';
import { useCallback, useRef, useState } from 'react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * Metadata about a reprocess in progress.
 */
export interface ReprocessEntry {
  /**
   * Stable document ID — never changes even after the worker rotates track_id.
   * Used for document lookup in prune logic and the render layer.
   */
  documentId: string;
  /** Human-readable document name shown in the progress panel. */
  documentName: string;
  /**
   * Batch track_id from POST /documents/reprocess response ("reprocess_...").
   * Used as Map key for deduplication. NOT passed to progress components —
   * callers must derive the live track_id from the documents cache instead.
   */
  trackId: string;
  /** True when source_type is "pdf". Drives nested PDF converting detail on IngestionRunCard. */
  isPdf: boolean;
  /**
   * Reprocess mode: "full" | "entities" | "merge".
   * "full" + isPdf → nest PdfUploadProgress under IngestionRunCard while converting.
   * All others → IngestionRunCard only.
   */
  mode: string;
}

/**
 * Options passed when adding a reprocess entry.
 */
export interface AddReprocessEntryOptions {
  /** Stable document ID. Required. */
  documentId: string;
  /** True when source_type is "pdf". */
  isPdf?: boolean;
  /** Reprocess mode. Defaults to "entities". */
  mode?: string;
}

/**
 * Whether a reprocess entry should nest PDF converting page detail.
 * Entities-only and non-PDF use IngestionRunCard without PDF page slot.
 */
export function shouldUsePdfReprocessPanel(
  isPdf: boolean,
  mode: string | undefined,
): boolean {
  return isPdf && mode === 'full';
}

/**
 * Return type for useReprocessTracking.
 */
export interface UseReprocessTrackingReturn {
  /** All currently-active reprocess entries (unordered). */
  reprocessEntries: ReprocessEntry[];

  /**
   * Add a new reprocess entry.
   * Idempotent by documentId: a second call with a new trackId upgrades the entry
   * (provisional → live pdf_processing-…).
   *
   * @param documentName - Display name for the progress panel.
   * @param trackId      - Provisional or live progress track id.
   * @param options      - documentId (required), isPdf, mode.
   */
  addReprocessEntry: (
    documentName: string,
    trackId: string,
    options: AddReprocessEntryOptions,
  ) => void;

  /**
   * Explicitly remove a single entry by its batch trackId.
   * Removal is deferred by 3s so the user sees the terminal state.
   */
  removeReprocessEntry: (trackId: string) => void;

  /**
   * Immediately remove an entry by documentId (skip/error rollback).
   */
  removeReprocessEntryByDocumentId: (documentId: string) => void;

  /**
   * Prune entries whose backing document has reached a terminal state.
   * Uses documentId (not trackId) for lookup — survives track_id rotation by worker.
   */
  pruneTerminalReprocessEntries: (docs: Document[]) => void;
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useReprocessTracking(): UseReprocessTrackingReturn {
  const [entries, setEntries] = useState<Map<string, ReprocessEntry>>(
    () => new Map(),
  );
  /** Docs the user dismissed — suppress bindLiveTask re-mount until a new admit. */
  const dismissedDocIdsRef = useRef<Set<string>>(new Set());

  const addReprocessEntry = useCallback(
    (
      documentName: string,
      trackId: string,
      options: AddReprocessEntryOptions,
    ) => {
      const isProvisional = isProvisionalReprocessTrackId(trackId);
      if (isProvisional) {
        // New admit clears a prior dismiss for this document.
        dismissedDocIdsRef.current.delete(options.documentId);
      } else if (dismissedDocIdsRef.current.has(options.documentId)) {
        // User dismissed during Queuing; do not re-add on HTTP success bind.
        return;
      }

      setEntries((prev) => {
        const existing = [...prev.entries()].find(
          ([, e]) => e.documentId === options.documentId,
        );
        if (existing) {
          const [oldKey, oldEntry] = existing;
          if (oldEntry.trackId === trackId) return prev;
          // Upgrade provisional → live (or replace track id) without duplicate rows.
          const next = new Map(prev);
          next.delete(oldKey);
          next.set(trackId, {
            ...oldEntry,
            documentName: documentName || oldEntry.documentName,
            trackId,
            isPdf: options.isPdf ?? oldEntry.isPdf,
            mode: options.mode ?? oldEntry.mode,
          });
          return next;
        }
        const next = new Map(prev);
        next.set(trackId, {
          documentId: options.documentId,
          documentName,
          trackId,
          isPdf: options.isPdf ?? false,
          mode: options.mode ?? 'entities',
        });
        return next;
      });
    },
    [],
  );

  const removeReprocessEntry = useCallback((trackId: string) => {
    // Keep completed/failed panels visible for 3s (same pattern as upload).
    setTimeout(() => {
      setEntries((prev) => {
        if (!prev.has(trackId)) return prev;
        const next = new Map(prev);
        next.delete(trackId);
        return next;
      });
    }, 3000);
  }, []);

  const removeReprocessEntryByDocumentId = useCallback((documentId: string) => {
    dismissedDocIdsRef.current.add(documentId);
    setEntries((prev) => {
      const match = [...prev.entries()].find(
        ([, e]) => e.documentId === documentId,
      );
      if (!match) return prev;
      const next = new Map(prev);
      next.delete(match[0]);
      return next;
    });
  }, []);

  const pruneTerminalReprocessEntries = useCallback((docs: Document[]) => {
    if (!docs.length) return;
    setEntries((prev) => {
      if (prev.size === 0) return prev;
      for (const [trackId, entry] of prev) {
        // Never prune client-only "Queuing…" rows — admit may still be in flight
        // while list status briefly looks terminal without pin protection.
        if (isProvisionalReprocessTrackId(entry.trackId)) continue;
        // FIX: look up by documentId, not by track_id.
        // WHY: the worker overwrites document.track_id with the actual task UUID
        // after 2s. Looking up by the original "reprocess_..." batch track_id
        // would never find the document → panels stuck forever.
        const match = docs.find((d) => d.id === entry.documentId);
        if (!match) continue;
        const displayStatus = getDocumentDisplayStatus(match);
        if (isTerminalStatus(displayStatus)) {
          const captured = trackId;
          setTimeout(() => {
            setEntries((p) => {
              if (!p.has(captured)) return p;
              const n = new Map(p);
              n.delete(captured);
              return n;
            });
          }, 3000);
        }
      }
      return prev;
    });
  }, []);

  return {
    reprocessEntries: [...entries.values()],
    addReprocessEntry,
    removeReprocessEntry,
    removeReprocessEntryByDocumentId,
    pruneTerminalReprocessEntries,
  };
}
