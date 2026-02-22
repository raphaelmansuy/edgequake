'use client';

import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { reprocessDocument, retryTask } from '@/lib/api/edgequake';
import type { Document } from '@/types';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
    ChevronDown,
    Loader2,
    RefreshCcw,
    RotateCcw,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface ResetDocumentStatusButtonProps {
  /**
   * The document to reset
   */
  document: Document;
  /**
   * Whether to show only as an icon button
   */
  iconOnly?: boolean;
  /**
   * Size of the button
   */
  size?: 'default' | 'sm' | 'lg' | 'icon';
  /**
   * Callback when reset is successful
   */
  onReset?: () => void;
}

/**
 * Button component to reset a document's status back to pending for reprocessing.
 * Can either retry the existing task or trigger a full reprocess.
 */
export function ResetDocumentStatusButton({
  document,
  iconOnly = false,
  size = 'sm',
  onReset,
}: ResetDocumentStatusButtonProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [showConfirm, setShowConfirm] = useState(false);

  // Retry task mutation (uses existing task if available)
  const retryMutation = useMutation({
    mutationFn: async () => {
      if (document.track_id) {
        await retryTask(document.track_id);
        return { success: true };
      }
      // Fall back to reprocess if no track_id (shouldn't happen)
      throw new Error('No track_id available for reprocessing');
    },
    onSuccess: () => {
      toast.success(
        t('documents.reset.success', 'Document queued for reprocessing'),
        {
          description: t(
            'documents.reset.successDesc',
            'The document will be processed again shortly.'
          ),
        }
      );
      queryClient.invalidateQueries({ queryKey: ['documents'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-status'] });
      onReset?.();
    },
    onError: (error) => {
      toast.error(
        t('documents.reset.failed', 'Failed to reset document'),
        {
          description: error instanceof Error ? error.message : 'Unknown error',
        }
      );
    },
  });

  // Full reprocess mutation
  // WHY: reprocessDocument() sends { document_id } to the backend, which expects the
  // document's `id` field (KV metadata key) — NOT the track_id.  Passing track_id
  // caused the backend to silently find zero matching documents to reprocess.
  const reprocessMutation = useMutation({
    mutationFn: () => {
      if (!document.id) {
        throw new Error('No document id available for reprocessing');
      }
      return reprocessDocument(document.id);
    },
    onSuccess: () => {
      toast.success(
        t('documents.reprocess.success', 'Document queued for reprocessing'),
        {
          description: t(
            'documents.reprocess.successDesc',
            'The document will be fully reprocessed.'
          ),
        }
      );
      queryClient.invalidateQueries({ queryKey: ['documents'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-status'] });
      onReset?.();
    },
    onError: (error) => {
      toast.error(
        t('documents.reprocess.failed', 'Failed to reprocess document'),
        {
          description: error instanceof Error ? error.message : 'Unknown error',
        }
      );
    },
  });

  const isLoading = retryMutation.isPending || reprocessMutation.isPending;

  // Only show for failed, completed, or cancelled documents
  // WHY: Cancelled documents should be retryable just like failed ones.
  const canReset = document.status === 'failed' || document.status === 'completed' || document.status === 'cancelled';

  if (!canReset) {
    return null;
  }

  // Icon-only mode
  if (iconOnly) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => setShowConfirm(true)}
              disabled={isLoading}
            >
              {isLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <RotateCcw className="h-4 w-4" />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            {(document.status === 'failed' || document.status === 'cancelled')
              ? t('documents.reset.retryFailed', 'Retry failed document')
              : t('documents.reset.reprocess', 'Reprocess document')}
          </TooltipContent>
        </Tooltip>

        <AlertDialog open={showConfirm} onOpenChange={setShowConfirm}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>
                {t('documents.reset.confirmTitle', 'Reset Document Status?')}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {(document.status === 'failed' || document.status === 'cancelled')
                  ? t(
                      'documents.reset.confirmDescFailed',
                      'This will retry processing the failed document. Any existing error will be cleared.'
                    )
                  : t(
                      'documents.reset.confirmDescCompleted',
                      'This will reprocess the document, potentially updating extracted entities and relationships.'
                    )}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>{t('common.cancel', 'Cancel')}</AlertDialogCancel>
              <AlertDialogAction
                onClick={() => {
                  (document.status === 'failed' || document.status === 'cancelled')
                    ? retryMutation.mutate()
                    : reprocessMutation.mutate();
                }}
              >
                {t('common.confirm', 'Confirm')}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </TooltipProvider>
    );
  }

  // Full button with dropdown for options
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size={size} disabled={isLoading}>
            {isLoading ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <RotateCcw className="mr-2 h-4 w-4" />
            )}
            {t('documents.reset.button', 'Reset Status')}
            <ChevronDown className="ml-2 h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {(document.status === 'failed' || document.status === 'cancelled') && document.track_id && (
            <DropdownMenuItem onClick={() => retryMutation.mutate()}>
              <RefreshCcw className="mr-2 h-4 w-4" />
              {t('documents.reset.retry', 'Retry Processing')}
            </DropdownMenuItem>
          )}
          <DropdownMenuItem onClick={() => setShowConfirm(true)}>
            <RotateCcw className="mr-2 h-4 w-4" />
            {t('documents.reset.fullReprocess', 'Full Reprocess')}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <AlertDialog open={showConfirm} onOpenChange={setShowConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t('documents.reset.reprocessTitle', 'Reprocess Document?')}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                'documents.reset.reprocessDesc',
                'This will fully reprocess the document from scratch. Existing entities and relationships from this document may be updated.'
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.cancel', 'Cancel')}</AlertDialogCancel>
            <AlertDialogAction onClick={() => reprocessMutation.mutate()}>
              {t('documents.reset.reprocess', 'Reprocess')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

export default ResetDocumentStatusButton;
