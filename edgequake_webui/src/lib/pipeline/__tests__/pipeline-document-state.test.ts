import { describe, expect, it } from 'bun:test';
import type { Document } from '@/types';
import {
  buildIngestionAlertHeadline,
} from '../ingestion-alert-presenter';
import { translateIngestionDetail } from '../ingestion-user-messages';
import {
  detectStuckDocuments,
  hasQueueCoverage,
  isActiveProcessingStatus,
  isOrphanAdmissionShell,
  isWaitingStatus,
  needsReuploadNotReprocess,
  orphanQueuedTaskCount,
  resolvePipelineUiState,
  summarizePipelineDocuments,
} from '../pipeline-document-state';

function doc(overrides: Partial<Document> & { id: string }): Document {
  const { id, ...rest } = overrides;
  // Infer a non-terminal status when only current_stage is set (terminal status
  // now wins over stale stages in getDocumentDisplayStatus).
  const stage = rest.current_stage?.toLowerCase();
  const terminalStages = new Set([
    'completed',
    'indexed',
    'failed',
    'cancelled',
    'partial_failure',
    'partial_success',
  ]);
  const inferredStatus =
    rest.status ??
    (stage
      ? terminalStages.has(stage)
        ? stage
        : 'processing'
      : 'completed');
  return {
    id,
    title: id,
    file_name: `${id}.pdf`,
    status: inferredStatus,
    ...rest,
  } as Document;
}

const t = (key: string, defaultValue: string, options?: Record<string, unknown>) => {
  let text = defaultValue;
  if (options) {
    for (const [name, value] of Object.entries(options)) {
      text = text.replace(`{{${name}}}`, String(value));
    }
  }
  return text;
};

describe('pipeline-document-state', () => {
  it('classifies pending as waiting, not active', () => {
    expect(isWaitingStatus('pending')).toBe(true);
    expect(isActiveProcessingStatus('pending')).toBe(false);
  });

  it('classifies extracting as active processing', () => {
    expect(isWaitingStatus('extracting')).toBe(false);
    expect(isActiveProcessingStatus('extracting')).toBe(true);
  });

  it('summarizes active vs waiting documents', () => {
    const summary = summarizePipelineDocuments([
      doc({ id: 'a', current_stage: 'extracting' }),
      doc({ id: 'b', current_stage: 'pending', stage_message: 'Auto-recovered' }),
      doc({ id: 'c', status: 'completed' }),
    ]);

    expect(summary.activeCount).toBe(1);
    expect(summary.waitingCount).toBe(1);
    expect(summary.activeDocs.map((d) => d.id)).toEqual(['a']);
    expect(summary.waitingDocs.map((d) => d.id)).toEqual(['b']);
  });

  it('treats completed status as terminal even when current_stage is stale', () => {
    const summary = summarizePipelineDocuments([
      doc({
        id: 'done',
        status: 'completed',
        current_stage: 'extracting',
        stage_message: 'Extracting entities…',
      }),
    ]);

    expect(summary.activeCount).toBe(0);
    expect(summary.waitingCount).toBe(0);
  });

  it('computes orphan queued tasks not tied to waiting docs', () => {
    expect(orphanQueuedTaskCount(3, 1)).toBe(2);
    expect(orphanQueuedTaskCount(1, 2)).toBe(0);
    expect(orphanQueuedTaskCount(2, 0, 1)).toBe(1);
    expect(orphanQueuedTaskCount(1, 0, 1)).toBe(0);
  });

  it('detects stuck waiting docs when task queue is idle and recovery signal present', () => {
    const state = resolvePipelineUiState(
      [
        doc({
          id: 'deep',
          current_stage: 'pending',
          stage_message: 'Auto-recovered after server restart',
          updated_at: '2026-01-01T00:00:00Z',
        }),
      ],
      { is_busy: false, pending_tasks: 0, processing_tasks: 0 },
    );

    expect(state.isStuck).toBe(true);
    expect(state.isQueuedOnly).toBe(false);
    expect(state.alertMode).toBe('stuck');
    expect(state.stuckDocCount).toBe(1);
    expect(state.showPipelineIndicator).toBe(true);
  });

  it('does NOT show red stuck banner for a fresh upload without tasks yet', () => {
    const nowIso = new Date().toISOString();
    const state = resolvePipelineUiState(
      [
        doc({
          id: 'chanel',
          file_name: 'Chanel_Loop.pdf',
          current_stage: 'pending',
          status: 'pending',
          stage_message: 'Waiting for a processing slot',
          created_at: nowIso,
          updated_at: nowIso,
        }),
      ],
      { is_busy: false, pending_tasks: 0, processing_tasks: 0 },
    );

    expect(state.isStuck).toBe(false);
    expect(state.isQueuedOnly).toBe(true);
    expect(state.alertMode).toBe('queued');
    expect(state.showPipelineIndicator).toBe(true);
  });

  it('does NOT mark track-admitted waiting docs as stuck when aged', () => {
    const state = resolvePipelineUiState(
      [
        doc({
          id: 'tracked',
          current_stage: 'queued',
          status: 'pending',
          track_id: 'track-1',
          updated_at: '2026-01-01T00:00:00Z',
        }),
      ],
      { is_busy: false, pending_tasks: 0, processing_tasks: 0 },
    );

    expect(state.isStuck).toBe(false);
    expect(state.alertMode).toBe('queued');
  });

  it('marks aged waiting docs without track or recovery as stuck', () => {
    const state = resolvePipelineUiState(
      [
        doc({
          id: 'orphan',
          current_stage: 'pending',
          status: 'pending',
          updated_at: '2026-01-01T00:00:00Z',
        }),
      ],
      { is_busy: false, pending_tasks: 0, processing_tasks: 0 },
    );

    expect(state.isStuck).toBe(true);
    expect(state.alertMode).toBe('stuck');
  });

  it('treats waiting docs as queued when pending tasks exist', () => {
    const state = resolvePipelineUiState(
      [doc({ id: 'deep', current_stage: 'pending' })],
      { is_busy: false, pending_tasks: 1, processing_tasks: 0 },
    );

    expect(state.isStuck).toBe(false);
    expect(state.isQueuedOnly).toBe(true);
    expect(state.alertMode).toBe('queued');
  });

  it('resolvePipelineUiState treats active docs as processing not waiting', () => {
    const state = resolvePipelineUiState(
      [doc({ id: 'a', current_stage: 'extracting' })],
      { pending_tasks: 2, processing_tasks: 1, is_busy: true },
    );

    expect(state.isActivelyProcessing).toBe(true);
    expect(state.isQueuedOnly).toBe(false);
    expect(state.alertMode).toBe('working');
    expect(state.activeDocCount).toBe(1);
  });

  it('does not show Processing 0 when only is_busy with no active docs or tasks', () => {
    const state = resolvePipelineUiState(
      [doc({ id: 'done', status: 'completed' })],
      { is_busy: true, pending_tasks: 0, processing_tasks: 0 },
    );

    expect(state.isActivelyProcessing).toBe(false);
    expect(state.showPipelineIndicator).toBe(false);
    expect(state.activeDocCount).toBe(0);
  });

  it('hides processing banner when all docs are ingested despite stale processing_tasks', () => {
    const state = resolvePipelineUiState(
      [
        doc({ id: 'a', status: 'completed', current_stage: 'completed' }),
        doc({ id: 'b', status: 'completed', current_stage: 'storing' }),
      ],
      { is_busy: true, pending_tasks: 0, processing_tasks: 1, running_tasks: 1 },
    );

    expect(state.isActivelyProcessing).toBe(false);
    expect(state.showPipelineIndicator).toBe(false);
    expect(state.activeDocCount).toBe(0);
    expect(state.alertMode).not.toBe('working');
  });

  it('uses running task count when docs lag behind the queue', () => {
    const state = resolvePipelineUiState([], {
      is_busy: true,
      pending_tasks: 0,
      processing_tasks: 2,
    });

    expect(state.isActivelyProcessing).toBe(true);
    expect(state.alertMode).toBe('working');
    expect(state.activeDocCount).toBe(2);
    expect(state.showPipelineIndicator).toBe(true);
  });

  it('uses mixed alert mode when active and waiting docs coexist', () => {
    const state = resolvePipelineUiState(
      [
        doc({ id: 'a', current_stage: 'extracting' }),
        doc({ id: 'b', current_stage: 'pending' }),
      ],
      { pending_tasks: 1, processing_tasks: 1, is_busy: true },
    );

    expect(state.alertMode).toBe('mixed');
    expect(state.isStuck).toBe(false);
  });

  it('hasQueueCoverage reflects pending, processing, and busy flags', () => {
    expect(hasQueueCoverage(undefined, 0, 0)).toBe(false);
    expect(hasQueueCoverage({ is_busy: true }, 0, 0)).toBe(true);
    expect(hasQueueCoverage(undefined, 1, 0)).toBe(true);
    expect(hasQueueCoverage(undefined, 0, 1)).toBe(true);
  });

  it('detectStuckDocuments ignores fresh waiting docs without recovery signal', () => {
    const now = Date.now();
    const summary = summarizePipelineDocuments([
      doc({
        id: 'fresh',
        current_stage: 'pending',
        created_at: new Date(now).toISOString(),
        updated_at: new Date(now).toISOString(),
      }),
    ]);
    expect(detectStuckDocuments(summary, false, now)).toEqual([]);
  });

  it('detectStuckDocuments returns aged waiting docs without queue coverage', () => {
    const summary = summarizePipelineDocuments([
      doc({
        id: 'w',
        current_stage: 'pending',
        updated_at: '2026-01-01T00:00:00Z',
      }),
    ]);
    expect(detectStuckDocuments(summary, false).map((d) => d.id)).toEqual(['w']);
    expect(detectStuckDocuments(summary, true)).toEqual([]);
  });

  it('aged uploading admission seed is orphan — not Working', () => {
    const now = Date.now();
    const orphan = doc({
      id: 'invarian',
      file_name: 'invarian_2607.11875v2.md',
      current_stage: 'uploading',
      stage_message: 'Document received, starting processing',
      stage_progress: 0,
      track_id: 'insert-dead',
      admission_staging: true,
      updated_at: new Date(now - 120_000).toISOString(),
      created_at: new Date(now - 120_000).toISOString(),
    });
    expect(isOrphanAdmissionShell(orphan, now)).toBe(true);
    // Queued behind busy workers: not Needs attention.
    expect(
      isOrphanAdmissionShell(orphan, now, { hasQueueCoverage: true }),
    ).toBe(false);
    const summary = summarizePipelineDocuments([orphan]);
    expect(summary.activeCount).toBe(0);
    expect(summary.waitingCount).toBe(1);
    expect(detectStuckDocuments(summary, false, now).map((d) => d.id)).toEqual([
      'invarian',
    ]);
    expect(detectStuckDocuments(summary, true, now)).toEqual([]);
    const state = resolvePipelineUiState([orphan], {
      pending_tasks: 0,
      processing_tasks: 0,
    });
    expect(state.isActivelyProcessing).toBe(false);
    expect(state.isStuck).toBe(true);
    expect(state.alertMode).toBe('stuck');
    const queuedBehind = resolvePipelineUiState([orphan], {
      pending_tasks: 1,
      processing_tasks: 1,
      is_busy: true,
    });
    expect(queuedBehind.isStuck).toBe(false);
    expect(queuedBehind.isActivelyProcessing).toBe(true);
  });

  it('slow client upload without admission_staging is not orphan after grace', () => {
    const now = Date.now();
    const slowClient = doc({
      id: 'slow-client',
      file_name: 'notes.md',
      current_stage: 'uploading',
      stage_message: 'Queued for processing…',
      stage_progress: 0,
      // No admission_staging — optimistic client shell only
      updated_at: new Date(now - 120_000).toISOString(),
    });
    expect(isOrphanAdmissionShell(slowClient, now)).toBe(false);
  });

  it('needsReuploadNotReprocess detects recovered staging shells', () => {
    expect(
      needsReuploadNotReprocess(
        doc({
          id: 'r',
          status: 'failed',
          failure_code: 'server_restart_interrupted',
          error_message: 'Orphaned staging admission — please re-upload',
        }),
      ),
    ).toBe(true);
  });

  it('fresh uploading admission is not orphan within grace', () => {
    const now = Date.now();
    const fresh = doc({
      id: 'fresh-up',
      current_stage: 'uploading',
      stage_message: 'Document received, starting processing',
      admission_staging: true,
      track_id: 'insert-live',
      updated_at: new Date(now - 5_000).toISOString(),
      created_at: new Date(now - 5_000).toISOString(),
    });
    expect(isOrphanAdmissionShell(fresh, now)).toBe(false);
    const summary = summarizePipelineDocuments([fresh]);
    expect(summary.activeCount).toBe(1);
  });

  it('aged uploading seed with busy queue is Working, not Needs attention', () => {
    // SPEC-086 ops: global queue coverage suppresses false-orphan while PDF extracts.
    const now = Date.now();
    const queuedSeed = doc({
      id: 'invarian',
      file_name: 'invarian.md',
      current_stage: 'uploading',
      stage_message: 'Document received, starting processing',
      admission_staging: true,
      track_id: 'insert-queued',
      updated_at: new Date(now - 120_000).toISOString(),
    });
    const live = doc({
      id: 'live',
      current_stage: 'extracting',
      stage_message: 'Extracting — 4/98',
      track_id: 'insert-live',
      updated_at: new Date(now).toISOString(),
    });
    const state = resolvePipelineUiState([queuedSeed, live], {
      pending_tasks: 1,
      processing_tasks: 1,
      is_busy: true,
    });
    expect(state.isActivelyProcessing).toBe(true);
    expect(state.stuckDocs).toEqual([]);
    expect(state.isStuck).toBe(false);
    expect(state.activeDocCount).toBeGreaterThanOrEqual(1);
  });

  it('aged pending with track_id stays Queued (SPEC-048); orphan uploading does not', () => {
    const now = Date.now();
    const pendingWithTrack = doc({
      id: 'stale-track',
      current_stage: 'pending',
      track_id: 'insert-gone',
      updated_at: '2026-01-01T00:00:00Z',
    });
    const summaryPending = summarizePipelineDocuments([pendingWithTrack]);
    expect(detectStuckDocuments(summaryPending, false)).toEqual([]);

    const orphan = doc({
      id: 'orphan-up',
      current_stage: 'uploading',
      stage_message: 'Document received, starting processing',
      admission_staging: true,
      track_id: 'insert-dead',
      updated_at: new Date(now - 120_000).toISOString(),
    });
    const summaryOrphan = summarizePipelineDocuments([orphan]);
    expect(detectStuckDocuments(summaryOrphan, false, now).map((d) => d.id)).toEqual([
      'orphan-up',
    ]);
  });
});

describe('ingestion-user-messages', () => {
  it('translates auto-recovered message for stuck context', () => {
    const message = translateIngestionDetail(
      doc({
        id: 'deep',
        file_name: 'deep_2604.pdf',
        stage_message: 'Auto-recovered after server restart (was in pending stage)',
      }),
      'stuck',
    );
    expect(message).toContain('no worker is processing');
    expect(message).toContain('reprocess');
  });

  it('translates auto-recovered message for queued context', () => {
    const message = translateIngestionDetail(
      doc({
        id: 'deep',
        stage_message: 'Auto-recovered after server restart',
      }),
      'queued',
    );
    expect(message).toContain('Resuming from checkpoint');
  });

  it('translates graph merge stage message with live relationship counters', () => {
    const message = translateIngestionDetail(
      doc({
        id: 'deep',
        file_name: 'deep_2604.26962v2.pdf',
        stage_message:
          'Storing in knowledge graph — RelationshipGraph (2654/2654 entities (100%), 128/1999 relationships (6%))',
      }),
      'active',
    );
    expect(message).toContain('Saving relationships');
    expect(message).toContain('128');
    expect(message).toContain('(6%)');
    expect(message).not.toContain('Storing in knowledge graph');
  });
});

describe('ingestion-alert-presenter', () => {
  it('builds stuck headline copy', () => {
    const state = resolvePipelineUiState(
      [
        doc({
          id: 'x',
          current_stage: 'pending',
          stage_message: 'Auto-recovered after server restart',
          updated_at: '2026-01-01T00:00:00Z',
        }),
      ],
      { pending_tasks: 0, processing_tasks: 0 },
    );
    const headline = buildIngestionAlertHeadline(state, t);
    expect(headline.dataTestId).toBe('ingestion-alert-stuck');
    expect(headline.text).toContain('need attention');
    expect(headline.showAlert).toBe(true);
  });

  it('builds queued headline for fresh upload (not stuck)', () => {
    const nowIso = new Date().toISOString();
    const state = resolvePipelineUiState(
      [
        doc({
          id: 'new',
          current_stage: 'pending',
          created_at: nowIso,
          updated_at: nowIso,
        }),
      ],
      { pending_tasks: 0, processing_tasks: 0 },
    );
    const headline = buildIngestionAlertHeadline(state, t);
    expect(headline.dataTestId).toBe('ingestion-alert-queued');
    expect(headline.showAlert).toBe(false);
  });
});
