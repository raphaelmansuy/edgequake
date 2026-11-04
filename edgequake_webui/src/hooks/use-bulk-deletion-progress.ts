/**
 * @module useBulkDeletionProgress
 * @description Listens for BulkDeletion* WebSocket events and tracks per-document progress.
 *
 * WHY: The bulk delete endpoint is a single HTTP call but the server broadcasts
 * per-document progress via WebSocket. This hook correlates by `wipe_track_id`
 * and falls back to task-status polling after reconnect / missed events.
 *
 * @implements SPEC-050: Bulk delete progress (AC-050-05).
 */
'use client';

import { getTaskStatus } from '@/lib/api/edgequake';
import { getWebSocketClient } from '@/lib/websocket';
import type {
  BulkDeletionCompletedEvent,
  BulkDeletionFailedEvent,
  BulkDeletionItemProgressEvent,
  BulkDeletionStartedEvent,
  WebSocketProgressMessage,
} from '@/types/ingestion';
import { useCallback, useEffect, useRef, useState } from 'react';

export interface BulkDeletionItemState {
  document_id: string;
  entities_removed: number;
  relationships_removed: number;
}

export interface BulkDeletionProgressState {
  total: number;
  completed: number;
  skipped: number;
  items: BulkDeletionItemState[];
  isComplete: boolean;
  isFailed: boolean;
  errorMessage?: string;
  totalEntitiesRemoved: number;
  totalRelationshipsRemoved: number;
  wipeTrackId?: string | null;
}

const initialState = (
  wipeTrackId?: string | null,
): BulkDeletionProgressState => ({
  total: 0,
  completed: 0,
  skipped: 0,
  items: [],
  isComplete: false,
  isFailed: false,
  errorMessage: undefined,
  totalEntitiesRemoved: 0,
  totalRelationshipsRemoved: 0,
  wipeTrackId: wipeTrackId ?? null,
});

/** Exported for unit tests — wipe ID correlation for bulk deletion WS events. */
export function matchesWipe(
  eventWipeId: string | undefined | null,
  expected: string | null | undefined,
): boolean {
  // Until admit returns a track id, accept unscoped events.
  if (!expected) return true;
  // Once expected is set, require an exact match (ignore legacy unscoped events).
  if (!eventWipeId) return false;
  return eventWipeId === expected;
}

/**
 * Tracks bulk deletion progress from WebSocket events (+ optional task poll).
 *
 * @param enabled - Whether to listen for events
 * @param wipeTrackId - Durable wipe correlation id from HTTP 202 admit
 */
export function useBulkDeletionProgress(
  enabled: boolean,
  wipeTrackId?: string | null,
): BulkDeletionProgressState & { reset: () => void } {
  const [state, setState] = useState<BulkDeletionProgressState>(() =>
    initialState(wipeTrackId),
  );
  const wipeRef = useRef(wipeTrackId);
  // WHY in an effect and not during render: this ref mirrors a prop for use *inside the WebSocket
  // message handler below*, which runs outside render. Writing it during render is what the
  // `refs` rule forbids; an effect assignment is the documented way to mirror a prop for a
  // callback, and by the time a message arrives the effect has run.
  useEffect(() => {
    wipeRef.current = wipeTrackId;
  }, [wipeTrackId]);

  const reset = useCallback(() => {
    setState(initialState(wipeRef.current));
  }, []);

  // Keep the reported track id in step with the prop without a second render.
  //
  // WHY during render: the previous effect re-ran `setState` on every prop change, cascading a
  // render whose only job was to copy a value the component already had. Comparing during render
  // does the same work in the render that is already happening.
  const [stateTrackId, setStateTrackId] = useState(wipeTrackId ?? null);
  if (stateTrackId !== (wipeTrackId ?? null)) {
    setStateTrackId(wipeTrackId ?? null);
    setState((prev) => ({ ...prev, wipeTrackId: wipeTrackId ?? null }));
  }

  useEffect(() => {
    if (!enabled) return;

    const client = getWebSocketClient();

    const handleMessage = (message: WebSocketProgressMessage) => {
      const expected = wipeRef.current;
      if (message.type === 'BulkDeletionStarted') {
        const ev = message as BulkDeletionStartedEvent;
        if (!matchesWipe(ev.data.wipe_track_id, expected)) return;
        setState((prev) => ({
          ...prev,
          total: ev.data.total,
          completed: 0,
          items: [],
          isComplete: false,
          isFailed: false,
          errorMessage: undefined,
          wipeTrackId: ev.data.wipe_track_id ?? prev.wipeTrackId,
        }));
      } else if (message.type === 'BulkDeletionItemProgress') {
        const ev = message as BulkDeletionItemProgressEvent;
        if (!matchesWipe(ev.data.wipe_track_id, expected)) return;
        setState((prev) => ({
          ...prev,
          completed: ev.data.completed,
          total: ev.data.total,
          items: [
            ...prev.items.filter((i) => i.document_id !== ev.data.document_id),
            {
              document_id: ev.data.document_id,
              entities_removed: ev.data.entities_removed,
              relationships_removed: ev.data.relationships_removed,
            },
          ],
        }));
      } else if (message.type === 'BulkDeletionCompleted') {
        const ev = message as BulkDeletionCompletedEvent;
        if (!matchesWipe(ev.data.wipe_track_id, expected)) return;
        setState((prev) => ({
          ...prev,
          completed: ev.data.deleted_count,
          skipped: ev.data.skipped_count,
          totalEntitiesRemoved: ev.data.total_entities_removed,
          totalRelationshipsRemoved: ev.data.total_relationships_removed,
          isComplete: true,
          isFailed: false,
        }));
      } else if (message.type === 'BulkDeletionFailed') {
        const ev = message as BulkDeletionFailedEvent;
        if (!matchesWipe(ev.data.wipe_track_id, expected)) return;
        setState((prev) => ({
          ...prev,
          completed: ev.data.deleted_count ?? prev.completed,
          isComplete: true,
          isFailed: true,
          errorMessage: ev.data.error_message,
        }));
      }
    };

    client.on('progress', handleMessage as (...args: unknown[]) => void);
    return () => {
      client.off('progress', handleMessage as (...args: unknown[]) => void);
    };
  }, [enabled]);

  // Poll task status as reconnect / missed-WS fallback.
  useEffect(() => {
    if (!enabled || !wipeTrackId) return;
    let cancelled = false;
    const poll = async () => {
      try {
        const task = await getTaskStatus(wipeTrackId);
        if (cancelled) return;
        const status = (task.status || '').toLowerCase();
        if (status === 'indexed' || status === 'completed') {
          const deleted =
            typeof task.result?.deleted_count === 'number'
              ? task.result.deleted_count
              : undefined;
          setState((prev) => ({
            ...prev,
            isComplete: true,
            isFailed: false,
            completed: prev.completed || deleted || prev.completed,
          }));
        } else if (status === 'failed' || status === 'cancelled') {
          setState((prev) => ({
            ...prev,
            isComplete: true,
            isFailed: true,
            errorMessage: task.error_message || 'Workspace wipe failed',
          }));
        }
      } catch {
        // Task may not be visible yet; keep waiting for WS.
      }
    };
    void poll();
    const id = window.setInterval(() => {
      void poll();
    }, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [enabled, wipeTrackId]);

  return { ...state, reset };
}
