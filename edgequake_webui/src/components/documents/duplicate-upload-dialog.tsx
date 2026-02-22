/**
 * @fileoverview Dialog for resolving duplicate document uploads.
 *
 * WHY: When the backend detects a duplicate (same SHA-256 per workspace),
 * we need user confirmation before deciding to replace the existing
 * document (delete old + upload new) or skip the upload. This is especially
 * important for batch uploads where multiple files may be duplicates.
 *
 * @implements FEAT-dup-detection - Duplicate upload resolution dialog
 * @implements BR-dup-replace    - Replace = force_reindex re-upload for PDFs
 * @implements BR-dup-skip       - Skip = silently discard duplicate upload
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { FileText, RefreshCw, SkipForward } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/**
 * Information about a duplicate file detected during upload.
 */
export interface PendingDuplicate {
  /** User-visible file name */
  fileName: string;
  /** Existing document ID that matches (short form shown to user) */
  existingDocId: string;
  /** The original File object so it can be re-uploaded after replacing */
  file: File;
}

/** User decision for a single duplicate. */
export type DuplicateDecision = 'replace' | 'skip';

/** Map from existingDocId → decision for each pending duplicate. */
export type DuplicateResolutions = Record<string, DuplicateDecision>;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface DuplicateUploadDialogProps {
  /** List of detected duplicate files waiting for user decision. */
  duplicates: PendingDuplicate[];
  /** Called when the user confirms their decisions (may be empty if dialog closed). */
  onResolve: (resolutions: DuplicateResolutions) => void;
  /** Whether the dialog is visible. */
  open: boolean;
}

export function DuplicateUploadDialog({
  duplicates,
  onResolve,
  open,
}: DuplicateUploadDialogProps) {
  const { t } = useTranslation();

  // Per-file decision state: 'replace' is the default (user expects reprocess)
  const [decisions, setDecisions] = useState<Record<string, DuplicateDecision>>(
    () => Object.fromEntries(duplicates.map((d) => [d.existingDocId, 'replace'])),
  );

  // Reset decisions when a new batch of duplicates arrives
  // WHY: useState initializer only runs on mount; if component stays mounted
  //      while duplicates change, decisions must be re-initialized.
  useEffect(() => {
    setDecisions(
      Object.fromEntries(duplicates.map((d) => [d.existingDocId, 'replace'])),
    );
  }, [duplicates]);

  // Derived list with per-file decision for rendering
  const decisionEntries = useMemo(() => {
    return duplicates.map((d) => ({
      ...d,
      decision: decisions[d.existingDocId] ?? 'replace',
    }));
  }, [duplicates, decisions]);

  const replaceCount = useMemo(
    () => decisionEntries.filter((e) => e.decision === 'replace').length,
    [decisionEntries],
  );
  const skipCount = decisionEntries.length - replaceCount;

  // ---------------------------------------------------------------------------
  // Handlers
  // ---------------------------------------------------------------------------

  const setDecision = useCallback((docId: string, decision: DuplicateDecision) => {
    setDecisions((prev) => ({ ...prev, [docId]: decision }));
  }, []);

  const setAll = useCallback((decision: DuplicateDecision) => {
    setDecisions(
      Object.fromEntries(duplicates.map((d) => [d.existingDocId, decision])),
    );
  }, [duplicates]);

  const handleConfirm = useCallback(() => {
    onResolve(decisions);
  }, [onResolve, decisions]);

  const handleSkipAll = useCallback(() => {
    onResolve(Object.fromEntries(duplicates.map((d) => [d.existingDocId, 'skip'])));
  }, [onResolve, duplicates]);

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  const title =
    duplicates.length === 1
      ? t('documents.duplicateDialog.titleSingle', 'Duplicate document detected')
      : t('documents.duplicateDialog.titleMultiple', '{{count}} duplicate documents detected', {
          count: duplicates.length,
        });

  const description =
    duplicates.length === 1
      ? t(
          'documents.duplicateDialog.descriptionSingle',
          'This file already exists in the workspace. Would you like to replace it (reprocess the existing document) or skip this upload?',
        )
      : t(
          'documents.duplicateDialog.descriptionMultiple',
          'The following files already exist in the workspace. Choose what to do with each one.',
        );

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && handleSkipAll()}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5 text-amber-500" />
            {title}
          </DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        {/* Batch action buttons */}
        {duplicates.length > 1 && (
          <div className="flex gap-2 pb-1">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setAll('replace')}
              className="text-xs"
            >
              <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
              {t('documents.duplicateDialog.replaceAll', 'Replace all')}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setAll('skip')}
              className="text-xs"
            >
              <SkipForward className="h-3.5 w-3.5 mr-1.5" />
              {t('documents.duplicateDialog.skipAll', 'Skip all')}
            </Button>
          </div>
        )}

        {/* File list */}
        <ScrollArea className="max-h-64">
          <div className="space-y-3 pr-4">
            {decisionEntries.map((entry) => (
              <DuplicateRow
                key={entry.existingDocId}
                entry={entry}
                onDecision={setDecision}
              />
            ))}
          </div>
        </ScrollArea>

        {/* Summary */}
        {duplicates.length > 1 && (
          <p className="text-xs text-muted-foreground">
            {t(
              'documents.duplicateDialog.summary',
              '{{replace}} will be replaced · {{skip}} will be skipped',
              { replace: replaceCount, skip: skipCount },
            )}
          </p>
        )}

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={handleSkipAll}>
            {t('documents.duplicateDialog.cancelAll', 'Skip all & close')}
          </Button>
          <Button onClick={handleConfirm}>
            {t('documents.duplicateDialog.confirm', 'Confirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Row subcomponent
// ---------------------------------------------------------------------------

interface RowEntry {
  fileName: string;
  existingDocId: string;
  decision: DuplicateDecision;
}

interface DuplicateRowProps {
  entry: RowEntry;
  onDecision: (docId: string, decision: DuplicateDecision) => void;
}

function DuplicateRow({ entry, onDecision }: DuplicateRowProps) {
  const { t } = useTranslation();
  const isReplace = entry.decision === 'replace';

  return (
    <div className="flex items-start gap-3 p-3 rounded-lg border bg-muted/30">
      <FileText className="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground" />
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate">{entry.fileName}</p>
        <p className="text-xs text-muted-foreground">
          {t('documents.duplicateDialog.existingId', 'Existing: {{id}}', {
            id: entry.existingDocId.slice(0, 8),
          })}
        </p>
      </div>
      {/* Toggle: Replace / Skip */}
      <div className="flex items-center gap-2 shrink-0">
        <Badge
          variant={isReplace ? 'default' : 'outline'}
          className="cursor-pointer text-xs select-none"
          onClick={() => onDecision(entry.existingDocId, 'replace')}
        >
          <RefreshCw className="h-3 w-3 mr-1" />
          {t('documents.duplicateDialog.replace', 'Replace')}
        </Badge>
        <Badge
          variant={!isReplace ? 'default' : 'outline'}
          className="cursor-pointer text-xs select-none"
          onClick={() => onDecision(entry.existingDocId, 'skip')}
        >
          <SkipForward className="h-3 w-3 mr-1" />
          {t('documents.duplicateDialog.skip', 'Skip')}
        </Badge>
      </div>
    </div>
  );
}
