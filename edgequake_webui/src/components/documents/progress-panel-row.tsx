'use client';

/**
 * @module ProgressPanelRow
 * @description SPEC-086: one IngestionRunCard for all formats; PDF page detail nested under converting.
 *
 * @implements SPEC-051: Upload-parity progress for reprocess.
 * @implements SPEC-054: Admission cleaning vs queued presenters.
 * @implements SPEC-086: Format-agnostic progress presenter.
 */

import { AdmissionPhaseRow } from '@/components/documents/admission-phase-row';
import { IngestionRunCard } from '@/components/documents/ingestion-run-card';
import { PdfUploadProgress } from '@/components/documents/pdf-upload-progress';
import { Button } from '@/components/ui/button';
import { useIngestionProgress } from '@/hooks/use-ingestion-progress';
import { shouldShowReprocessQueuingPanel } from '@/lib/documents/reprocess-cache';
import { buildIngestionRunViewFromProgress } from '@/lib/pipeline/ingestion-run-view';
import { sourceTypeFromFileName } from '@/lib/upload/file-kind';
import { X } from 'lucide-react';
import { useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';

export interface ProgressPanelRowProps {
  trackId: string;
  documentName: string;
  /**
   * When true, nest PDF converting detail under the shared stepper.
   */
  isPdf?: boolean;
  currentStage?: string | null;
  stageMessage?: string | null;
  onRemove?: () => void;
  onComplete?: () => void;
  onFailed?: (error: string) => void;
  onCancel?: () => void;
  'data-testid'?: string;
  'data-track-id'?: string;
}

function resolveAdmissionPhase(
  currentStage?: string | null,
): 'cleaning' | 'queued' {
  return (currentStage || '').toLowerCase() === 'cleaning'
    ? 'cleaning'
    : 'queued';
}

/**
 * A single progress row used in both UploadProgressList and the reprocess
 * panels section in DocumentManager.
 */
export function ProgressPanelRow({
  trackId,
  documentName,
  isPdf = false,
  currentStage,
  stageMessage,
  onRemove,
  onComplete,
  onFailed,
  onCancel,
  'data-testid': testId,
  'data-track-id': dataTrackId,
}: ProgressPanelRowProps) {
  const { t } = useTranslation();
  const isAdmission = shouldShowReprocessQueuingPanel(trackId);
  const admissionPhase = resolveAdmissionPhase(currentStage);
  const dismissHint = t(
    'documents.reprocess.dismissHint',
    'Hides progress; processing continues.',
  );

  const { progress, cancel } = useIngestionProgress(
    isAdmission ? null : trackId,
    {
      documentId: trackId,
      documentName,
    },
  );

  const run = useMemo(() => {
    if (!progress) return null;
    const sourceType = isPdf
      ? 'pdf'
      : sourceTypeFromFileName(documentName);
    return buildIngestionRunViewFromProgress(progress, {
      sourceType,
      filename: documentName,
    });
  }, [progress, isPdf, documentName]);

  useEffect(() => {
    if (!progress) return;
    if (progress.status === 'completed') onComplete?.();
    if (progress.status === 'failed') {
      onFailed?.(progress.progress.latest_message || 'Failed');
    }
  }, [progress, onComplete, onFailed]);

  const handleCancel = () => {
    // Always cancel the live track via WS; optional parent dismiss callback.
    cancel();
    onCancel?.();
  };

  return (
    <div
      className="relative p-2 rounded-lg border bg-card"
      data-testid={
        testId ??
        (isAdmission
          ? 'reprocess-provisional-progress-row'
          : isPdf
            ? 'pdf-progress-row'
            : 'text-ingestion-progress-row')
      }
      data-track-id={dataTrackId ?? trackId}
      data-provisional={isAdmission ? 'true' : undefined}
      data-admission={isAdmission ? admissionPhase : undefined}
    >
      {isAdmission ? (
        <AdmissionPhaseRow
          phase={admissionPhase}
          documentName={documentName}
          stageMessage={stageMessage}
          variant="row"
          data-testid={
            admissionPhase === 'cleaning'
              ? 'reprocess-cleaning-row'
              : 'reprocess-queuing-row'
          }
        />
      ) : run ? (
        <IngestionRunCard
          run={run}
          compact
          onCancel={handleCancel}
          nestedDetail={
            isPdf ? (
              <PdfUploadProgress
                trackId={trackId}
                filename={documentName}
                compact
                nested
                onComplete={onComplete}
                onFailed={onFailed}
              />
            ) : undefined
          }
        />
      ) : (
        <div className="text-sm text-muted-foreground py-1" data-testid="spec086-run-loading">
          {documentName}
          <span className="block text-xs">Queued for processing…</span>
        </div>
      )}
      {onRemove && (
        <Button
          variant="ghost"
          size="icon"
          className="absolute top-1 right-1 h-8 w-8"
          onClick={onRemove}
          aria-label={t(
            'documents.reprocess.dismissAria',
            'Dismiss progress — hides progress; processing continues',
          )}
          title={dismissHint}
        >
          <X className="h-4 w-4" />
          <span className="sr-only">{dismissHint}</span>
        </Button>
      )}
    </div>
  );
}
