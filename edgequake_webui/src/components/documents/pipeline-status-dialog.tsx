'use client';

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cancelPipeline, getPipelineStatus } from '@/lib/api/edgequake';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Activity, Loader2, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface PipelineStatusDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function PipelineStatusDialog({
  open,
  onOpenChange,
}: PipelineStatusDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ['pipeline-status'],
    queryFn: getPipelineStatus,
    refetchInterval: open ? 2000 : false, // Poll every 2s when dialog is open
    enabled: open,
  });

  const cancelMutation = useMutation({
    mutationFn: cancelPipeline,
    onSuccess: () => {
      toast.success('Pipeline cancelled');
      queryClient.invalidateQueries({ queryKey: ['pipeline-status'] });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(`Failed to cancel: ${error instanceof Error ? error.message : 'Unknown error'}`);
    },
  });

  const handleCancel = () => {
    cancelMutation.mutate();
  };

  const formatTime = (timestamp: string) => {
    try {
      return new Date(timestamp).toLocaleTimeString();
    } catch {
      return timestamp;
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            {t('pipeline.title')}
          </DialogTitle>
        </DialogHeader>

        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : data?.is_busy ? (
          <div className="space-y-4">
            {/* Job Info */}
            <div>
              <p className="text-sm font-medium">
                {t('pipeline.job', { name: data.job_name || 'Processing' })}
              </p>
              {data.start_time && (
                <p className="text-sm text-muted-foreground">
                  {t('pipeline.started', { time: formatTime(data.start_time) })}
                </p>
              )}
            </div>

            {/* Progress Bar */}
            {data.progress !== undefined && (
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span>{t('pipeline.progress')}</span>
                  <span>{Math.round(data.progress)}%</span>
                </div>
                <Progress value={data.progress} />
                {data.current !== undefined && data.total !== undefined && (
                  <p className="text-xs text-muted-foreground text-center">
                    {t('pipeline.processed', { current: data.current, total: data.total })}
                  </p>
                )}
              </div>
            )}

            {/* Messages Log */}
            {data.messages && data.messages.length > 0 && (
              <div className="space-y-2">
                <p className="text-sm font-medium">{t('pipeline.messages')}</p>
                <ScrollArea className="h-32 rounded-md border bg-muted p-2">
                  {data.messages.map((msg: string, i: number) => (
                    <p key={i} className="text-xs font-mono">
                      {msg}
                    </p>
                  ))}
                </ScrollArea>
              </div>
            )}

            {/* Cancel Button */}
            <Button
              variant="destructive"
              onClick={handleCancel}
              disabled={cancelMutation.isPending}
              className="w-full"
            >
              {cancelMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <XCircle className="mr-2 h-4 w-4" />
              )}
              {t('pipeline.cancel')}
            </Button>
          </div>
        ) : (
          <div className="py-8 text-center text-muted-foreground">
            {t('pipeline.idle')}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

/**
 * Pipeline Status Indicator for the header
 */
export function PipelineStatusIndicator() {
  const { t } = useTranslation();
  
  const { data } = useQuery({
    queryKey: ['pipeline-status'],
    queryFn: getPipelineStatus,
    refetchInterval: 5000, // Poll every 5s
  });

  if (!data?.is_busy) {
    return null;
  }

  return (
    <div className="flex items-center gap-1.5 text-sm text-orange-500 animate-pulse">
      <Loader2 className="h-3 w-3 animate-spin" />
      <span className="hidden sm:inline">{t('pipeline.busy')}</span>
    </div>
  );
}
