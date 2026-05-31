"use client";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { getDocuments, getEnhancedPipelineStatus } from "@/lib/api/edgequake";
import { buildDocumentNameMap } from "@/lib/pipeline/pipeline-formatters";
import {
  scopedQueryKey,
  usePipelineWorkspace,
} from "@/lib/pipeline/pipeline-workspace-context";
import { useQuery } from "@tanstack/react-query";
import { Activity } from "lucide-react";
import { useMemo } from "react";
import { PipelineMessageItem } from "./pipeline-message-item";

export function PipelineActivityLogCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();

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

  const { data: documentsData } = useQuery({
    queryKey: scopedQueryKey("documents", selectedTenantId, selectedWorkspaceId),
    queryFn: () => getDocuments({ page: 1, page_size: 100 }),
    refetchInterval: 5000,
  });

  const documentMap = useMemo(
    () => buildDocumentNameMap(documentsData?.items ?? []),
    [documentsData],
  );

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
                <PipelineMessageItem
                  key={idx}
                  message={msg}
                  documentMap={documentMap}
                />
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
