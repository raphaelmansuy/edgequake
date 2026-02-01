/**
 * @module PipelineMonitor
 * @description Comprehensive pipeline monitoring component with real-time updates.
 *
 * Features:
 * - Real-time pipeline status overview
 * - Per-document processing stages
 * - Historical processing activity log
 * - Task queue visualization
 * - Processing metrics and ETA
 *
 * @implements FEAT0004 - Processing status tracking
 * @implements UC0007 - User monitors document processing progress
 * @implements OODA-11 - Stage progress visibility
 * @implements OODA-37 - Workspace isolation in pipeline monitor
 */
'use client';

import { StatusBadge, getDocumentDisplayStatus, isProcessingStatus, normalizeStatus } from '@/components/documents/status-badge';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useChunkProgress } from '@/hooks';
import {
    getDocuments,
    getEnhancedPipelineStatus,
    getQueueMetrics,
    getTasksList,
    requestPipelineCancellation,
} from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { PipelineMessage, QueueMetrics, TaskResponse } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
    Activity,
    AlertCircle,
    AlertTriangle,
    ArrowLeft,
    Brain,
    Building2,
    CheckCircle,
    ChevronDown,
    Clock,
    Cpu,
    DollarSign,
    FileText,
    Gauge,
    Layers,
    Loader2,
    RefreshCw,
    StopCircle,
    Timer,
    Users,
    XCircle,
    Zap
} from 'lucide-react';
import Link from 'next/link';
import { createContext, useContext, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

/**
 * Workspace Context for pipeline monitoring components.
 * 
 * WHY: All child components need access to tenant/workspace context
 * to properly scope their queries. Passing via context avoids prop drilling.
 * 
 * @implements OODA-37 - Workspace isolation for tenant data protection
 */
interface PipelineWorkspaceContext {
  selectedTenantId: string | null;
  selectedWorkspaceId: string | null;
  workspaceName: string;
}

const PipelineWorkspaceContext = createContext<PipelineWorkspaceContext>({
  selectedTenantId: null,
  selectedWorkspaceId: null,
  workspaceName: 'All Workspaces',
});

/**
 * Hook to access pipeline workspace context
 */
function usePipelineWorkspace(): PipelineWorkspaceContext {
  return useContext(PipelineWorkspaceContext);
}

/**
 * Generate scoped query key that includes workspace context.
 * 
 * WHY: Query keys must include workspace to prevent cache pollution
 * between tenants. This ensures data isolation at the cache level.
 */
function scopedQueryKey(base: string, tenantId: string | null, workspaceId: string | null): (string | null)[] {
  return [base, tenantId, workspaceId];
}

/**
 * Simplified Pipeline Phases - Accurate representation
 * 
 * @implements OODA-37 - Accurate pipeline visualization
 * 
 * WHY: The previous 4-stage display (Chunking → Extracting → Embedding → Indexing)
 * was MISLEADING. In reality, after chunking, chunks are processed in PARALLEL
 * through extract+embed. This simplified view accurately represents:
 * - Pending: Waiting to start
 * - Processing: Chunking + map-reduce extraction (combined)
 * - Completed: Successfully indexed
 * - Failed: Errors during processing
 */
const PIPELINE_PHASES = [
  { key: 'pending', label: 'Pending', icon: Clock, color: 'text-yellow-500', bgColor: 'bg-yellow-50 border-yellow-500' },
  { key: 'processing', label: 'Processing', icon: Zap, color: 'text-blue-500', bgColor: 'bg-blue-50 border-blue-500' },
  { key: 'completed', label: 'Completed', icon: CheckCircle, color: 'text-green-500', bgColor: 'bg-green-50 border-green-500' },
  { key: 'failed', label: 'Failed', icon: XCircle, color: 'text-red-500', bgColor: 'bg-red-50 border-red-500' },
] as const;

/**
 * Message level configuration
 */
const levelConfig = {
  info: { icon: Activity, color: 'text-blue-500', bgColor: 'bg-blue-50 dark:bg-blue-950' },
  warn: { icon: AlertCircle, color: 'text-yellow-500', bgColor: 'bg-yellow-50 dark:bg-yellow-950' },
  error: { icon: XCircle, color: 'text-red-500', bgColor: 'bg-red-50 dark:bg-red-950' },
} as const;

/**
 * Format task type for display
 */
function formatTaskType(taskType: string): string {
  return taskType
    .replace(/_/g, ' ')
    .replace(/\b\w/g, l => l.toUpperCase());
}

/**
 * Message Item Component
 * 
 * @implements OODA-37 - Human-readable activity messages
 * 
 * WHY: Users need to understand what's happening in plain language.
 * Technical IDs are confusing - document names are meaningful.
 */
function MessageItem({ message, documentMap }: { message: PipelineMessage; documentMap: Map<string, string> }) {
  const config = levelConfig[message.level as keyof typeof levelConfig] || levelConfig.info;
  const Icon = config.icon;

  // Format message to replace UUIDs with document names
  const formattedMessage = useMemo(() => {
    // UUID regex pattern
    const uuidPattern = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi;

    return message.message.replace(uuidPattern, (uuid) => {
      const docName = documentMap.get(uuid.toLowerCase());
      if (docName) {
        // Return document name, shortened if too long
        return docName.length > 30 ? `${docName.slice(0, 27)}...` : docName;
      }
      // If not found in documents, show shortened UUID
      return `doc-${uuid.slice(0, 8)}`;
    });
  }, [message.message, documentMap]);

  return (
    <div className={`flex items-start gap-2 py-1.5 px-2 rounded text-xs ${config.bgColor}`}>
      <Icon className={`h-3 w-3 mt-0.5 shrink-0 ${config.color}`} />
      <div className="flex-1 min-w-0">
        <p className="break-words">{formattedMessage}</p>
        <p className="text-[10px] text-muted-foreground mt-0.5">
          {formatDistanceToNow(new Date(message.timestamp), { addSuffix: true })}
        </p>
      </div>
    </div>
  );
}

/**
 * Chunk-Level Progress Card
 * 
 * @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
 * 
 * WHY: The real progression of document ingestion is chunks processed
 * vs chunks remaining. This component provides granular visibility into
 * the map-reduce extraction phase.
 */
function ChunkProgressCard() {
  const { chunkProgress, hasActiveProgress } = useChunkProgress();

  // Convert Map to array for rendering
  const activeProgress = useMemo(() => {
    return Array.from(chunkProgress.values())
      .filter(p => {
        const age = Date.now() - p.lastUpdated.getTime();
        return age < 60000; // Show progress from last 60 seconds
      })
      .sort((a, b) => b.lastUpdated.getTime() - a.lastUpdated.getTime());
  }, [chunkProgress]);

  // Format cost for display
  const formatCost = (cost: number) => {
    if (cost < 0.0001) return '< $0.0001';
    if (cost < 0.01) return `$${cost.toFixed(4)}`;
    return `$${cost.toFixed(3)}`;
  };

  // Format ETA for display
  const formatEta = (seconds: number) => {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  };

  // Format tokens for display
  const formatTokens = (tokens: number) => {
    if (tokens < 1000) return tokens.toString();
    if (tokens < 1000000) return `${(tokens / 1000).toFixed(1)}K`;
    return `${(tokens / 1000000).toFixed(2)}M`;
  };

  if (activeProgress.length === 0) {
    return null; // Don't show if no active chunk progress
  }

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg flex items-center gap-2">
          <Layers className="h-5 w-5" />
          Chunk Progress
          {hasActiveProgress && (
            <Badge variant="outline" className="text-blue-500 border-blue-500 animate-pulse">
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              Live
            </Badge>
          )}
        </CardTitle>
        <CardDescription>Real-time chunk-level extraction progress</CardDescription>
      </CardHeader>
      <CardContent>
        <ScrollArea className="h-64">
          <div className="space-y-4">
            {activeProgress.map((progress) => (
              <div
                key={progress.documentId}
                className="p-3 rounded-lg border bg-card space-y-3"
              >
                {/* Document header */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <FileText className="h-4 w-4 text-muted-foreground" />
                    <span className="text-sm font-medium truncate max-w-48">
                      {progress.documentId}
                    </span>
                  </div>
                  <Badge variant="secondary" className="text-xs">
                    {progress.percentComplete}%
                  </Badge>
                </div>

                {/* Chunk progress bar */}
                <div className="space-y-1">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <Zap className="h-3 w-3" />
                      Chunk {progress.chunkIndex + 1} / {progress.totalChunks}
                    </span>
                    <span className="flex items-center gap-1">
                      <Timer className="h-3 w-3" />
                      ETA: {formatEta(progress.etaSeconds)}
                    </span>
                  </div>
                  <Progress value={progress.percentComplete} className="h-2" />
                </div>

                {/* Current chunk preview */}
                {progress.chunkPreview && (
                  <div className="text-xs text-muted-foreground bg-muted/50 p-2 rounded">
                    <span className="text-foreground font-medium">Current: </span>
                    "{progress.chunkPreview.slice(0, 80)}..."
                  </div>
                )}

                {/* Metrics row */}
                <div className="grid grid-cols-3 gap-2 text-xs">
                  <div className="flex items-center gap-1 text-muted-foreground">
                    <Brain className="h-3 w-3" />
                    <span>In: {formatTokens(progress.tokensIn)}</span>
                  </div>
                  <div className="flex items-center gap-1 text-muted-foreground">
                    <Cpu className="h-3 w-3" />
                    <span>Out: {formatTokens(progress.tokensOut)}</span>
                  </div>
                  <div className="flex items-center gap-1 text-green-600">
                    <DollarSign className="h-3 w-3" />
                    <span>{formatCost(progress.costUsd)}</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}

/**
 * Pipeline Phases Visualization - Simplified and Accurate
 * 
 * @implements OODA-37 - Accurate pipeline phases with workspace scope
 * @implements SPEC-001/Issue-11 - Consolidated widget with Cancel button
 * 
 * WHY: Shows documents grouped by their actual phase (Pending → Processing → Completed/Failed)
 * instead of the misleading 4-stage model. This reflects the real processing flow.
 * The Cancel button is integrated here to avoid redundant PipelineProgressCard.
 */
function PipelineStagesCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();
  const queryClient = useQueryClient();

  const { data: documents } = useQuery({
    queryKey: scopedQueryKey('documents', selectedTenantId, selectedWorkspaceId),
    queryFn: () => getDocuments({ page: 1, page_size: 100 }),
    refetchInterval: 3000,
    select: (data) => data.items,
  });

  // Also fetch pipeline status for cancel functionality
  const { data: status } = useQuery({
    queryKey: scopedQueryKey('enhanced-pipeline-status', selectedTenantId, selectedWorkspaceId),
    queryFn: () => getEnhancedPipelineStatus(selectedTenantId ?? undefined, selectedWorkspaceId ?? undefined),
    refetchInterval: 2000,
  });

  const cancelMutation = useMutation({
    mutationFn: requestPipelineCancellation,
    onSuccess: () => {
      toast.success('Pipeline cancellation requested');
      queryClient.invalidateQueries({ queryKey: scopedQueryKey('enhanced-pipeline-status', selectedTenantId, selectedWorkspaceId) });
    },
    onError: (error) => {
      toast.error(`Cancel failed: ${error instanceof Error ? error.message : 'Unknown'}`);
    },
  });

  // Count documents by phase (simplified from 4 stages to 4 phases)
  const phaseCounts = useMemo(() => {
    if (!documents) return { pending: 0, processing: 0, completed: 0, failed: 0 };

    return documents.reduce<Record<string, number>>((acc, doc) => {
      const status = normalizeStatus(doc.status);

      // Map all processing-related statuses to 'processing'
      // isProcessingStatus already includes: processing, chunking, extracting, embedding, indexing
      if (isProcessingStatus(status)) {
        acc.processing = (acc.processing || 0) + 1;
      } else if (status === 'pending') {
        acc.pending = (acc.pending || 0) + 1;
      } else if (status === 'completed' || status === 'indexed') {
        acc.completed = (acc.completed || 0) + 1;
      } else if (status === 'failed' || status === 'cancelled') {
        acc.failed = (acc.failed || 0) + 1;
      }
      return acc;
    }, { pending: 0, processing: 0, completed: 0, failed: 0 });
  }, [documents]);

  const totalDocs = documents?.length || 0;
  const isActive = phaseCounts.processing > 0 || phaseCounts.pending > 0;

  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-lg flex items-center gap-2">
              <Layers className="h-5 w-5" />
              Document Pipeline
            </CardTitle>
            <CardDescription>
              {totalDocs} documents in workspace
            </CardDescription>
          </div>
          {/* Integrated Cancel Button - replaces redundant PipelineProgressCard */}
          {status?.is_busy && (
            <Button
              variant="destructive"
              size="sm"
              onClick={() => cancelMutation.mutate()}
              disabled={cancelMutation.isPending || status.cancellation_requested}
            >
              {cancelMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <StopCircle className="mr-2 h-4 w-4" />
              )}
              {status.cancellation_requested ? 'Cancelling...' : 'Cancel'}
            </Button>
          )}
          {!status?.is_busy && isActive && (
            <Badge variant="outline" className="text-yellow-500 border-yellow-500">
              <Clock className="h-3 w-3 mr-1" />
              Queued
            </Badge>
          )}
          {!status?.is_busy && !isActive && totalDocs > 0 && (
            <Badge variant="outline" className="text-green-500 border-green-500">
              <CheckCircle className="h-3 w-3 mr-1" />
              Idle
            </Badge>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {PIPELINE_PHASES.map((phase) => {
            const count = phaseCounts[phase.key] || 0;
            const Icon = phase.icon;
            const isActivePhase = count > 0;

            return (
              <div
                key={phase.key}
                className={`flex flex-col items-center p-4 rounded-lg border-2 transition-all ${isActivePhase ? phase.bgColor : 'bg-muted/50 border-muted'
                  }`}
              >
                <Icon className={`h-6 w-6 ${isActivePhase ? phase.color : 'text-muted-foreground'} ${phase.key === 'processing' && isActivePhase ? 'animate-pulse' : ''
                  }`} />
                <span className={`text-sm font-medium mt-2 ${isActivePhase ? phase.color : 'text-muted-foreground'}`}>
                  {phase.label}
                </span>
                <span className={`text-2xl font-bold ${isActivePhase ? phase.color : 'text-muted-foreground'}`}>
                  {count}
                </span>
              </div>
            );
          })}
        </div>

        {/* Progress bar showing overall completion */}
        {totalDocs > 0 && (
          <div className="mt-4">
            <div className="flex justify-between text-xs text-muted-foreground mb-1">
              <span>Pipeline Progress</span>
              <span>{phaseCounts.completed} / {totalDocs} completed</span>
            </div>
            <Progress
              value={totalDocs > 0 ? (phaseCounts.completed / totalDocs) * 100 : 0}
              className="h-2"
            />
          </div>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Activity Log Component
 * 
 * @implements OODA-37 - Workspace-scoped activity log with human-readable messages
 * 
 * WHY: Activity log must only show events for the current workspace
 * and display document names instead of cryptic UUIDs for user clarity.
 */
function ActivityLogCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();

  const { data: status } = useQuery({
    queryKey: scopedQueryKey('enhanced-pipeline-status', selectedTenantId, selectedWorkspaceId),
    queryFn: () => getEnhancedPipelineStatus(selectedTenantId ?? undefined, selectedWorkspaceId ?? undefined),
    refetchInterval: 2000,
  });

  // Fetch documents to build ID → Name lookup
  const { data: documentsData } = useQuery({
    queryKey: scopedQueryKey('documents', selectedTenantId, selectedWorkspaceId),
    queryFn: () => getDocuments({ page: 1, page_size: 100 }),
    refetchInterval: 5000,
  });

  // Build document ID → name map
  const documentMap = useMemo(() => {
    const map = new Map<string, string>();
    if (documentsData?.items) {
      for (const doc of documentsData.items) {
        const displayName = doc.title || doc.file_name || `Document ${doc.id.slice(0, 8)}`;
        map.set(doc.id.toLowerCase(), displayName);
      }
    }
    return map;
  }, [documentsData?.items]);

  const messages = status?.history_messages || [];

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg flex items-center gap-2">
          <Activity className="h-5 w-5" />
          Activity Log
        </CardTitle>
        <CardDescription>Recent pipeline events</CardDescription>
      </CardHeader>
      <CardContent>
        {messages.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">
            No recent activity
          </p>
        ) : (
          <ScrollArea className="h-64">
            <div className="space-y-1">
              {[...messages].reverse().map((msg, idx) => (
                <MessageItem key={idx} message={msg} documentMap={documentMap} />
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Queue Metrics Card - Objective B: Workspace-Level Task Queue Visibility
 *
 * @implements FEAT0570 - Queue metrics display
 * @implements OODA-21 - Queue metrics frontend integration
 * @implements OODA-37 - Workspace-scoped queue metrics
 * @implements OODA-04 - Multi-tenant isolation for queue metrics
 *
 * WHY: Users need visibility into queue state for capacity planning:
 * - Worker utilization shows processing capacity
 * - Throughput rate indicates processing speed
 * - Wait time estimates help set expectations
 *
 * CRITICAL: Queue metrics MUST be filtered by tenant/workspace to ensure
 * users only see activity from their own workspace. Without this isolation,
 * users could see "Live" indicator when other tenants are processing documents.
 */
function QueueMetricsCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();

  const { data: metrics, isLoading } = useQuery<QueueMetrics>({
    queryKey: scopedQueryKey('queue-metrics', selectedTenantId, selectedWorkspaceId),
    queryFn: () => getQueueMetrics(
      selectedTenantId ?? undefined,
      selectedWorkspaceId ?? undefined,
    ),
    refetchInterval: 3000,
  });

  // Format time for display
  const formatTime = (seconds: number): string => {
    if (seconds < 60) return `${Math.round(seconds)}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${Math.round(seconds % 60)}s`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  };

  // Format throughput for display
  const formatThroughput = (docsPerMin: number): string => {
    if (docsPerMin < 0.1) return '< 0.1/min';
    if (docsPerMin < 1) return `${docsPerMin.toFixed(1)}/min`;
    return `${Math.round(docsPerMin)}/min`;
  };

  if (isLoading) {
    return (
      <Card>
        <CardContent className="p-6 flex flex-col items-center justify-center gap-2">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          <p className="text-sm text-muted-foreground">Loading queue metrics...</p>
        </CardContent>
      </Card>
    );
  }

  const utilization = metrics?.worker_utilization ?? 0;
  const activeWorkers = metrics?.active_workers ?? 0;
  const maxWorkers = metrics?.max_workers ?? 1;
  const pendingCount = metrics?.pending_count ?? 0;
  const isActive = pendingCount > 0 || activeWorkers > 0;

  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <Gauge className="h-5 w-5" />
            Queue Metrics
          </CardTitle>
          {isActive && (
            <Badge variant="outline" className="text-blue-500 border-blue-500 animate-pulse">
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              Live
            </Badge>
          )}
        </div>
        <CardDescription>Task queue capacity and performance</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Worker Utilization Gauge */}
        <div className="space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="flex items-center gap-1.5 text-muted-foreground">
              <Users className="h-4 w-4" />
              Workers
            </span>
            <span className="font-medium">
              {activeWorkers}/{maxWorkers} ({utilization}%)
            </span>
          </div>
          <Progress
            value={utilization}
            className={`h-2 ${utilization >= 90 ? '[&>div]:bg-red-500' : utilization >= 70 ? '[&>div]:bg-yellow-500' : ''}`}
          />
        </div>

        {/* Metrics Tiles */}
        <div className="grid grid-cols-3 gap-2 text-sm">
          <div className="p-2 bg-blue-50 dark:bg-blue-950 rounded text-center">
            <div className="flex items-center justify-center gap-1 text-xs text-muted-foreground mb-1">
              <Zap className="h-3 w-3" />
              <span>Throughput</span>
            </div>
            <p className="text-lg font-bold text-blue-600">
              {formatThroughput(metrics?.throughput_per_minute ?? 0)}
            </p>
          </div>
          <div className="p-2 bg-purple-50 dark:bg-purple-950 rounded text-center">
            <div className="flex items-center justify-center gap-1 text-xs text-muted-foreground mb-1">
              <Clock className="h-3 w-3" />
              <span>Avg Wait</span>
            </div>
            <p className="text-lg font-bold text-purple-600">
              {formatTime(metrics?.avg_wait_time_seconds ?? 0)}
            </p>
          </div>
          <div className="p-2 bg-orange-50 dark:bg-orange-950 rounded text-center">
            <div className="flex items-center justify-center gap-1 text-xs text-muted-foreground mb-1">
              <Timer className="h-3 w-3" />
              <span>Queue ETA</span>
            </div>
            <p className="text-lg font-bold text-orange-600">
              {formatTime(metrics?.estimated_queue_time_seconds ?? 0)}
            </p>
          </div>
        </div>

        {/* Queue Status Footer */}
        <div className="flex items-center justify-between text-xs text-muted-foreground pt-2 border-t">
          <span>Queue: {pendingCount} pending</span>
          {metrics?.rate_limited && (
            <Badge variant="destructive" className="text-[10px]">
              <AlertTriangle className="h-3 w-3 mr-1" />
              Rate Limited
            </Badge>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

/**
 * Processing Documents Table
 * 
 * @implements OODA-37 - Workspace-scoped processing documents
 */
function ProcessingDocumentsCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();

  const { data: documents, isLoading } = useQuery({
    queryKey: scopedQueryKey('documents', selectedTenantId, selectedWorkspaceId),
    queryFn: () => getDocuments({ page: 1, page_size: 50 }),
    refetchInterval: 2000,
    select: (data) => data.items.filter((d) => isProcessingStatus(normalizeStatus(d.status))),
  });

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg flex items-center gap-2">
          <FileText className="h-5 w-5" />
          Processing Documents
          {documents && documents.length > 0 && (
            <Badge variant="secondary">{documents.length}</Badge>
          )}
        </CardTitle>
        <CardDescription>Documents currently in the pipeline</CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex flex-col justify-center items-center gap-2 py-4">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <p className="text-sm text-muted-foreground">Loading documents...</p>
          </div>
        ) : documents && documents.length > 0 ? (
          <ScrollArea className="h-64">
            <div className="space-y-2">
              {documents.map((doc) => (
                <div
                  key={doc.id}
                  className="flex items-center justify-between p-2 rounded-lg border bg-card"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
                    <div className="min-w-0">
                      <p className="text-sm font-medium truncate">
                        {doc.title || doc.file_name || doc.id.slice(0, 8)}
                      </p>
                      <p className="text-xs text-muted-foreground">
                        {doc.content_length ? `${(doc.content_length / 1024).toFixed(1)} KB` : 'Unknown size'}
                      </p>
                    </div>
                  </div>
                  <StatusBadge status={getDocumentDisplayStatus(doc)} />
                </div>
              ))}
            </div>
          </ScrollArea>
        ) : (
          <p className="text-sm text-muted-foreground text-center py-4">
            No documents currently processing
          </p>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Task Queue Card - Enhanced for Objective B: Wait Time Per Document
 *
 * @implements FEAT0572 - Wait time per document in queue
 * @implements OODA-22 - Queue order and wait time display
 * @implements OODA-37 - Workspace-scoped task queue
 *
 * WHY: Users need to see queue position and wait time to understand
 * when their documents will be processed.
 */
function TaskQueueCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();

  const { data: tasks, isLoading } = useQuery({
    queryKey: scopedQueryKey('tasks', selectedTenantId, selectedWorkspaceId),
    queryFn: () => getTasksList({ page_size: 50 }),
    refetchInterval: 3000,
  });

  // Format wait time for display
  const formatWaitTime = (createdAt: string): string => {
    const waitMs = Date.now() - new Date(createdAt).getTime();
    const seconds = Math.floor(waitMs / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    if (minutes < 60) return `${minutes}m ${remainingSeconds}s`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ${minutes % 60}m`;
  };

  // Split tasks into pending and processing, sorted by wait time
  const { pendingTasks, processingTasks } = useMemo(() => {
    if (!tasks?.tasks) return { pendingTasks: [], processingTasks: [] };

    const pending = tasks.tasks
      .filter((t: TaskResponse) => t.status === 'pending')
      .sort((a: TaskResponse, b: TaskResponse) =>
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      );

    const processing = tasks.tasks
      .filter((t: TaskResponse) => t.status === 'processing')
      .sort((a: TaskResponse, b: TaskResponse) =>
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      );

    return { pendingTasks: pending, processingTasks: processing };
  }, [tasks]);

  const totalWaiting = pendingTasks.length;

  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <Clock className="h-5 w-5" />
            Task Queue
          </CardTitle>
          {totalWaiting > 0 && (
            <Badge variant="outline" className="text-yellow-500 border-yellow-500">
              {totalWaiting} waiting
            </Badge>
          )}
        </div>
        <CardDescription>Pending and processing tasks with wait times</CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex flex-col justify-center items-center gap-2 py-4">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <p className="text-sm text-muted-foreground">Loading task queue...</p>
          </div>
        ) : pendingTasks.length === 0 && processingTasks.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">
            No pending or processing tasks
          </p>
        ) : (
          <ScrollArea className="h-64">
            <div className="space-y-4">
              {/* Pending Tasks Section */}
              {pendingTasks.length > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                    <Clock className="h-3 w-3" />
                    PENDING ({pendingTasks.length})
                  </div>
                  <div className="space-y-1">
                    {pendingTasks.slice(0, 10).map((task: TaskResponse, index: number) => (
                      <div
                        key={task.track_id}
                        className="flex items-center justify-between py-1.5 px-2 rounded bg-yellow-50/50 dark:bg-yellow-950/30 text-xs"
                      >
                        <div className="flex items-center gap-2 min-w-0">
                          <span className="font-bold text-yellow-600 w-4">#{index + 1}</span>
                          <span className="font-medium truncate max-w-32">
                            {formatTaskType(task.task_type)}
                          </span>
                        </div>
                        <div className="flex items-center gap-2 text-muted-foreground">
                          <Timer className="h-3 w-3" />
                          <span>{formatWaitTime(task.created_at)}</span>
                        </div>
                      </div>
                    ))}
                    {pendingTasks.length > 10 && (
                      <p className="text-xs text-muted-foreground text-center py-1">
                        +{pendingTasks.length - 10} more in queue
                      </p>
                    )}
                  </div>
                </div>
              )}

              {/* Processing Tasks Section */}
              {processingTasks.length > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                    <Loader2 className="h-3 w-3 animate-spin" />
                    PROCESSING ({processingTasks.length})
                  </div>
                  <div className="space-y-1">
                    {processingTasks.map((task: TaskResponse) => (
                      <div
                        key={task.track_id}
                        className="flex items-center justify-between py-1.5 px-2 rounded bg-blue-50/50 dark:bg-blue-950/30 text-xs"
                      >
                        <div className="flex items-center gap-2 min-w-0">
                          <Loader2 className="h-3 w-3 animate-spin text-blue-500" />
                          <span className="font-medium truncate max-w-32">
                            {formatTaskType(task.task_type)}
                          </span>
                        </div>
                        <div className="flex items-center gap-2 text-muted-foreground">
                          <span>Started {formatWaitTime(task.started_at || task.created_at)} ago</span>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Main Pipeline Monitor Component
 * 
 * @implements OODA-37 - Workspace isolation for tenant data protection
 * 
 * WHY: The Pipeline Monitor provides context for all child components
 * to ensure they query data only for the current workspace. This prevents
 * data leakage between tenants.
 */
export function PipelineMonitor() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { selectedTenantId, selectedWorkspaceId, workspaces } = useTenantStore();

  // Get workspace name for display
  const currentWorkspace = workspaces.find(w => w.id === selectedWorkspaceId);
  const workspaceName = currentWorkspace?.name || 'All Workspaces';

  // Create context value for child components
  const workspaceContext: PipelineWorkspaceContext = {
    selectedTenantId,
    selectedWorkspaceId,
    workspaceName,
  };

  // Handler for refresh button using scoped queries
  const handleRefresh = () => {
    queryClient.invalidateQueries({
      queryKey: scopedQueryKey('enhanced-pipeline-status', selectedTenantId, selectedWorkspaceId)
    });
    queryClient.invalidateQueries({
      queryKey: scopedQueryKey('documents', selectedTenantId, selectedWorkspaceId)
    });
    queryClient.invalidateQueries({
      queryKey: scopedQueryKey('tasks', selectedTenantId, selectedWorkspaceId)
    });
    queryClient.invalidateQueries({
      queryKey: scopedQueryKey('queue-metrics', selectedTenantId, selectedWorkspaceId)
    });
    toast.success('Refreshed');
  };

  return (
    <PipelineWorkspaceContext.Provider value={workspaceContext}>
      <div className="flex flex-col h-[calc(100vh-theme(spacing.20))]">
        {/* Fixed Header */}
        <div className="flex-shrink-0 sticky top-0 z-10 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
          <div className="container mx-auto px-6 py-4 max-w-7xl">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <Link href="/documents">
                  <Button variant="ghost" size="sm">
                    <ArrowLeft className="h-4 w-4 mr-2" />
                    Back to Documents
                  </Button>
                </Link>
                <div>
                  <h1 className="text-2xl font-bold">Pipeline Monitor</h1>
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Building2 className="h-4 w-4" />
                    <span>{workspaceName}</span>
                    {!selectedWorkspaceId && (
                      <Badge variant="destructive" className="text-xs">
                        No workspace selected
                      </Badge>
                    )}
                  </div>
                </div>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={handleRefresh}
              >
                <RefreshCw className="h-4 w-4 mr-2" />
                Refresh
              </Button>
            </div>
          </div>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto">
          <div className="container mx-auto p-4 sm:p-6 max-w-7xl pb-8">
            {/* Pipeline Stages Overview - CRITICAL INFO AT TOP */}
            <PipelineStagesCard />

            {/* Chunk-Level Progress (Real-time) - ACTIVE PROCESSING */}
            <div className="mt-4 sm:mt-6">
              <ChunkProgressCard />
            </div>

            {/* Processing Documents - ACTIVE WORK */}
            <div className="mt-4 sm:mt-6">
              <ProcessingDocumentsCard />
            </div>

            {/* Secondary Info Grid - RESPONSIVE LAYOUT */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6 mt-4 sm:mt-6">
              {/* Queue Metrics - Operational */}
              <QueueMetricsCard />

              {/* Activity Log - History */}
              <ActivityLogCard />
            </div>

            {/* Collapsible Advanced Details - SPEC-001/Issue-12 */}
            <details className="mt-4 sm:mt-6 mb-4 group">
              <summary className="cursor-pointer list-none flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors">
                <ChevronDown className="h-4 w-4 transition-transform group-open:rotate-180" />
                <span>Advanced Details</span>
              </summary>
              <div className="mt-4">
                <TaskQueueCard />
              </div>
            </details>
          </div>
        </div>
      </div>
    </PipelineWorkspaceContext.Provider>
  );
}
