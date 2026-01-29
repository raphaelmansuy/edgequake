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
import { useChunkProgress } from '@/hooks';
import { getEnhancedPipelineStatus, requestPipelineCancellation } from '@/lib/api/edgequake';
import type { PipelineMessage } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import { Activity, AlertTriangle, Check, CheckCircle, Clock, Database, DollarSign, Eraser, FileText, Info, Layers, Loader2, Sparkles, Timer, XCircle, Zap } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

/**
 * Clear phase statistics for rebuild operations.
 *
 * @implements OODA-26: Clear phase statistics display
 */
export interface ClearStats {
  /** Number of entity nodes cleared (KG rebuild) */
  nodesCleared?: number;
  /** Number of relationship edges cleared (KG rebuild) */
  edgesCleared?: number;
  /** Number of vector embeddings cleared */
  vectorsCleared?: number;
}

interface PipelineStatusDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Optional title override for the dialog */
  title?: string;
  /** Optional subtitle for context */
  subtitle?: string;
  /** OODA-26: Clear phase statistics from rebuild operation */
  clearStats?: ClearStats;
  /** CRITICAL: Tenant ID for multi-tenancy isolation */
  tenantId?: string;
  /** CRITICAL: Workspace ID for multi-tenancy isolation */
  workspaceId?: string;
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

/**
 * Chunk Progress Section - Objective C: Rebuild Operations Visibility
 *
 * @implements SPEC-001/Objective-C: Chunk-level progress for rebuilds
 * @implements OODA-24: Enhance PipelineStatusDialog with chunk progress
 *
 * WHY: Users need to see chunk-level progress during rebuilds to understand
 * the actual work being done, not just document counts.
 */
function ChunkProgressSection() {
  const { chunkProgress, hasActiveProgress } = useChunkProgress();

  // Format time for display
  const formatTime = (seconds: number): string => {
    if (seconds < 60) return `${Math.round(seconds)}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${Math.round(seconds % 60)}s`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  };

  // Format cost for display
  const formatCost = (cost: number): string => {
    if (cost < 0.0001) return '< $0.0001';
    if (cost < 0.01) return `$${cost.toFixed(4)}`;
    return `$${cost.toFixed(3)}`;
  };

  // Format tokens for display
  const formatTokens = (tokens: number): string => {
    if (tokens < 1000) return tokens.toString();
    if (tokens < 1000000) return `${(tokens / 1000).toFixed(1)}K`;
    return `${(tokens / 1000000).toFixed(2)}M`;
  };

  // Convert Map to array, filter recent, sort by recency
  const activeProgress = useMemo(() => {
    return Array.from(chunkProgress.values())
      .filter(p => {
        const age = Date.now() - p.lastUpdated.getTime();
        return age < 60000; // Show progress from last 60 seconds
      })
      .sort((a, b) => b.lastUpdated.getTime() - a.lastUpdated.getTime());
  }, [chunkProgress]);

  if (!hasActiveProgress || activeProgress.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2 text-sm font-medium">
        <Layers className="h-4 w-4" />
        Chunk Progress
        <Badge variant="outline" className="text-blue-500 border-blue-500 animate-pulse ml-auto">
          <Loader2 className="h-3 w-3 mr-1 animate-spin" />
          Live
        </Badge>
      </div>
      <div className="space-y-2">
        {activeProgress.slice(0, 3).map((progress) => (
          <div
            key={progress.documentId}
            className="p-2 rounded-lg border bg-muted/30 space-y-2 text-xs"
          >
            {/* Document header */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5 min-w-0">
                <FileText className="h-3 w-3 text-muted-foreground shrink-0" />
                <span className="font-medium truncate max-w-32">
                  {progress.documentId.split('-').slice(0, 2).join('-')}...
                </span>
              </div>
              <Badge variant="secondary" className="text-[10px]">
                {progress.percentComplete}%
              </Badge>
            </div>
            
            {/* Chunk progress bar */}
            <div className="space-y-1">
              <div className="flex items-center justify-between text-muted-foreground">
                <span className="flex items-center gap-1">
                  <Zap className="h-3 w-3" />
                  Chunk {progress.chunkIndex + 1} / {progress.totalChunks}
                </span>
                <span className="flex items-center gap-1">
                  <Timer className="h-3 w-3" />
                  ETA: {formatTime(progress.etaSeconds)}
                </span>
              </div>
              <Progress value={progress.percentComplete} className="h-1.5" />
            </div>

            {/* Metrics row */}
            <div className="flex items-center justify-between text-muted-foreground">
              <span>In: {formatTokens(progress.tokensIn)} | Out: {formatTokens(progress.tokensOut)}</span>
              <span className="flex items-center gap-1 text-green-600">
                <DollarSign className="h-3 w-3" />
                {formatCost(progress.costUsd)}
              </span>
            </div>
          </div>
        ))}
        {activeProgress.length > 3 && (
          <p className="text-xs text-muted-foreground text-center">
            +{activeProgress.length - 3} more documents processing
          </p>
        )}
      </div>
    </div>
  );
}

/**
 * Rebuild Phase Indicator - Objective C: Rebuild Operations Visibility
 *
 * @implements SPEC-001/Objective-C: Multi-phase progress for rebuilds
 * @implements OODA-25: Visual phase stepper for rebuild operations
 *
 * WHY: Users need to understand which phase of rebuild they're in:
 * - KG Rebuild: Clear → Extract → Embed (3 phases)
 * - Embed Rebuild: Clear → Embed (2 phases)
 */
interface RebuildPhase {
  id: string;
  label: string;
  icon: React.ReactNode;
  status: 'complete' | 'active' | 'pending';
}

function RebuildPhaseIndicator({
  jobName,
  processedDocs,
  totalDocs,
  isBusy,
}: {
  jobName?: string;
  processedDocs: number;
  totalDocs: number;
  isBusy: boolean;
}) {
  // Detect rebuild type from job_name prefix
  const isKgRebuild = jobName?.startsWith('rebuild_kg_');
  const isEmbedRebuild = jobName?.startsWith('rebuild_embed_');

  // Only show for rebuild operations
  if (!isKgRebuild && !isEmbedRebuild) return null;

  // Calculate current phase based on progress
  // Phase 1 (Clear): Always complete when we see status (instant operation)
  // Phase 2 (Extract): Active while processing docs (for KG rebuild)
  // Phase 3 (Embed): Final phase (for KG rebuild), or Phase 2 for embed rebuild
  const progressRatio = totalDocs > 0 ? processedDocs / totalDocs : 0;

  let phases: RebuildPhase[];

  if (isKgRebuild) {
    // KG Rebuild: Clear → Extract → Embed
    phases = [
      {
        id: 'clear',
        label: 'Clear',
        icon: <Eraser className="h-3 w-3" />,
        status: 'complete', // Clear is instant, always done when we see status
      },
      {
        id: 'extract',
        label: 'Extract',
        icon: <Sparkles className="h-3 w-3" />,
        status: progressRatio < 1 && isBusy ? 'active' : progressRatio >= 1 ? 'complete' : 'pending',
      },
      {
        id: 'embed',
        label: 'Embed',
        icon: <Database className="h-3 w-3" />,
        status: progressRatio >= 1 && !isBusy ? 'complete' : 'pending',
      },
    ];
  } else {
    // Embed Rebuild: Clear → Embed
    phases = [
      {
        id: 'clear',
        label: 'Clear',
        icon: <Eraser className="h-3 w-3" />,
        status: 'complete', // Clear is instant
      },
      {
        id: 'embed',
        label: 'Embed',
        icon: <Database className="h-3 w-3" />,
        status: progressRatio < 1 && isBusy ? 'active' : progressRatio >= 1 ? 'complete' : 'pending',
      },
    ];
  }

  // Find the active phase for description
  const activePhase = phases.find(p => p.status === 'active');
  const phaseDescriptions: Record<string, string> = {
    clear: 'Clearing existing data...',
    extract: 'Re-extracting entities from documents...',
    embed: 'Re-embedding vectors...',
  };

  return (
    <div className="p-3 bg-gradient-to-r from-blue-50 to-purple-50 dark:from-blue-950/30 dark:to-purple-950/30 rounded-lg border border-blue-200 dark:border-blue-800">
      {/* Header */}
      <div className="flex items-center gap-2 mb-3">
        <Activity className="h-4 w-4 text-blue-600" />
        <span className="text-sm font-medium">
          {isKgRebuild ? 'Knowledge Graph Rebuild' : 'Embeddings Rebuild'}
        </span>
      </div>

      {/* Phase Stepper */}
      <div className="flex items-center justify-between gap-2">
        {phases.map((phase, index) => (
          <div key={phase.id} className="flex items-center gap-2 flex-1">
            {/* Phase circle */}
            <div
              className={`
                flex items-center justify-center w-7 h-7 rounded-full shrink-0
                ${phase.status === 'complete' ? 'bg-green-500 text-white' : ''}
                ${phase.status === 'active' ? 'bg-blue-500 text-white animate-pulse' : ''}
                ${phase.status === 'pending' ? 'bg-gray-200 dark:bg-gray-700 text-gray-500' : ''}
              `}
            >
              {phase.status === 'complete' ? (
                <Check className="h-4 w-4" />
              ) : (
                phase.icon
              )}
            </div>

            {/* Phase label */}
            <span
              className={`text-xs font-medium ${
                phase.status === 'active' ? 'text-blue-600 dark:text-blue-400' : 
                phase.status === 'complete' ? 'text-green-600 dark:text-green-400' : 
                'text-gray-400'
              }`}
            >
              {phase.label}
            </span>

            {/* Connector line (except for last phase) */}
            {index < phases.length - 1 && (
              <div
                className={`flex-1 h-0.5 ${
                  phase.status === 'complete' ? 'bg-green-400' : 'bg-gray-200 dark:bg-gray-700'
                }`}
              />
            )}
          </div>
        ))}
      </div>

      {/* Active phase description */}
      {activePhase && (
        <p className="text-xs text-muted-foreground mt-2 text-center italic">
          {phaseDescriptions[activePhase.id] || 'Processing...'}
        </p>
      )}
    </div>
  );
}

/**
 * Clear Summary Section - Objective C: Rebuild Operations Visibility
 *
 * @implements OODA-26: Display clear phase statistics
 *
 * WHY: Users need to see what was cleared during rebuild to understand
 * the scope of the operation and verify it completed correctly.
 */
function ClearSummarySection({ clearStats }: { clearStats?: ClearStats }) {
  // Only show if any stats are provided
  if (
    !clearStats ||
    (clearStats.nodesCleared === undefined &&
      clearStats.edgesCleared === undefined &&
      clearStats.vectorsCleared === undefined)
  ) {
    return null;
  }

  const hasGraphStats =
    clearStats.nodesCleared !== undefined || clearStats.edgesCleared !== undefined;
  const hasVectorStats = clearStats.vectorsCleared !== undefined;

  // Determine grid columns based on what stats we have
  const statCount =
    (clearStats.nodesCleared !== undefined ? 1 : 0) +
    (clearStats.edgesCleared !== undefined ? 1 : 0) +
    (clearStats.vectorsCleared !== undefined ? 1 : 0);

  const gridClass =
    statCount === 3
      ? 'grid-cols-3'
      : statCount === 2
        ? 'grid-cols-2'
        : 'grid-cols-1';

  return (
    <div className="p-3 bg-green-50 dark:bg-green-950/30 rounded-lg border border-green-200 dark:border-green-800">
      <div className="flex items-center gap-2 mb-2">
        <Check className="h-4 w-4 text-green-600" />
        <span className="text-sm font-medium text-green-700 dark:text-green-400">
          Clear Phase Complete
        </span>
      </div>
      <div className={`grid ${gridClass} gap-2 text-sm`}>
        {clearStats.nodesCleared !== undefined && (
          <div className="text-center p-2 bg-white dark:bg-gray-900 rounded">
            <p className="text-[10px] text-muted-foreground uppercase tracking-wide">
              Entities
            </p>
            <p className="text-lg font-bold text-green-600">
              {clearStats.nodesCleared.toLocaleString()}
            </p>
          </div>
        )}
        {clearStats.edgesCleared !== undefined && (
          <div className="text-center p-2 bg-white dark:bg-gray-900 rounded">
            <p className="text-[10px] text-muted-foreground uppercase tracking-wide">
              Relations
            </p>
            <p className="text-lg font-bold text-green-600">
              {clearStats.edgesCleared.toLocaleString()}
            </p>
          </div>
        )}
        {clearStats.vectorsCleared !== undefined && (
          <div className="text-center p-2 bg-white dark:bg-gray-900 rounded">
            <p className="text-[10px] text-muted-foreground uppercase tracking-wide">
              Vectors
            </p>
            <p className="text-lg font-bold text-green-600">
              {clearStats.vectorsCleared.toLocaleString()}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

export function PipelineStatusDialog({
  open,
  onOpenChange,
  title,
  subtitle,
  clearStats,
  tenantId,
  workspaceId,
}: PipelineStatusDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [showCancelConfirm, setShowCancelConfirm] = useState(false);

  // Use enhanced pipeline status with history messages (Phase 3)
  // CRITICAL: Include tenantId and workspaceId for multi-tenancy isolation
  const { data, isLoading } = useQuery({
    queryKey: ['enhanced-pipeline-status', tenantId, workspaceId],
    queryFn: () => getEnhancedPipelineStatus(tenantId, workspaceId),
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
      toast.error(t('pipeline.cancelError', 'Failed to cancel pipeline'), {
        description: error instanceof Error ? error.message : t('common.unknownError', 'An unknown error occurred'),
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => cancelMutation.mutate(),
        },
      });
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

  // OODA-08: Calculate ETA based on processing rate
  const eta = useMemo(() => {
    if (!data?.job_start || !data.processed_documents || data.processed_documents === 0) {
      return null;
    }
    
    const startTime = new Date(data.job_start).getTime();
    const now = Date.now();
    const elapsedMs = now - startTime;
    const elapsedMinutes = elapsedMs / 60000;
    
    // Need at least 30 seconds of data for reasonable estimate
    if (elapsedMinutes < 0.5) {
      return t('pipeline.etaCalculating', 'Calculating...');
    }
    
    const rate = data.processed_documents / elapsedMinutes;
    const remaining = data.total_documents - data.processed_documents;
    
    if (remaining <= 0) {
      return t('pipeline.etaComplete', 'Almost done');
    }
    
    const etaMinutes = remaining / rate;
    
    if (etaMinutes < 1) {
      return t('pipeline.etaLessThanMinute', 'Less than a minute');
    }
    if (etaMinutes < 60) {
      return t('pipeline.etaMinutes', '~{{count}} min', { count: Math.ceil(etaMinutes) });
    }
    const hours = Math.floor(etaMinutes / 60);
    const mins = Math.ceil(etaMinutes % 60);
    if (mins === 0) {
      return t('pipeline.etaHours', '~{{count}} hour(s)', { count: hours });
    }
    return t('pipeline.etaHoursMinutes', '~{{hours}}h {{mins}}m', { hours, mins });
  }, [data?.job_start, data?.processed_documents, data?.total_documents, t]);
  
  // Use custom title or default
  const dialogTitle = title || t('pipeline.title', 'Pipeline Status');

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              {dialogTitle}
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
            {subtitle && (
              <p className="text-sm text-muted-foreground">{subtitle}</p>
            )}
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

              {/* OODA-25: Rebuild Phase Indicator - only shows for rebuild operations */}
              <RebuildPhaseIndicator
                jobName={data.job_name}
                processedDocs={data.processed_documents}
                totalDocs={data.total_documents}
                isBusy={data.is_busy}
              />

              {/* OODA-26: Clear Summary Section - shows what was cleared */}
              <ClearSummarySection clearStats={clearStats} />

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
                  {/* OODA-08: ETA display */}
                  {eta && (
                    <div className="flex items-center justify-center gap-1.5 text-xs text-muted-foreground">
                      <Clock className="h-3 w-3" />
                      <span>{t('pipeline.eta', 'ETA: {{time}}', { time: eta })}</span>
                    </div>
                  )}
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

              {/* OODA-24: Chunk-Level Progress Section */}
              <ChunkProgressSection />

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

              {/* REQ-23: Close button that closes dialog WITHOUT stopping rebuild */}
              {/* OODA-05: Reorder buttons - Close is default action (right side), Cancel is secondary (left side) */}
              <div className="flex gap-2">
                {/* Cancel Button - secondary action on the left */}
                <Button
                  variant="outline"
                  onClick={handleCancelClick}
                  disabled={cancelMutation.isPending || data.cancellation_requested}
                  className="flex-1"
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
                {/* Close Button - default action on the right */}
                <Button
                  variant="default"
                  onClick={() => onOpenChange(false)}
                  className="flex-1"
                  autoFocus
                >
                  {t('common.close', 'Close')}
                </Button>
              </div>
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
 * 
 * @param tenantId - CRITICAL: Tenant ID for multi-tenancy isolation
 * @param workspaceId - CRITICAL: Workspace ID for multi-tenancy isolation
 */
export function PipelineStatusIndicator({ 
  tenantId, 
  workspaceId 
}: { 
  tenantId?: string; 
  workspaceId?: string; 
}) {
  const { t } = useTranslation();
  
  const { data } = useQuery({
    queryKey: ['enhanced-pipeline-status', tenantId, workspaceId],
    queryFn: () => getEnhancedPipelineStatus(tenantId, workspaceId),
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
