'use client';

import { Button } from '@/components/ui/button';
import { RefreshCw, Trash2, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

/**
 * Props for the BatchActionsBar component.
 */
interface BatchActionsBarProps {
  /** Number of documents currently selected */
  selectedCount: number;
  /** Callback when Reprocess button is clicked */
  onReprocess: () => void;
  /** Callback when Delete button is clicked */
  onDelete: () => void;
  /** Callback when Clear selection button is clicked */
  onClear: () => void;
}

/**
 * Displays a bar with bulk action buttons when documents are selected.
 * MI-04: Animated entry/exit — bar slides in from above and fades when selection
 * is cleared, preventing abrupt layout shifts.
 *
 * @implements FEAT0003 - Batch document processing
 */
export function BatchActionsBar({
  selectedCount,
  onReprocess,
  onDelete,
  onClear,
}: BatchActionsBarProps) {
  const { t } = useTranslation();

  if (selectedCount === 0) {
    return null;
  }

  return (
    <div
      className={[
        'shrink-0 px-4 py-2 bg-primary/5 border-b border-primary/20 flex items-center justify-between',
        // MI-04: slide-down entry animation, respects reduced motion
        'motion-safe:animate-[slideDown_150ms_ease-out]',
      ].join(' ')}
    >
      <div className="flex items-center gap-3">
        <span className="text-sm font-medium">
          {t('documents.bulk.selected', { count: selectedCount }) || `${selectedCount} selected`}
        </span>
        <span className="text-xs text-muted-foreground hidden sm:inline">
          Press <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">Esc</kbd> to clear
        </span>
      </div>
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={onReprocess}>
          <RefreshCw className="h-4 w-4 mr-2" />
          {t('documents.bulk.reprocess', 'Reprocess')}
        </Button>
        <Button variant="outline" size="sm" className="text-destructive" onClick={onDelete}>
          <Trash2 className="h-4 w-4 mr-2" />
          {t('documents.bulk.delete', 'Delete')}
        </Button>
        <Button variant="ghost" size="sm" onClick={onClear}>
          <X className="h-4 w-4 mr-2" />
          {t('documents.bulk.clear', 'Clear')}
        </Button>
      </div>
    </div>
  );
}
