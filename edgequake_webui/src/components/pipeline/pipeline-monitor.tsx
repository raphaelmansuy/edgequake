/**
 * @module PipelineMonitor
 * @description Comprehensive pipeline monitoring component with real-time updates.
 *
 * @implements FEAT0004 - Processing status tracking
 * @implements UC0007 - User monitors document processing progress
 * @implements OODA-11 - Stage progress visibility
 * @implements OODA-37 - Workspace isolation in pipeline monitor
 */
"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { PageHeader } from "@/components/shared/page-header";
import { PipelineActivityLogCard } from "@/components/pipeline/pipeline-activity-log-card";
import { PipelineChunkProgressCard } from "@/components/pipeline/pipeline-chunk-progress-card";
import { PipelineProcessingDocumentsCard } from "@/components/pipeline/pipeline-processing-documents-card";
import { PipelineQueueMetricsCard } from "@/components/pipeline/pipeline-queue-metrics-card";
import { PipelineStagesCard } from "@/components/pipeline/pipeline-stages-card";
import { PipelineTaskQueueCard } from "@/components/pipeline/pipeline-task-queue-card";
import {
  PipelineWorkspaceContext,
  scopedQueryKey,
} from "@/lib/pipeline/pipeline-workspace-context";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Building2, ChevronDown, RefreshCw } from "lucide-react";
import Link from "next/link";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

export function PipelineMonitor() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { selectedTenantId, selectedWorkspaceId, workspaces } = useTenantStore();

  const currentWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);
  const workspaceName = currentWorkspace?.name || t("pipeline.allWorkspaces", "All Workspaces");

  const workspaceContext = {
    selectedTenantId,
    selectedWorkspaceId,
    workspaceName,
  };

  const handleRefresh = () => {
    for (const base of [
      "enhanced-pipeline-status",
      "documents",
      "tasks",
      "queue-metrics",
    ]) {
      queryClient.invalidateQueries({
        queryKey: scopedQueryKey(base, selectedTenantId, selectedWorkspaceId),
      });
    }
    toast.success(t("common.refresh", "Refreshed"));
  };

  return (
    <PipelineWorkspaceContext.Provider value={workspaceContext}>
      <div
        className="flex h-full min-h-0 flex-col overflow-clip"
        data-testid="spec100-pipeline-shell"
      >
        <div className="flex-shrink-0 sticky top-0 z-10 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b">
          <div className="container mx-auto px-page py-3 max-w-7xl">
            <PageHeader
              className="mb-0"
              title={t("pipeline.title", "Pipeline Monitor")}
              description={
                <span className="inline-flex items-center gap-2">
                  <Building2 className="h-4 w-4" />
                  <span>{workspaceName}</span>
                  {!selectedWorkspaceId && (
                    <Badge variant="destructive" className="text-xs">
                      {t("pipeline.noWorkspace", "No workspace selected")}
                    </Badge>
                  )}
                </span>
              }
              actions={
                <>
                  <Link href="/documents">
                    <Button variant="ghost" size="sm">
                      <ArrowLeft className="h-4 w-4 mr-2" />
                      {t("pipeline.backToDocuments", "Back to Documents")}
                    </Button>
                  </Link>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleRefresh}
                    data-testid="pipeline-refresh-button"
                  >
                    <RefreshCw className="h-4 w-4 mr-2" />
                    {t("common.refresh", "Refresh")}
                  </Button>
                </>
              }
            />
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto">
          <div
            className="container mx-auto p-page max-w-7xl"
            data-testid="spec100-pipeline-main"
          >
            <PipelineStagesCard />

            <div className="mt-page">
              <PipelineProcessingDocumentsCard />
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-page mt-page">
              <PipelineQueueMetricsCard />
              <PipelineActivityLogCard />
            </div>

            <div className="mt-page">
              <PipelineChunkProgressCard />
            </div>

            <details className="mt-page mb-page group">
              <summary className="cursor-pointer list-none flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors">
                <ChevronDown className="h-4 w-4 transition-transform group-open:rotate-180" />
                <span>{t("pipeline.advancedDetails", "Advanced Details")}</span>
              </summary>
              <div className="mt-page">
                <PipelineTaskQueueCard />
              </div>
            </details>
          </div>
        </div>
      </div>
    </PipelineWorkspaceContext.Provider>
  );
}
