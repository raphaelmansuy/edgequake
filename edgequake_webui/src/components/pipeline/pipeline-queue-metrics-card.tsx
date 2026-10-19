"use client";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { getQueueMetrics } from "@/lib/api/edgequake";
import { isInitialLoading } from "@/lib/layout/cls-stability";
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
    placeholderData: (previous) => previous,
  });

  const cold = isInitialLoading(isLoading, Boolean(metrics));
  const utilization = metrics?.worker_utilization ?? 0;
  const activeWorkers = metrics?.active_workers ?? 0;
  const maxWorkers = metrics?.max_workers ?? 1;
  const pendingCount = metrics?.pending_count ?? 0;
  const isActive = pendingCount > 0 || activeWorkers > 0;

  return (
    <Card data-testid="spec100-pipeline-queue-metrics" className="min-h-[280px]">
      <CardHeader className="pb-2">
        <div className="eq-toolbar justify-between">
          <CardTitle className="flex items-center gap-2">
            <Gauge className="h-4 w-4 text-muted-foreground" />
            Queue Metrics
          </CardTitle>
          {isActive && !cold && (
            <Badge variant="outline" className="text-blue-600 dark:text-blue-400 border-blue-500">
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              Live
            </Badge>
          )}
        </div>
        <CardDescription>Task queue capacity and performance</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {cold ? (
          <div className="space-y-4" data-testid="spec100-pipeline-queue-skeleton">
            <Skeleton className="h-2 w-full" />
            <div className="eq-metric-grid">
              <Skeleton className="h-16" />
              <Skeleton className="h-16" />
              <Skeleton className="h-16" />
            </div>
            <Skeleton className="h-4 w-32" />
          </div>
        ) : (
          <>
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
                aria-label="Worker utilization"
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

            <div className="eq-metric-grid text-sm">
              <div className="rounded-md border bg-muted/30 px-3 py-2.5">
                <div className="mb-1 flex items-center gap-1.5 text-xs text-muted-foreground">
                  <Zap className="h-3 w-3" />
                  <span>Throughput</span>
                </div>
                <p className="text-lg font-semibold tabular-nums">
                  {formatThroughput(metrics?.throughput_per_minute ?? 0)}
                </p>
              </div>
              <div className="rounded-md border bg-muted/30 px-3 py-2.5">
                <div className="mb-1 flex items-center gap-1.5 text-xs text-muted-foreground">
                  <Clock className="h-3 w-3" />
                  <span>Avg Wait</span>
                </div>
                <p className="text-lg font-semibold tabular-nums">
                  {formatDurationSeconds(metrics?.avg_wait_time_seconds ?? 0)}
                </p>
              </div>
              <div className="rounded-md border bg-muted/30 px-3 py-2.5">
                <div className="mb-1 flex items-center gap-1.5 text-xs text-muted-foreground">
                  <Timer className="h-3 w-3" />
                  <span>Queue ETA</span>
                </div>
                <p className="text-lg font-semibold tabular-nums">
                  {formatDurationSeconds(metrics?.estimated_queue_time_seconds ?? 0)}
                </p>
              </div>
            </div>

            <div className="flex items-center justify-between text-xs text-muted-foreground pt-2 border-t">
              <span>Queue: {pendingCount} pending</span>
              {metrics?.rate_limited && (
                <Badge variant="destructive" className="text-xs">
                  <AlertTriangle className="h-3 w-3 mr-1" />
                  Rate Limited
                </Badge>
              )}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
