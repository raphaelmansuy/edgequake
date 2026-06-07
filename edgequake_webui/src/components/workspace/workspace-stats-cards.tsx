"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import type { WorkspaceStats } from "@/lib/api/edgequake/workspaces";
import { resolveWorkspaceStatCounts } from "@/lib/workspace/stats-display";
import type { Workspace } from "@/types";
import { Database, FileText, GitBranch, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface WorkspaceStatsCardsProps {
  workspace: Workspace;
  stats?: WorkspaceStats;
  isLoadingStats: boolean;
}

export function WorkspaceStatsCards({
  workspace,
  stats,
  isLoadingStats,
}: WorkspaceStatsCardsProps) {
  const { t } = useTranslation();
  const counts = resolveWorkspaceStatCounts(stats, workspace);

  const items = [
    {
      key: "documents",
      icon: FileText,
      label: t("workspace.documents", "Documents"),
      value: counts.documents,
      footer:
        workspace.max_documents != null ? (
          <p className="text-xs text-muted-foreground mt-1">
            {t("workspace.maxDocuments", "Max")}:{" "}
            {workspace.max_documents.toLocaleString()}
          </p>
        ) : null,
    },
    {
      key: "entities",
      icon: GitBranch,
      label: t("workspace.entities", "Entities"),
      value: counts.entities,
      footer: null,
    },
    {
      key: "relationships",
      icon: Layers,
      label: t("workspace.relationships", "Relationships"),
      value: counts.relationships,
      footer: null,
    },
    {
      key: "chunks",
      icon: Database,
      label: t("workspace.chunks", "Chunks"),
      value: counts.chunks,
      footer: null,
    },
  ] as const;

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {items.map(({ key, icon: Icon, label, value, footer }) => (
        <Card key={key}>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <Icon className="h-4 w-4" />
              {label}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {isLoadingStats ? (
                <Skeleton className="h-8 w-16" />
              ) : (
                value
              )}
            </div>
            {footer}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
