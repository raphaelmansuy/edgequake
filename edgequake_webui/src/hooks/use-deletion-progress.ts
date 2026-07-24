/**
 * @module useDeletionProgress
 * @description Subscribes to single-doc Deletion* WebSocket events and mirrors
 * them into the delete-session SSOT for the feedback zone.
 *
 * SPEC-069: poll fallback while sessions are active (mirror wipe) so missed WS
 * still terminates the panel; long graph phase shows liveness via session fields.
 *
 * @implements SPEC-050: Delete progress parity with ingestion.
 */

'use client';

import {
  applyDeletionCompleted,
  applyDeletionFailed,
  applyDeletionPhase,
  applyDeletionStarted,
  getDeleteSessions,
  subscribeDeleteSessions,
  type DeletionSessionEntry,
} from '@/lib/documents/deletion-session';
import { getDocument, getTaskStatus } from '@/lib/api/edgequake';
import { getWebSocketClient } from '@/lib/websocket';
import type {
  DeletionCompletedEvent,
  DeletionFailedEvent,
  DeletionPhaseEvent,
  DeletionStartedEvent,
  WebSocketProgressMessage,
} from '@/types/ingestion';
import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useSyncExternalStore } from 'react';
import { toast } from 'sonner';
import { invalidateKnowledgeGraph } from '@/lib/cache-manager';

function subscribe(cb: () => void): () => void {
  return subscribeDeleteSessions(cb);
}

function getSnapshot(): DeletionSessionEntry[] {
  return getDeleteSessions();
}

function isNotFoundError(err: unknown): boolean {
  if (!err || typeof err !== 'object') return false;
  const e = err as { status?: number; statusCode?: number; message?: string };
  if (e.status === 404 || e.statusCode === 404) return true;
  const msg = (e.message || '').toLowerCase();
  return msg.includes('404') || msg.includes('not found');
}

/**
 * Reactive list of in-flight / completing delete sessions for the feedback zone.
 * Also attaches the global WS listener while any consumer is mounted.
 */
export function useDeletionSessions(): DeletionSessionEntry[] {
  const sessions = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  const queryClient = useQueryClient();

  useEffect(() => {
    const client = getWebSocketClient();

    const handleMessage = (message: WebSocketProgressMessage) => {
      if (message.type === 'DeletionStarted') {
        const ev = message as DeletionStartedEvent;
        applyDeletionStarted(ev.data.document_id);
      } else if (message.type === 'DeletionPhase') {
        const ev = message as DeletionPhaseEvent;
        applyDeletionPhase({
          documentId: ev.data.document_id,
          phase: ev.data.phase,
          phaseLabel: ev.data.phase_label,
          itemsProcessed: ev.data.items_processed,
          itemsTotal: ev.data.items_total,
        });
      } else if (message.type === 'DeletionCompleted') {
        const ev = message as DeletionCompletedEvent;
        applyDeletionCompleted({
          documentId: ev.data.document_id,
          chunksDeleted: ev.data.chunks_deleted,
          entitiesRemoved: ev.data.entities_removed,
          relationshipsRemoved: ev.data.relationships_removed,
          embeddingsDeleted: ev.data.embeddings_deleted,
          partialFailure: ev.data.partial_failure,
          error: ev.data.error,
        });
        if (ev.data.partial_failure) {
          toast.error('Document delete incomplete', {
            description:
              ev.data.error ||
              'Graph cascade reported a partial failure; document may still appear as delete_failed.',
          });
        }
        // Terminal: refresh list + KG (HTTP only admitted the job).
        queryClient.invalidateQueries({ queryKey: ['documents'] });
        invalidateKnowledgeGraph(queryClient);
      } else if (message.type === 'DeletionFailed') {
        const ev = message as DeletionFailedEvent;
        applyDeletionFailed(ev.data.document_id, ev.data.error);
        toast.error('Document delete failed', {
          description: ev.data.error || 'Cascade could not complete; document left as delete_failed.',
        });
        queryClient.invalidateQueries({ queryKey: ['documents'] });
      }
    };

    client.on('progress', handleMessage as (...args: unknown[]) => void);

    return () => {
      client.off('progress', handleMessage as (...args: unknown[]) => void);
    };
  }, [queryClient]);

  // SPEC-069: poll document / deletion task while any session is active.
  useEffect(() => {
    const active = sessions.filter((s) => s.status === 'active');
    if (active.length === 0) return;

    let cancelled = false;
    const pollOne = async (entry: DeletionSessionEntry) => {
      if (entry.trackId) {
        try {
          const task = await getTaskStatus(entry.trackId);
          if (cancelled) return;
          const status = (task.status || '').toLowerCase();
          if (status === 'failed' || status === 'cancelled') {
            applyDeletionFailed(
              entry.documentId,
              task.error_message || 'Deletion failed',
            );
            queryClient.invalidateQueries({ queryKey: ['documents'] });
            return;
          }
          if (status === 'indexed' || status === 'completed') {
            applyDeletionCompleted({
              documentId: entry.documentId,
              chunksDeleted: 0,
              entitiesRemoved: 0,
              relationshipsRemoved: 0,
              embeddingsDeleted: 0,
              partialFailure: false,
              error: null,
            });
            queryClient.invalidateQueries({ queryKey: ['documents'] });
            invalidateKnowledgeGraph(queryClient);
            return;
          }
        } catch {
          // Task may not be visible yet; fall through to document poll.
        }
      }

      try {
        const doc = await getDocument(entry.documentId);
        if (cancelled) return;
        const status = (doc.status || '').toLowerCase();
        // Only terminal delete_failed — generic `failed` is often an orphan
        // staging shell (SPEC-086) and must not abort an in-flight dismiss.
        if (status === 'delete_failed') {
          applyDeletionFailed(
            entry.documentId,
            doc.error_message ||
              doc.stage_message ||
              'Deletion failed',
          );
          queryClient.invalidateQueries({ queryKey: ['documents'] });
        }
      } catch (err) {
        if (cancelled) return;
        // Gone from catalog → treat as successful delete when WS missed terminal.
        if (isNotFoundError(err)) {
          applyDeletionCompleted({
            documentId: entry.documentId,
            chunksDeleted: 0,
            entitiesRemoved: 0,
            relationshipsRemoved: 0,
            embeddingsDeleted: 0,
            partialFailure: false,
            error: null,
          });
          queryClient.invalidateQueries({ queryKey: ['documents'] });
          invalidateKnowledgeGraph(queryClient);
        }
      }
    };

    const pollAll = async () => {
      await Promise.all(active.map((e) => pollOne(e)));
    };
    void pollAll();
    const id = window.setInterval(() => {
      void pollAll();
    }, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [sessions, queryClient]);

  return sessions;
}

/** Mark a session failed when HTTP DELETE rejects (WS may not have completed). */
export function markDeleteSessionFailed(
  documentId: string,
  error: string,
): void {
  applyDeletionFailed(documentId, error);
}
