"use client";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useCurrentTime } from "@/hooks/use-current-time";
import { getTasksList } from "@/lib/api/edgequake";
import {
  formatTaskType,
  formatWaitTimeMs,
  partitionTasksByStatus,
} from "@/lib/pipeline/pipeline-formatters";
import {
  scopedQueryKey,
  usePipelineWorkspace,
} from "@/lib/pipeline/pipeline-workspace-context";
import type { TaskResponse } from "@/types";
import { useQuery } from "@tanstack/react-query";
import { Clock, Loader2, Timer } from "lucide-react";
import { useMemo } from "react";

export function PipelineTaskQueueCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();
  const now = useCurrentTime(1000);

  const { data: tasks, isLoading } = useQuery({
    queryKey: scopedQueryKey("tasks", selectedTenantId, selectedWorkspaceId),
    queryFn: () => getTasksList({ page_size: 50 }),
    refetchInterval: 3000,
  });

  const { pendingTasks, processingTasks } = useMemo(
    () => partitionTasksByStatus(tasks?.tasks ?? []),
    [tasks],
  );

  const formatWaitTime = (createdAt: string) =>
    formatWaitTimeMs(now - new Date(createdAt).getTime());

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
                          <span className="font-bold text-yellow-600 w-4">
                            #{index + 1}
                          </span>
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
                          <span>
                            Started{" "}
                            {formatWaitTime(task.started_at || task.created_at)} ago
                          </span>
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
