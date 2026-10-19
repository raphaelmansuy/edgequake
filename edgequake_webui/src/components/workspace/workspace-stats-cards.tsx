"use client";

import { StatStrip } from "@/components/shared/stat-strip";
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
          <p className="mt-1.5 text-xs text-muted-foreground">
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
    <StatStrip>
      {items.map(({ key, icon: Icon, label, value, footer }) => (
        <div key={key} className="flex min-w-0 flex-col bg-card px-4 py-3.5">
          <p className="flex items-center gap-2 text-[0.8125rem] font-medium text-muted-foreground">
            <Icon className="h-4 w-4 shrink-0" aria-hidden="true" />
            <span className="truncate">{label}</span>
          </p>
          <div className="mt-2 text-[1.75rem] font-semibold leading-none tracking-tight tabular-nums">
            {isLoadingStats ? <Skeleton className="h-7 w-16" /> : value}
          </div>
          {footer}
        </div>
      ))}
    </StatStrip>
  );
}
