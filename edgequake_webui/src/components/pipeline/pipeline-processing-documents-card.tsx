"use client";

import {
  getDocumentDisplayStatus,
  isProcessingStatus,
  normalizeStatus,
  StatusBadge,
} from "@/components/documents/status-badge";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { getDocuments } from "@/lib/api/edgequake";
import {
  scopedQueryKey,
  usePipelineWorkspace,
} from "@/lib/pipeline/pipeline-workspace-context";
import { useQuery } from "@tanstack/react-query";
import { FileText, Loader2 } from "lucide-react";

export function PipelineProcessingDocumentsCard() {
  const { selectedTenantId, selectedWorkspaceId } = usePipelineWorkspace();

  const { data: documents, isLoading } = useQuery({
    queryKey: scopedQueryKey("documents", selectedTenantId, selectedWorkspaceId),
    queryFn: () => getDocuments({ page: 1, page_size: 50 }),
    refetchInterval: 2000,
    select: (data) =>
      data.items.filter((doc) =>
        isProcessingStatus(normalizeStatus(doc.status)),
      ),
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
                        {doc.content_length
                          ? `${(doc.content_length / 1024).toFixed(1)} KB`
                          : "Unknown size"}
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
