'use client';

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
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
            {/* Statistics */}
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div className="p-2 bg-muted rounded">
                <p className="text-muted-foreground">Processing</p>
                <p className="text-xl font-bold">{data.running_tasks}</p>
              </div>
              <div className="p-2 bg-muted rounded">
                <p className="text-muted-foreground">Queued</p>
                <p className="text-xl font-bold">{data.queued_tasks}</p>
              </div>
              <div className="p-2 bg-muted rounded">
                <p className="text-muted-foreground">Completed</p>
                <p className="text-xl font-bold text-green-600">{data.completed_tasks}</p>
              </div>
              <div className="p-2 bg-muted rounded">
                <p className="text-muted-foreground">Failed</p>
                <p className="text-xl font-bold text-red-600">{data.failed_tasks}</p>
              </div>
            </div>

            {/* Recent Tasks */}
            {data.tasks && data.tasks.length > 0 && (
              <div className="space-y-2">
                <p className="text-sm font-medium">{t('pipeline.messages', 'Recent Tasks')}</p>
                <ScrollArea className="h-32 rounded-md border bg-muted p-2">
                  {data.tasks.slice(0, 10).map((task) => (
                    <div key={task.track_id} className="flex items-center justify-between py-1 text-xs font-mono">
                      <span className="truncate flex-1">{task.track_id.slice(0, 8)}...</span>
                      <span className={`ml-2 px-1 rounded ${
                        task.status === 'processing' ? 'bg-yellow-200 text-yellow-800' :
                        task.status === 'indexed' ? 'bg-green-200 text-green-800' :
                        task.status === 'failed' ? 'bg-red-200 text-red-800' :
                        'bg-gray-200 text-gray-800'
                      }`}>
                        {task.status}
                      </span>
                    </div>
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
          <div className="py-8 text-center">
            <p className="text-muted-foreground mb-2">{t('pipeline.idle')}</p>
            {data && (
              <p className="text-xs text-muted-foreground">
                Total: {data.completed_tasks} completed, {data.failed_tasks} failed
              </p>
            )}
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
