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
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  getDocuments,
  getEnhancedPipelineStatus,
  getTasksList,
  requestPipelineCancellation,
} from '@/lib/api/edgequake';
import type { PipelineMessage, TaskResponse } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
  Activity,
  AlertCircle,
  ArrowLeft,
  Brain,
  CheckCircle,
  Clock,
  Cpu,
  Database,
  FileText,
  Loader2,
  RefreshCw,
  Scissors,
  StopCircle,
  XCircle,
} from 'lucide-react';
import Link from 'next/link';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { StatusBadge, normalizeStatus, isProcessingStatus } from '@/components/documents/status-badge';

/**
 * Pipeline Stages for visualization
 */
const PIPELINE_STAGES = [
  { key: 'chunking', label: 'Chunking', icon: Scissors, description: 'Splitting document into chunks' },
  { key: 'extracting', label: 'Extracting', icon: Brain, description: 'LLM entity extraction' },
  { key: 'embedding', label: 'Embedding', icon: Cpu, description: 'Vector embeddings' },
  { key: 'indexing', label: 'Indexing', icon: Database, description: 'Graph storage' },
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
 */
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
 * Pipeline Progress Card
 */
function PipelineProgressCard() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  
  const { data: status, isLoading } = useQuery({
    queryKey: ['enhanced-pipeline-status'],
    queryFn: getEnhancedPipelineStatus,
    refetchInterval: 2000,
  });
  
  const cancelMutation = useMutation({
    mutationFn: requestPipelineCancellation,
    onSuccess: () => {
      toast.success('Pipeline cancellation requested');
      queryClient.invalidateQueries({ queryKey: ['enhanced-pipeline-status'] });
    },
    onError: (error) => {
      toast.error(`Cancel failed: ${error instanceof Error ? error.message : 'Unknown'}`);
    },
  });
  
  // Calculate progress and ETA
  const totalDocs = status?.total_documents ?? 0;
  const processedDocs = status?.processed_documents ?? 0;
  const progress = totalDocs > 0
    ? (processedDocs / totalDocs) * 100
    : 0;
  
  const eta = useMemo(() => {
    if (!status?.job_start || !processedDocs) return null;
    
    const elapsedMs = Date.now() - new Date(status.job_start).getTime();
    if (elapsedMs < 30000) return 'Calculating...';
    
    const rate = processedDocs / (elapsedMs / 60000);
    const remaining = totalDocs - processedDocs;
    if (remaining <= 0) return 'Almost done';
    
    const etaMinutes = remaining / rate;
    if (etaMinutes < 1) return 'Less than a minute';
    if (etaMinutes < 60) return `~${Math.ceil(etaMinutes)} min`;
    return `~${Math.floor(etaMinutes / 60)}h ${Math.ceil(etaMinutes % 60)}m`;
  }, [status?.job_start, processedDocs, totalDocs]);
  
  if (isLoading) {
    return (
      <Card>
        <CardContent className="p-6 flex items-center justify-center">
          <Loader2 className="h-6 w-6 animate-spin" />
        </CardContent>
      </Card>
    );
  }
  
  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Pipeline Status
          </CardTitle>
          {status?.is_busy && (
            <Badge variant="outline" className="text-orange-500 border-orange-500 animate-pulse">
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              Active
            </Badge>
          )}
          {!status?.is_busy && (
            <Badge variant="outline" className="text-green-500 border-green-500">
              <CheckCircle className="h-3 w-3 mr-1" />
              Idle
            </Badge>
          )}
        </div>
        {status?.job_name && (
          <CardDescription>{status.job_name}</CardDescription>
        )}
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Progress */}
        {(status?.total_documents ?? 0) > 0 && (
          <div className="space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">
                {status?.processed_documents ?? 0}/{status?.total_documents ?? 0} documents
              </span>
              <span className="font-medium">{Math.round(progress)}%</span>
            </div>
            <Progress value={progress} className="h-2" />
            {eta && (
              <div className="flex items-center justify-center gap-1.5 text-xs text-muted-foreground">
                <Clock className="h-3 w-3" />
                <span>ETA: {eta}</span>
              </div>
            )}
          </div>
        )}
        
        {/* Task Stats Grid */}
        <div className="grid grid-cols-4 gap-2 text-sm">
          <div className="p-2 bg-yellow-50 dark:bg-yellow-950 rounded text-center">
            <p className="text-xs text-muted-foreground">Pending</p>
            <p className="text-xl font-bold text-yellow-600">{status?.pending_tasks ?? 0}</p>
          </div>
          <div className="p-2 bg-blue-50 dark:bg-blue-950 rounded text-center">
            <p className="text-xs text-muted-foreground">Processing</p>
            <p className="text-xl font-bold text-blue-600">{status?.processing_tasks ?? 0}</p>
          </div>
          <div className="p-2 bg-green-50 dark:bg-green-950 rounded text-center">
            <p className="text-xs text-muted-foreground">Completed</p>
            <p className="text-xl font-bold text-green-600">{status?.completed_tasks ?? 0}</p>
          </div>
          <div className="p-2 bg-red-50 dark:bg-red-950 rounded text-center">
            <p className="text-xs text-muted-foreground">Failed</p>
            <p className="text-xl font-bold text-red-600">{status?.failed_tasks ?? 0}</p>
          </div>
        </div>
        
        {/* Cancel Button */}
        {status?.is_busy && (
          <Button
            variant="destructive"
            size="sm"
            onClick={() => cancelMutation.mutate()}
            disabled={cancelMutation.isPending || status.cancellation_requested}
            className="w-full"
          >
            {cancelMutation.isPending ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <StopCircle className="mr-2 h-4 w-4" />
            )}
            {status.cancellation_requested ? 'Cancellation Pending...' : 'Cancel Pipeline'}
          </Button>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Pipeline Stages Visualization
 */
function PipelineStagesCard() {
  const { data: documents } = useQuery({
    queryKey: ['documents'],
    queryFn: () => getDocuments({ page: 1, page_size: 100 }),
    refetchInterval: 3000,
    select: (data) => data.items,
  });
  
  // Count documents at each stage
  const stageCounts = useMemo(() => {
    if (!documents) return {};
    return documents.reduce<Record<string, number>>((acc, doc) => {
      const status = normalizeStatus(doc.status);
      acc[status] = (acc[status] || 0) + 1;
      return acc;
    }, {});
  }, [documents]);
  
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg">Processing Stages</CardTitle>
        <CardDescription>Documents in each pipeline stage</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex items-center justify-between gap-2">
          {PIPELINE_STAGES.map((stage, index) => {
            const count = stageCounts[stage.key] || 0;
            const Icon = stage.icon;
            return (
              <div key={stage.key} className="flex items-center gap-2">
                <div className={`flex flex-col items-center p-3 rounded-lg border ${
                  count > 0 ? 'bg-primary/10 border-primary' : 'bg-muted'
                }`}>
                  <Icon className={`h-5 w-5 ${count > 0 ? 'text-primary animate-pulse' : 'text-muted-foreground'}`} />
                  <span className="text-xs font-medium mt-1">{stage.label}</span>
                  <span className={`text-lg font-bold ${count > 0 ? 'text-primary' : 'text-muted-foreground'}`}>
                    {count}
                  </span>
                </div>
                {index < PIPELINE_STAGES.length - 1 && (
                  <span className="text-muted-foreground">→</span>
                )}
              </div>
            );
          })}
          {/* Completed */}
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground">→</span>
            <div className={`flex flex-col items-center p-3 rounded-lg border ${
              stageCounts['completed'] > 0 ? 'bg-green-50 border-green-500' : 'bg-muted'
            }`}>
              <CheckCircle className={`h-5 w-5 ${stageCounts['completed'] > 0 ? 'text-green-500' : 'text-muted-foreground'}`} />
              <span className="text-xs font-medium mt-1">Done</span>
              <span className={`text-lg font-bold ${stageCounts['completed'] > 0 ? 'text-green-500' : 'text-muted-foreground'}`}>
                {stageCounts['completed'] || 0}
              </span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

/**
 * Activity Log Component
 */
function ActivityLogCard() {
  const { data: status } = useQuery({
    queryKey: ['enhanced-pipeline-status'],
    queryFn: getEnhancedPipelineStatus,
    refetchInterval: 2000,
  });
  
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
                <MessageItem key={idx} message={msg} />
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Processing Documents Table
 */
function ProcessingDocumentsCard() {
  const { data: documents, isLoading } = useQuery({
    queryKey: ['documents'],
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
          <div className="flex justify-center py-4">
            <Loader2 className="h-6 w-6 animate-spin" />
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
                  <StatusBadge status={normalizeStatus(doc.status)} />
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
 * Task Queue Card
 */
function TaskQueueCard() {
  const { data: tasks, isLoading } = useQuery({
    queryKey: ['tasks'],
    queryFn: () => getTasksList({ page_size: 20 }),
    refetchInterval: 3000,
  });
  
  const recentTasks = tasks?.tasks?.slice(0, 10) || [];
  
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-lg">Task Queue</CardTitle>
        <CardDescription>Recent background tasks</CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex justify-center py-4">
            <Loader2 className="h-6 w-6 animate-spin" />
          </div>
        ) : recentTasks.length > 0 ? (
          <ScrollArea className="h-48">
            <div className="space-y-1">
              {recentTasks.map((task: TaskResponse) => (
                <div
                  key={task.track_id}
                  className="flex items-center justify-between py-1.5 px-2 rounded hover:bg-muted/50 text-xs"
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="font-medium">{formatTaskType(task.task_type)}</span>
                  </div>
                  <Badge
                    variant={
                      task.status === 'indexed' ? 'default' :
                      task.status === 'failed' ? 'destructive' :
                      task.status === 'processing' ? 'secondary' :
                      'outline'
                    }
                    className="text-[10px]"
                  >
                    {task.status}
                  </Badge>
                </div>
              ))}
            </div>
          </ScrollArea>
        ) : (
          <p className="text-sm text-muted-foreground text-center py-4">
            No recent tasks
          </p>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * Main Pipeline Monitor Component
 */
export function PipelineMonitor() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  
  return (
    <div className="container mx-auto p-6 max-w-7xl">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-4">
          <Link href="/documents">
            <Button variant="ghost" size="sm">
              <ArrowLeft className="h-4 w-4 mr-2" />
              Back to Documents
            </Button>
          </Link>
          <div>
            <h1 className="text-2xl font-bold">Pipeline Monitor</h1>
            <p className="text-muted-foreground">Real-time document ingestion tracking</p>
          </div>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            queryClient.invalidateQueries({ queryKey: ['enhanced-pipeline-status'] });
            queryClient.invalidateQueries({ queryKey: ['documents'] });
            queryClient.invalidateQueries({ queryKey: ['tasks'] });
            toast.success('Refreshed');
          }}
        >
          <RefreshCw className="h-4 w-4 mr-2" />
          Refresh
        </Button>
      </div>
      
      {/* Pipeline Stages Overview */}
      <PipelineStagesCard />
      
      {/* Main Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-6">
        {/* Left Column */}
        <div className="space-y-6">
          <PipelineProgressCard />
          <ProcessingDocumentsCard />
        </div>
        
        {/* Right Column */}
        <div className="space-y-6">
          <ActivityLogCard />
          <TaskQueueCard />
        </div>
      </div>
    </div>
  );
}
