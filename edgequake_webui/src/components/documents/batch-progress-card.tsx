'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { getTrackStatus } from '@/lib/api/edgequake';
import type { TrackStatusResponse } from '@/types';
import { useQuery } from '@tanstack/react-query';
import {
    CheckCircle,
    Clock,
    FileText,
    Loader2,
    X,
    XCircle,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { StatusBadge, getDocumentDisplayStatus } from './status-badge';

interface BatchProgressCardProps {
  trackId: string;
  onClose: () => void;
  onComplete?: () => void;
}

export function BatchProgressCard({ trackId, onClose, onComplete }: BatchProgressCardProps) {
  const { t } = useTranslation();

  const { data: trackStatus, isLoading, isError, error } = useQuery({
    queryKey: ['track-status', trackId],
    queryFn: () => getTrackStatus(trackId),
    refetchInterval: (query) => {
      // Stop polling on error
      if (query.state.error) {
        return false;
      }
      // Stop polling when complete
      const data = query.state.data as TrackStatusResponse | undefined;
      if (data?.is_complete) {
        // Call onComplete callback when done
        if (onComplete) {
          setTimeout(onComplete, 1000);
        }
        return false;
      }
      return 2000; // Poll every 2 seconds
    },
    enabled: !!trackId,
    retry: 2, // Only retry twice on failure
  });

  // Handle error state
  if (isError) {
    return (
      <Card className="border-destructive/50 shadow-lg">
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-medium flex items-center gap-2 text-destructive">
              <XCircle className="h-4 w-4" />
              {t('documents.batch.error', 'Tracking Error')}
            </CardTitle>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={onClose}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-2">
          <p className="text-sm text-muted-foreground">
            {t('documents.batch.errorMessage', 'Unable to track batch progress. Documents may still be processing.')}
          </p>
          <p className="text-xs text-muted-foreground">
            {error instanceof Error ? error.message : 'Unknown error'}
          </p>
        </CardContent>
      </Card>
    );
  }

  if (isLoading || !trackStatus) {
    return (
      <Card className="border-primary/50 shadow-lg">
        <CardContent className="p-4 flex flex-col items-center justify-center gap-2">
          <Loader2 className="h-6 w-6 animate-spin text-primary" />
          <p className="text-sm text-muted-foreground">
            {t('documents.batch.loading', 'Loading batch status...')}
          </p>
        </CardContent>
      </Card>
    );
  }

  const { status_summary, total_count, is_complete, latest_message, documents } = trackStatus;
  const completedCount = status_summary.completed + status_summary.failed;
  const progress = total_count > 0 ? (completedCount / total_count) * 100 : 0;

  return (
    <Card className="border-primary/50 shadow-lg">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <FileText className="h-4 w-4" />
            {t('documents.batch.title', 'Batch Upload Progress')}
          </CardTitle>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={onClose}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Progress Bar */}
        <div className="space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">
              {t('documents.batch.progress', '{{completed}}/{{total}} documents', {
                completed: completedCount,
                total: total_count,
              })}
            </span>
            <span className="font-medium">{Math.round(progress)}%</span>
          </div>
          <Progress value={progress} className="h-2" />
        </div>

        {/* Status Summary */}
        <div className="flex flex-wrap gap-2">
          {status_summary.pending > 0 && (
            <Badge variant="outline" className="gap-1">
              <Clock className="h-3 w-3 text-yellow-500" />
              {status_summary.pending} {t('documents.status.pending', 'pending')}
            </Badge>
          )}
          {status_summary.processing > 0 && (
            <Badge variant="outline" className="gap-1">
              <Loader2 className="h-3 w-3 text-blue-500 animate-spin" />
              {status_summary.processing} {t('documents.status.processing', 'processing')}
            </Badge>
          )}
          {status_summary.completed > 0 && (
            <Badge variant="outline" className="gap-1 text-green-600">
              <CheckCircle className="h-3 w-3" />
              {status_summary.completed} {t('documents.status.completed', 'completed')}
            </Badge>
          )}
          {status_summary.failed > 0 && (
            <Badge variant="outline" className="gap-1 text-red-600">
              <XCircle className="h-3 w-3" />
              {status_summary.failed} {t('documents.status.failed', 'failed')}
            </Badge>
          )}
        </div>

        {/* Latest Message */}
        {latest_message && (
          <p className="text-sm text-muted-foreground italic">
            {latest_message}
          </p>
        )}

        {/* Pipeline Stages Legend - helps users understand progression */}
        {status_summary.processing > 0 && (
          <div className="flex items-center justify-center gap-1 text-[10px] text-muted-foreground">
            <span>Chunking</span>
            <span>→</span>
            <span>Extracting</span>
            <span>→</span>
            <span>Embedding</span>
            <span>→</span>
            <span>Done</span>
          </div>
        )}

        {/* Document List (scrollable) - Now with granular status badges */}
        {documents && documents.length > 0 && (
          <ScrollArea className="h-32 rounded-md border">
            <div className="p-2 space-y-1">
              {documents.map((doc) => {
                // SPEC-002: Use unified current_stage if available
                const displayStatus = getDocumentDisplayStatus(doc);
                
                return (
                  <div
                    key={doc.id}
                    className="flex items-center justify-between py-1 px-2 rounded hover:bg-muted/50 text-xs"
                  >
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <FileText className="h-3 w-3 text-muted-foreground shrink-0" />
                      <span className="truncate">{doc.title || doc.file_name || doc.id.slice(0, 8)}</span>
                    </div>
                    <StatusBadge status={displayStatus} compact />
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        )}

        {/* Completion Status */}
        {is_complete && (
          <div className={`text-sm p-2 rounded ${
            status_summary.failed > 0 
              ? 'bg-amber-50 text-amber-700 dark:bg-amber-950 dark:text-amber-300' 
              : 'bg-green-50 text-green-700 dark:bg-green-950 dark:text-green-300'
          }`}>
            {status_summary.failed > 0 
              ? t('documents.batch.completedWithErrors', 'Completed with {{count}} error(s)', { count: status_summary.failed })
              : t('documents.batch.allComplete', 'All documents processed successfully!')
            }
          </div>
        )}

        {/* Track ID info (for debugging) */}
        <div className="text-[10px] text-muted-foreground/50 truncate">
          Track: {trackId}
        </div>
      </CardContent>
    </Card>
  );
}
