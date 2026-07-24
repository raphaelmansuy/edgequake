'use client';

import { Button } from '@/components/ui/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { DocumentDownloadMenu } from './document-download-menu';
import { needsReuploadNotReprocess } from '@/lib/pipeline/pipeline-document-state';
import type { Document } from '@/types';
import { Copy, Eye, MoreVertical, RefreshCw, StopCircle, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { ResetDocumentStatusButton } from './reset-document-status-button';
import { DeleteConfirmDialog } from './delete-confirm-dialog';
import { useState } from 'react';

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
  /** Callback to delete document — called after user confirms via dialog */
  onDelete: (id: string) => void;
  /** Whether a cancel operation is in progress */
  isCancelling?: boolean;
  /** Whether a delete operation is in progress for this document */
  isDeleting?: boolean;
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
 * SPEC-050: Delete action now opens DeleteConfirmDialog with impact preview.
 * 
 * @implements FEAT0001 - Document ingestion with entity extraction
 * @implements SPEC-050 - Impact preview before delete
 */
export function DocumentActionsMenu({
  doc,
  onViewPdf,
  onCancel,
  onReprocess,
  onDelete,
  isCancelling = false,
  isDeleting = false,
}: DocumentActionsMenuProps) {
  const { t } = useTranslation();
  // SPEC-050: Local state controls the DeleteConfirmDialog.
  // WHY: The dialog is scoped to this menu row — no need to lift state.
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

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
    <>
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

          <DocumentDownloadMenu document={doc} variant="submenu" />

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

          {/* Reprocess — hide for orphan staging shells (dismiss + re-upload). */}
          {!needsReuploadNotReprocess(doc) && (
            <DropdownMenuItem onClick={() => onReprocess(doc.id)}>
              <RefreshCw className="h-4 w-4 mr-2" />
              {t('documents.actions.reprocess')}
            </DropdownMenuItem>
          )}

          {/* SPEC-050: Delete now opens a confirm dialog with impact preview */}
          <DropdownMenuItem
            onClick={() => setDeleteDialogOpen(true)}
            className="text-destructive"
            disabled={isDeleting}
          >
            <Trash2 className="h-4 w-4 mr-2" />
            {t('documents.actions.delete')}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      {/* SPEC-050: Impact preview + confirm before delete */}
      <DeleteConfirmDialog
        open={deleteDialogOpen}
        onOpenChange={setDeleteDialogOpen}
        document={doc}
        onConfirm={onDelete}
        isDeleting={isDeleting}
      />
    </>
  );
}
