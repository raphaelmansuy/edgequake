"use client";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { getQueueMetrics } from "@/lib/api/edgequake";
import {
  formatDurationSeconds,
  formatThroughput,
} from "@/lib/pipeline/pipeline-formatters";
import {
  scopedQueryKey,
  usePipelineWorkspace,
} from "@/lib/pipeline/pipeline-workspace-context";
import type { QueueMetrics } from "@/types";
import { useQuery } from "@tanstack/react-query";
import {
  AlertTriangle,
  Clock,
  Gauge,
  Loader2,
  Timer,
  Users,
  Zap,
} from "lucide-react";

export function PipelineQueueMetricsCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();

  const { data: metrics, isLoading } = useQuery<QueueMetrics>({
    queryKey: scopedQueryKey("queue-metrics", selectedTenantId, selectedWorkspaceId),
    queryFn: () =>
      getQueueMetrics(
        selectedTenantId ?? undefined,
        selectedWorkspaceId ?? undefined,
      ),
    refetchInterval: 3000,
  });

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
            className={`h-2 ${
              utilization >= 90
                ? "[&>div]:bg-red-500"
                : utilization >= 70
                  ? "[&>div]:bg-yellow-500"
                  : ""
            }`}
          />
        </div>

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
              {formatDurationSeconds(metrics?.avg_wait_time_seconds ?? 0)}
            </p>
          </div>
          <div className="p-2 bg-orange-50 dark:bg-orange-950 rounded text-center">
            <div className="flex items-center justify-center gap-1 text-xs text-muted-foreground mb-1">
              <Timer className="h-3 w-3" />
              <span>Queue ETA</span>
            </div>
            <p className="text-lg font-bold text-orange-600">
              {formatDurationSeconds(metrics?.estimated_queue_time_seconds ?? 0)}
            </p>
          </div>
        </div>

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
