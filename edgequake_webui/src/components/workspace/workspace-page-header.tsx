"use client";

import { PageHeader } from "@/components/shared/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { Workspace } from "@/types";
import { RefreshCw, Settings } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface WorkspacePageHeaderProps {
  workspace: Workspace;
  onRefresh: () => void;
  /** Opens Reconfigure Workspace wizard (LAW-101-12). */
  onEditStart: () => void;
}

export function WorkspacePageHeader({
  workspace,
  onRefresh,
  onEditStart,
}: WorkspacePageHeaderProps) {
  const { t } = useTranslation();

  return (
    <PageHeader
      className="mb-0"
      title={
        <span className="inline-flex items-center gap-2.5">
          {workspace.name}
          <Badge variant={workspace.is_active ? "default" : "secondary"} className="font-medium">
            {workspace.is_active
              ? t("common.active", "Active")
              : t("common.inactive", "Inactive")}
          </Badge>
        </span>
      }
      description={workspace.description || undefined}
      actions={
        <>
          <Button variant="outline" size="sm" onClick={onRefresh}>
            <RefreshCw className="mr-2 h-4 w-4" />
            {t("common.refresh", "Refresh")}
          </Button>
          <Button size="sm" onClick={onEditStart} data-testid="workspace-edit-config">
            <Settings className="mr-2 h-4 w-4" />
            {t("workspace.editConfig", "Edit Configuration")}
          </Button>
        </>
      }
    />
  );
}
