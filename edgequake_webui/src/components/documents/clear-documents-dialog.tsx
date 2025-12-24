'use client';

import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
    AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { deleteAllDocuments } from '@/lib/api/edgequake';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Loader2, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface ClearDocumentsDialogProps {
  /**
   * Total number of documents that will be deleted.
   * Used for display purposes.
   */
  documentCount: number;
  /**
   * Callback when documents are cleared successfully.
   * @param deletedCount Number of documents that were deleted
   */
  onCleared?: (deletedCount: number) => void;
  /**
   * Whether to show the button (false to use only as a controlled dialog)
   */
  showTrigger?: boolean;
  /**
   * Controlled open state
   */
  open?: boolean;
  /**
   * Callback when open state changes
   */
  onOpenChange?: (open: boolean) => void;
}

const CONFIRMATION_TEXT = 'DELETE ALL';

/**
 * Dialog component for clearing all documents from the system.
 * Requires typing "DELETE ALL" to confirm the destructive action.
 * Connects to DELETE /api/v1/documents endpoint.
 */
export function ClearDocumentsDialog({
  documentCount,
  onCleared,
  showTrigger = true,
  open: controlledOpen,
  onOpenChange: controlledOnOpenChange,
}: ClearDocumentsDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [internalOpen, setInternalOpen] = useState(false);
  const [confirmation, setConfirmation] = useState('');

  // Use controlled or internal state
  const isOpen = controlledOpen !== undefined ? controlledOpen : internalOpen;
  const setOpen = controlledOnOpenChange || setInternalOpen;

  const isConfirmed = confirmation === CONFIRMATION_TEXT;

  const clearMutation = useMutation({
    mutationFn: deleteAllDocuments,
    onSuccess: (data) => {
      toast.success(
        t('documents.clearAll.success', 'Documents cleared'),
        {
          description: t('documents.clearAll.successDesc', 'Successfully deleted {{count}} document(s) and their associated data.', { count: data.deleted_count }),
          duration: 5000,
        }
      );
      // Refresh documents list
      queryClient.invalidateQueries({ queryKey: ['documents'] });
      queryClient.invalidateQueries({ queryKey: ['graph'] });
      queryClient.invalidateQueries({ queryKey: ['entities'] });
      // Reset and close
      setConfirmation('');
      setOpen(false);
      // Notify parent
      onCleared?.(data.deleted_count);
    },
    onError: (error) => {
      toast.error(
        t('documents.clearAll.failed', 'Clear failed'),
        {
          description: error instanceof Error ? error.message : t('common.unknownError', 'An error occurred'),
          action: {
            label: t('common.retry', 'Retry'),
            onClick: () => clearMutation.mutate(),
          },
        }
      );
    },
  });

  const handleClear = () => {
    if (!isConfirmed) return;
    clearMutation.mutate();
  };

  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen) {
      // Reset confirmation when closing
      setConfirmation('');
    }
    setOpen(newOpen);
  };

  // Don't show if no documents
  if (documentCount === 0) {
    return null;
  }

  const triggerButton = (
    <Button variant="destructive" size="sm">
      <Trash2 className="h-4 w-4 mr-2" />
      {t('documents.clearAll.button', 'Clear All')}
    </Button>
  );

  return (
    <AlertDialog open={isOpen} onOpenChange={handleOpenChange}>
      {showTrigger && (
        <AlertDialogTrigger asChild>
          {triggerButton}
        </AlertDialogTrigger>
      )}
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="h-5 w-5" />
            {t('documents.clearAll.title', 'Delete All Documents')}
          </AlertDialogTitle>
          <AlertDialogDescription className="space-y-3">
            <p>
              {t('documents.clearAll.warning', 'This action cannot be undone. This will permanently delete:')}
            </p>
            <ul className="list-disc list-inside space-y-1 text-sm">
              <li>{t('documents.clearAll.item1', '{{count}} document(s)', { count: documentCount })}</li>
              <li>{t('documents.clearAll.item2', 'All extracted entities and relationships')}</li>
              <li>{t('documents.clearAll.item3', 'All document chunks and embeddings')}</li>
            </ul>
            <div className="pt-2">
              <Label htmlFor="confirmation" className="text-sm font-medium">
                {t('documents.clearAll.typeToConfirm', 'Type {{text}} to confirm:', { text: CONFIRMATION_TEXT })}
              </Label>
              <Input
                id="confirmation"
                value={confirmation}
                onChange={(e) => setConfirmation(e.target.value)}
                placeholder={CONFIRMATION_TEXT}
                className="mt-2"
                disabled={clearMutation.isPending}
                autoComplete="off"
              />
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={clearMutation.isPending}>
            {t('common.cancel', 'Cancel')}
          </AlertDialogCancel>
          <AlertDialogAction
            onClick={handleClear}
            disabled={!isConfirmed || clearMutation.isPending}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {clearMutation.isPending ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                {t('documents.clearAll.deleting', 'Deleting...')}
              </>
            ) : (
              t('documents.clearAll.confirmButton', 'Delete All Documents')
            )}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
