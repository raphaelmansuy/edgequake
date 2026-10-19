"use client";

import { StatStrip } from "@/components/shared/stat-strip";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import {
  getEnhancedPipelineStatus,
  requestPipelineCancellation,
} from "@/lib/api/edgequake";
import { usePipelineDocuments } from "@/hooks/use-pipeline-documents";
import { PIPELINE_PHASES } from "@/lib/pipeline/pipeline-phases";
import {
  scopedQueryKey,
  usePipelineWorkspace,
} from "@/lib/pipeline/pipeline-workspace-context";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle, Clock, Layers, Loader2, StopCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

export function PipelineStagesCard() {
  const { t } = useTranslation();
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();
  const queryClient = useQueryClient();

  const { data: documents } = usePipelineDocuments({
    refetchInterval: 3000,
  });

  const { data: status } = useQuery({
    queryKey: scopedQueryKey(
      "enhanced-pipeline-status",
      selectedTenantId,
      selectedWorkspaceId,
    ),
    queryFn: () =>
      getEnhancedPipelineStatus(
        selectedTenantId ?? undefined,
        selectedWorkspaceId ?? undefined,
      ),
    refetchInterval: 2000,
  });

  const cancelMutation = useMutation({
    mutationFn: requestPipelineCancellation,
    onSuccess: () => {
      toast.success("Pipeline cancellation requested");
      queryClient.invalidateQueries({
        queryKey: scopedQueryKey(
          "enhanced-pipeline-status",
          selectedTenantId,
          selectedWorkspaceId,
        ),
      });
    },
    onError: (error) => {
      toast.error(
        `Cancel failed: ${error instanceof Error ? error.message : "Unknown"}`,
      );
    },
  });

  const phaseCounts = documents?.status_counts ?? {
    pending: 0,
    processing: 0,
    completed: 0,
    partial_failure: 0,
    failed: 0,
    cancelled: 0,
  };
  const totalDocs = documents?.total ?? 0;
  const isActive = phaseCounts.processing > 0 || phaseCounts.pending > 0;

  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="eq-toolbar justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Layers className="h-4 w-4 text-muted-foreground" />
              Document Pipeline
            </CardTitle>
            <CardDescription>
              {t("pipeline.documentsInWorkspace", {
                count: totalDocs,
                defaultValue_one: "{{count}} document in workspace",
                defaultValue_other: "{{count}} documents in workspace",
              })}
            </CardDescription>
          </div>
          {status?.is_busy && (
            <Button
              variant="outline"
              size="sm"
              className="border-destructive/40 text-destructive hover:bg-destructive/10 hover:text-destructive"
              onClick={() => cancelMutation.mutate()}
              disabled={
                cancelMutation.isPending || status.cancellation_requested
              }
            >
              {cancelMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <StopCircle className="mr-2 h-4 w-4" />
              )}
              {status.cancellation_requested ? "Cancelling..." : "Cancel"}
            </Button>
          )}
          {!status?.is_busy && isActive && (
            <Badge
              variant="outline"
              className="text-yellow-700 dark:text-yellow-400 border-yellow-600"
            >
              <Clock className="h-3 w-3 mr-1" />
              Queued
            </Badge>
          )}
          {!status?.is_busy && !isActive && totalDocs > 0 && (
            <Badge
              variant="outline"
              className="text-green-700 border-green-600 dark:text-green-400 dark:border-green-500"
            >
              <CheckCircle className="h-3 w-3 mr-1" />
              Idle
            </Badge>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <StatStrip aria-label={t("pipeline.phases", "Pipeline phases")}>
          {PIPELINE_PHASES.map((phase) => {
            const count =
              phaseCounts[phase.key as keyof typeof phaseCounts] || 0;
            const Icon = phase.icon;
            const isActivePhase = count > 0;

            return (
              <div
                key={phase.key}
                data-phase={phase.key}
                className="flex min-w-0 flex-col bg-card px-4 py-3.5"
              >
                <span
                  className={`flex items-center gap-2 text-[0.8125rem] font-medium ${
                    isActivePhase ? phase.color : "text-muted-foreground"
                  }`}
                >
                  <Icon
                    aria-hidden="true"
                    className={`h-4 w-4 shrink-0 ${
                      phase.key === "processing" && isActivePhase
                        ? "motion-safe:animate-pulse"
                        : ""
                    }`}
                  />
                  <span className="truncate">{phase.label}</span>
                </span>
                <span
                  className={`mt-2 text-[1.75rem] font-semibold leading-none tracking-tight tabular-nums ${
                    isActivePhase ? "text-foreground" : "text-muted-foreground/60"
                  }`}
                >
                  {count}
                </span>
              </div>
            );
          })}
        </StatStrip>

        {totalDocs > 0 && (
          <div className="mt-4">
            <div className="flex justify-between text-xs text-muted-foreground mb-1">
              <span>Pipeline Progress</span>
              <span>
                {phaseCounts.completed} / {totalDocs} completed
              </span>
            </div>
            <Progress
              aria-label="Pipeline progress"
              value={
                totalDocs > 0 ? (phaseCounts.completed / totalDocs) * 100 : 0
              }
              className="h-2"
            />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
