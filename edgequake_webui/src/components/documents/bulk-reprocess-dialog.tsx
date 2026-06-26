/**
 * @fileoverview Bulk choice dialog for reprocessing multiple selected documents.
 *
 * WHY: The toolbar "Reprocess" button acts on every selected document at once.
 * Showing a per-document dialog does not scale, so we present a single choice
 * (full re-conversion vs. entity-only re-extraction) that applies one mode to
 * the whole batch. Mirrors `ReprocessDialog` styling for consistency.
 *
 * @implements FEAT-reprocess-choice - Bulk reprocess intent selection
 */
'use client';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { FileSearch, RefreshCw, Zap } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ReprocessMode } from '@/lib/api/edgequake';

export interface BulkReprocessChoice {
  mode: ReprocessMode;
}

interface BulkReprocessDialogProps {
  /** Whether the dialog is visible. */
  open: boolean;
  /** Number of selected documents the choice will apply to. */
  count: number;
  /** Called with the user's choice when they confirm. */
  onConfirm: (choice: BulkReprocessChoice) => void;
  /** Called when the user cancels or dismisses the dialog. */
  onCancel: () => void;
}

export function BulkReprocessDialog({
  open,
  count,
  onConfirm,
  onCancel,
}: BulkReprocessDialogProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<ReprocessMode>('entities');

  // Reset to the safe default whenever the dialog (re)opens.
  useEffect(() => {
    if (open) setMode('entities');
  }, [open]);

  const title = t('documents.reprocessDialog.bulkTitle', 'Reprocess {{count}} documents?', {
    count,
  });

  const handleConfirm = useCallback(() => {
    onConfirm({ mode });
  }, [mode, onConfirm]);

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && onCancel()}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <RefreshCw className="h-5 w-5 text-primary" />
            {title}
          </DialogTitle>
          <DialogDescription>
            {t(
              'documents.reprocessDialog.bulkDescription',
              'Choose how the selected documents should be reprocessed. The same mode applies to all of them.',
            )}
          </DialogDescription>
        </DialogHeader>

        <RadioGroup
          value={mode}
          onValueChange={(v) => setMode(v as ReprocessMode)}
          className="gap-3"
        >
          <BulkReprocessOption
            value="full"
            icon={<FileSearch className="h-5 w-5" />}
            label={t(
              'documents.reprocessDialog.fullLabel',
              'Re-convert from PDF (slower, uses vision tokens)',
            )}
            description={t(
              'documents.reprocessDialog.fullDescriptionBulk',
              'Re-runs PDF -> markdown conversion for every selected PDF, then re-extracts entities. Use this when the markdown itself is wrong or stale. Non-PDF docs are re-ingested from scratch.',
            )}
            badge={t('documents.reprocessDialog.fullBadge', 'Slowest · costs tokens')}
          />
          <BulkReprocessOption
            value="entities"
            icon={<Zap className="h-5 w-5" />}
            label={t(
              'documents.reprocessDialog.entitiesLabel',
              'Re-extract entities only (reuse existing markdown)',
            )}
            description={t(
              'documents.reprocessDialog.entitiesDescriptionBulk',
              'Keeps cached markdown and only re-runs the knowledge-graph pipeline for every selected document. Fast and cheap.',
            )}
            badge={t('documents.reprocessDialog.entitiesBadge', 'Fast · no vision')}
          />
        </RadioGroup>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={onCancel}>
            {t('documents.reprocessDialog.cancel', 'Cancel')}
          </Button>
          <Button onClick={handleConfirm}>
            {t('documents.reprocessDialog.confirm', 'Reprocess')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface BulkReprocessOptionProps {
  value: ReprocessMode;
  icon: React.ReactNode;
  label: string;
  description: string;
  badge: string;
}

function BulkReprocessOption({
  value,
  icon,
  label,
  description,
  badge,
}: BulkReprocessOptionProps) {
  return (
    <label
      htmlFor={`bulk-reprocess-${value}`}
      className="flex items-start gap-3 rounded-lg border p-3 transition-colors cursor-pointer select-none hover:bg-muted/40"
    >
      <RadioGroupItem
        id={`bulk-reprocess-${value}`}
        value={value}
        className="mt-1"
      />
      <div className="mt-0.5 shrink-0 text-muted-foreground">{icon}</div>
      <div className="flex-1 min-w-0 space-y-1">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium leading-tight">{label}</span>
          <span className="text-[10px] uppercase tracking-wide rounded-full border px-1.5 py-0.5 text-muted-foreground">
            {badge}
          </span>
        </div>
        <p className="text-xs text-muted-foreground leading-relaxed">{description}</p>
      </div>
    </label>
  );
}

export default BulkReprocessDialog;
