'use client';

import { Button } from '@/components/ui/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import type { Document } from '@/types';
import { Copy, Eye, MoreVertical, RefreshCw, StopCircle, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { ResetDocumentStatusButton } from './reset-document-status-button';

/**
 * Props for the DocumentActionsMenu component.
 */
interface DocumentActionsMenuProps {
  /** The document this menu acts on */
  doc: Document;
  /** Callback to view PDF document */
  onViewPdf: (doc: Document) => void;
  /** Callback to cancel document processing */
  onCancel: (trackId: string) => void;
  /** Callback to reprocess document */
  onReprocess: (id: string) => void;
  /** Callback to delete document */
  onDelete: (id: string) => void;
  /** Whether a cancel operation is in progress */
  isCancelling?: boolean;
}

/** Processing status values that allow cancellation */
const CANCELLABLE_STATUSES = ['pending', 'processing'];
/** Processing stages that allow cancellation */
const CANCELLABLE_STAGES = [
  'converting', 'uploading', 'preprocessing', 'chunking',
  'extracting', 'gleaning', 'merging', 'summarizing', 'embedding', 'storing'
];

/**
 * Dropdown menu with document actions.
 * 
 * WHY: Extracted from DocumentManager for SRP compliance (OODA-09).
 * This component handles the actions dropdown for each document row.
 * 
 * @implements FEAT0001 - Document ingestion with entity extraction
 */
export function DocumentActionsMenu({
  doc,
  onViewPdf,
  onCancel,
  onReprocess,
  onDelete,
  isCancelling = false,
}: DocumentActionsMenuProps) {
  const { t } = useTranslation();

  const handleCopyId = () => {
    navigator.clipboard.writeText(doc.id);
    toast.success(t('documents.actions.idCopied', 'Document ID copied'));
  };

  const canCancel = 
    ((CANCELLABLE_STATUSES.includes(doc.status || '')) || 
    (CANCELLABLE_STAGES.includes(doc.current_stage || ''))) &&
    doc.track_id;

  const showViewPdf = doc.source_type === 'pdf' || doc.pdf_id;
  // WHY: Cancelled documents should also show the reset/reprocess option
  const showReset = doc.status === 'failed' || doc.status === 'partial_failure' || doc.status === 'cancelled';

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="h-8 w-8" aria-label="More actions">
          <MoreVertical className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {/* OODA-31: Copy document ID */}
        <DropdownMenuItem onClick={handleCopyId}>
          <Copy className="h-4 w-4 mr-2" />
          {t('documents.actions.copyId', 'Copy ID')}
        </DropdownMenuItem>

        {/* SPEC-002: View PDF/Markdown for PDF documents */}
        {showViewPdf && (
          <DropdownMenuItem onClick={() => onViewPdf(doc)}>
            <Eye className="h-4 w-4 mr-2" />
            {t('documents.actions.viewPdf', 'View PDF')}
          </DropdownMenuItem>
        )}

        {/* Reset status option for failed documents */}
        {showReset && (
          <DropdownMenuItem asChild>
            <div className="p-0">
              <ResetDocumentStatusButton document={doc} iconOnly={false} size="sm" />
            </div>
          </DropdownMenuItem>
        )}

        {/* Cancel option for processing documents */}
        {canCancel && (
          <DropdownMenuItem 
            onClick={() => onCancel(doc.track_id!)}
            className="text-orange-600"
            disabled={isCancelling}
          >
            <StopCircle className="h-4 w-4 mr-2" />
            {t('documents.actions.cancel', 'Cancel Extraction')}
          </DropdownMenuItem>
        )}

        {/* Reprocess */}
        <DropdownMenuItem onClick={() => onReprocess(doc.id)}>
          <RefreshCw className="h-4 w-4 mr-2" />
          {t('documents.actions.reprocess')}
        </DropdownMenuItem>

        {/* Delete */}
        <DropdownMenuItem
          onClick={() => onDelete(doc.id)}
          className="text-destructive"
        >
          <Trash2 className="h-4 w-4 mr-2" />
          {t('documents.actions.delete')}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
