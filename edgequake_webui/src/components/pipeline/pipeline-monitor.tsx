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

export function PipelineMonitor() {
  const queryClient = useQueryClient();
  const { selectedTenantId, selectedWorkspaceId, workspaces } = useTenantStore();

  const currentWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);
  const workspaceName = currentWorkspace?.name || "All Workspaces";

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
    toast.success("Refreshed");
  };

  return (
    <PipelineWorkspaceContext.Provider value={workspaceContext}>
      <div className="flex flex-col h-[calc(100vh-theme(spacing.20))]">
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
              <Button variant="outline" size="sm" onClick={handleRefresh}>
                <RefreshCw className="h-4 w-4 mr-2" />
                Refresh
              </Button>
            </div>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto">
          <div className="container mx-auto p-4 sm:p-6 max-w-7xl pb-8">
            <PipelineStagesCard />

            <div className="mt-4 sm:mt-6">
              <PipelineChunkProgressCard />
            </div>

            <div className="mt-4 sm:mt-6">
              <PipelineProcessingDocumentsCard />
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 sm:gap-6 mt-4 sm:mt-6">
              <PipelineQueueMetricsCard />
              <PipelineActivityLogCard />
            </div>

            <details className="mt-4 sm:mt-6 mb-4 group">
              <summary className="cursor-pointer list-none flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors">
                <ChevronDown className="h-4 w-4 transition-transform group-open:rotate-180" />
                <span>Advanced Details</span>
              </summary>
              <div className="mt-4">
                <PipelineTaskQueueCard />
              </div>
            </details>
          </div>
        </div>
      </div>
    </PipelineWorkspaceContext.Provider>
  );
}
