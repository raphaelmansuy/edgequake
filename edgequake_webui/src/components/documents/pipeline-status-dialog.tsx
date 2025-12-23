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
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { getEnhancedPipelineStatus, requestPipelineCancellation } from '@/lib/api/edgequake';
import type { PipelineMessage } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import { Activity, AlertTriangle, CheckCircle, Info, Loader2, XCircle } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface PipelineStatusDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const levelConfig = {
  info: { icon: Info, color: 'text-blue-500', bgColor: 'bg-blue-50 dark:bg-blue-950' },
  warn: { icon: AlertTriangle, color: 'text-yellow-500', bgColor: 'bg-yellow-50 dark:bg-yellow-950' },
  error: { icon: XCircle, color: 'text-red-500', bgColor: 'bg-red-50 dark:bg-red-950' },
} as const;

function MessageItem({ message }: { message: PipelineMessage }) {
  const config = levelConfig[message.level as keyof typeof levelConfig] || levelConfig.info;
  const Icon = config.icon;
  
  return (
    <div className={`flex items-start gap-2 py-1.5 px-2 rounded text-xs ${config.bgColor}`}>
      <Icon className={`h-3 w-3 mt-0.5 shrink-0 ${config.color}`} />
      <div className="flex-1 min-w-0">
        <p className="break-words">{message.message}</p>
        <p className="text-[10px] text-muted-foreground mt-0.5">
          {formatDistanceToNow(new Date(message.timestamp), { addSuffix: true })}
        </p>
      </div>
    </div>
  );
}

export function PipelineStatusDialog({
  open,
  onOpenChange,
}: PipelineStatusDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [showCancelConfirm, setShowCancelConfirm] = useState(false);

  // Use enhanced pipeline status with history messages (Phase 3)
  const { data, isLoading } = useQuery({
    queryKey: ['enhanced-pipeline-status'],
    queryFn: getEnhancedPipelineStatus,
    refetchInterval: open ? 2000 : false, // Poll every 2s when dialog is open
    enabled: open,
  });

  const cancelMutation = useMutation({
    mutationFn: requestPipelineCancellation,
    onSuccess: () => {
      toast.success(t('pipeline.cancelSuccess', 'Pipeline cancellation requested'));
      setShowCancelConfirm(false);
      queryClient.invalidateQueries({ queryKey: ['enhanced-pipeline-status'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-status'] });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(`Failed to cancel: ${error instanceof Error ? error.message : 'Unknown error'}`);
      setShowCancelConfirm(false);
    },
  });

  const handleCancelClick = () => {
    // Show confirmation dialog (Phase 4)
    setShowCancelConfirm(true);
  };

  const handleConfirmCancel = () => {
    cancelMutation.mutate();
  };

  // Calculate progress
  const progress = data?.total_documents && data.total_documents > 0
    ? (data.processed_documents / data.total_documents) * 100
    : 0;

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              {t('pipeline.title', 'Pipeline Status')}
              {data?.is_busy && (
                <Badge variant="outline" className="ml-2 text-orange-500 border-orange-500">
                  <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                  {t('pipeline.active', 'Active')}
                </Badge>
              )}
              {data?.cancellation_requested && (
                <Badge variant="destructive" className="ml-2">
                  {t('pipeline.cancelling', 'Cancelling...')}
                </Badge>
              )}
            </DialogTitle>
          </DialogHeader>

          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : data?.is_busy ? (
            <div className="space-y-4">
              {/* Job Info */}
              {data.job_name && (
                <div className="p-3 bg-muted/50 rounded-lg">
                  <p className="text-sm font-medium">{data.job_name}</p>
                  {data.job_start && (
                    <p className="text-xs text-muted-foreground">
                      Started {formatDistanceToNow(new Date(data.job_start), { addSuffix: true })}
                    </p>
                  )}
                </div>
              )}

              {/* Progress Bar */}
              {data.total_documents > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-muted-foreground">
                      {t('pipeline.progress', 'Progress: {{current}}/{{total}} documents', {
                        current: data.processed_documents,
                        total: data.total_documents,
                      })}
                    </span>
                    <span className="font-medium">{Math.round(progress)}%</span>
                  </div>
                  <Progress value={progress} className="h-2" />
                  {data.total_batches > 0 && (
                    <p className="text-xs text-muted-foreground text-center">
                      Batch {data.current_batch}/{data.total_batches}
                    </p>
                  )}
                </div>
              )}

              {/* Statistics Grid */}
              <div className="grid grid-cols-4 gap-2 text-sm">
                <div className="p-2 bg-yellow-50 dark:bg-yellow-950 rounded text-center">
                  <p className="text-xs text-muted-foreground">Pending</p>
                  <p className="text-lg font-bold text-yellow-600">{data.pending_tasks}</p>
                </div>
                <div className="p-2 bg-blue-50 dark:bg-blue-950 rounded text-center">
                  <p className="text-xs text-muted-foreground">Processing</p>
                  <p className="text-lg font-bold text-blue-600">{data.processing_tasks}</p>
                </div>
                <div className="p-2 bg-green-50 dark:bg-green-950 rounded text-center">
                  <p className="text-xs text-muted-foreground">Completed</p>
                  <p className="text-lg font-bold text-green-600">{data.completed_tasks}</p>
                </div>
                <div className="p-2 bg-red-50 dark:bg-red-950 rounded text-center">
                  <p className="text-xs text-muted-foreground">Failed</p>
                  <p className="text-lg font-bold text-red-600">{data.failed_tasks}</p>
                </div>
              </div>

              {/* History Messages (Phase 3) */}
              {data.history_messages && data.history_messages.length > 0 && (
                <div className="space-y-2">
                  <p className="text-sm font-medium flex items-center gap-2">
                    <Activity className="h-4 w-4" />
                    {t('pipeline.messages', 'Activity Log')}
                  </p>
                  <ScrollArea className="h-40 rounded-md border">
                    <div className="p-2 space-y-1">
                      {[...data.history_messages].reverse().map((msg, idx) => (
                        <MessageItem key={idx} message={msg} />
                      ))}
                    </div>
                  </ScrollArea>
                </div>
              )}

              {/* Latest Message */}
              {data.latest_message && !data.history_messages?.length && (
                <div className="p-3 bg-muted rounded-lg">
                  <p className="text-sm italic text-muted-foreground">{data.latest_message}</p>
                </div>
              )}

              {/* Cancel Button */}
              <Button
                variant="destructive"
                onClick={handleCancelClick}
                disabled={cancelMutation.isPending || data.cancellation_requested}
                className="w-full"
              >
                {cancelMutation.isPending ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <XCircle className="mr-2 h-4 w-4" />
                )}
                {data.cancellation_requested 
                  ? t('pipeline.cancelPending', 'Cancellation Pending...')
                  : t('pipeline.cancel', 'Cancel Pipeline')
                }
              </Button>
            </div>
          ) : (
            <div className="py-8 text-center space-y-4">
              <div className="flex justify-center">
                <CheckCircle className="h-12 w-12 text-green-500" />
              </div>
              <div>
                <p className="text-muted-foreground mb-2">{t('pipeline.idle', 'Pipeline is idle')}</p>
                {data && (
                  <p className="text-sm text-muted-foreground">
                    {t('pipeline.summary', '{{completed}} completed, {{failed}} failed', {
                      completed: data.completed_tasks,
                      failed: data.failed_tasks,
                    })}
                  </p>
                )}
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>

      {/* Cancel Confirmation Dialog (Phase 4) */}
      <AlertDialog open={showCancelConfirm} onOpenChange={setShowCancelConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('pipeline.cancelConfirmTitle', 'Cancel Pipeline?')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('pipeline.cancelConfirmDesc', 
                'This will stop processing after the current document. {{count}} document(s) have been processed so far.',
                { count: data?.processed_documents || 0 }
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t('common.keepProcessing', 'Keep Processing')}</AlertDialogCancel>
            <AlertDialogAction 
              onClick={handleConfirmCancel}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t('common.yesCancel', 'Yes, Cancel')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

/**
 * Pipeline Status Indicator for the header
 */
export function PipelineStatusIndicator() {
  const { t } = useTranslation();
  
  const { data } = useQuery({
    queryKey: ['enhanced-pipeline-status'],
    queryFn: getEnhancedPipelineStatus,
    refetchInterval: 5000, // Poll every 5s
  });

  if (!data?.is_busy) {
    return null;
  }

  return (
    <div className="flex items-center gap-1.5 text-sm text-orange-500 animate-pulse">
      <Loader2 className="h-3 w-3 animate-spin" />
      <span className="hidden sm:inline">{t('pipeline.busy', 'Processing...')}</span>
      {data.total_documents > 0 && (
        <span className="text-xs">
          ({data.processed_documents}/{data.total_documents})
        </span>
      )}
    </div>
  );
}
