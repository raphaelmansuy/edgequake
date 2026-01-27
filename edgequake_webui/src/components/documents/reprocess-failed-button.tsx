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
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { reprocessFailedDocuments } from '@/lib/api/edgequake';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Loader2, RefreshCw } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface ReprocessFailedButtonProps {
  /**
   * Number of failed documents.
   * Button is hidden if this is 0.
   */
  failedCount: number;
  /**
   * Callback when reprocessing is started successfully.
   * @param trackId The track ID for monitoring reprocessing progress
   */
  onReprocessStarted?: (trackId: string) => void;
  /**
   * Whether to show a compact version (icon only with badge)
   */
  compact?: boolean;
  /**
   * Whether to show confirmation dialog before starting
   */
  showConfirmation?: boolean;
}

/**
 * Button component to trigger reprocessing of all failed documents.
 * Connects to POST /api/v1/documents/reprocess endpoint.
 * Only visible when there are failed documents.
 */
export function ReprocessFailedButton({
  failedCount,
  onReprocessStarted,
  compact = false,
  showConfirmation = true,
}: ReprocessFailedButtonProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [dialogOpen, setDialogOpen] = useState(false);

  const reprocessMutation = useMutation({
    mutationFn: reprocessFailedDocuments,
    onSuccess: (data) => {
      // OODA-37: Fixed to use correct backend response fields
      const requeuedCount = data.requeued ?? data.failed_found ?? failedCount;
      const message = requeuedCount > 0 
        ? t('documents.reprocessAll.startedDesc', 'Retrying {{count}} failed document(s)...', { count: requeuedCount })
        : t('documents.reprocessAll.noDocuments', 'No failed documents found to reprocess');
      
      toast.success(
        t('documents.reprocessAll.started', 'Reprocessing started'),
        {
          description: message,
          duration: 5000,
          action: {
            label: t('documents.viewStatus', 'View Status'),
            onClick: () => {
              // Parent component can handle showing the pipeline status dialog
            },
          },
        }
      );
      // Refresh documents list
      queryClient.invalidateQueries({ queryKey: ['documents'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-status'] });
      // Notify parent
      onReprocessStarted?.(data.track_id);
      setDialogOpen(false);
    },
    onError: (error) => {
      toast.error(
        t('documents.reprocessAll.failed', 'Reprocess failed'),
        {
          description: error instanceof Error ? error.message : t('common.unknownError', 'An error occurred'),
          action: {
            label: t('common.retry', 'Retry'),
            onClick: () => reprocessMutation.mutate(),
          },
        }
      );
    },
  });

  // Don't render if no failed documents
  if (failedCount === 0) {
    return null;
  }

  const handleReprocess = () => {
    if (showConfirmation) {
      setDialogOpen(true);
    } else {
      reprocessMutation.mutate();
    }
  };

  const buttonContent = (
    <>
      {reprocessMutation.isPending ? (
        <Loader2 className={`h-4 w-4 ${compact ? '' : 'mr-2'} animate-spin`} />
      ) : (
        <RefreshCw className={`h-4 w-4 ${compact ? '' : 'mr-2'}`} />
      )}
      {!compact && (
        reprocessMutation.isPending
          ? t('documents.reprocessAll.processing', 'Retrying...')
          : t('documents.reprocessAll.retryFailed', 'Retry Failed ({{count}})', { count: failedCount })
      )}
    </>
  );

  const button = (
    <Button
      variant="destructive"
      size="sm"
      onClick={showConfirmation ? undefined : handleReprocess}
      disabled={reprocessMutation.isPending}
      data-testid="reprocess-failed-button"
    >
      {buttonContent}
    </Button>
  );

  const wrappedButton = compact ? (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          {button}
        </TooltipTrigger>
        <TooltipContent>
          <p>{t('documents.reprocessAll.retryFailed', 'Retry Failed ({{count}})', { count: failedCount })}</p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  ) : button;

  if (!showConfirmation) {
    return wrappedButton;
  }

  return (
    <AlertDialog open={dialogOpen} onOpenChange={setDialogOpen}>
      <AlertDialogTrigger asChild>
        {wrappedButton}
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t('documents.reprocessAll.confirmTitle', 'Reprocess Failed Documents?')}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t('documents.reprocessAll.confirmDesc', 'This will retry processing for {{count}} failed document(s). This may take some time depending on document size.', { count: failedCount })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={reprocessMutation.isPending} data-testid="reprocess-failed-cancel">
            {t('common.cancel', 'Cancel')}
          </AlertDialogCancel>
          <AlertDialogAction
            onClick={() => reprocessMutation.mutate()}
            disabled={reprocessMutation.isPending}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            data-testid="reprocess-failed-confirm"
          >
            {reprocessMutation.isPending ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                {t('documents.reprocessAll.processing', 'Retrying...')}
              </>
            ) : (
              t('documents.reprocessAll.confirm', 'Retry All Failed')
            )}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
