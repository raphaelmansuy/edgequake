"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { Workspace } from "@/types";
import { FolderKanban, RefreshCw, Save, Settings } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface WorkspacePageHeaderProps {
  workspace: Workspace;
  isEditing: boolean;
  isSaving: boolean;
  onRefresh: () => void;
  onEditStart: () => void;
  onCancel: () => void;
  onSave: () => void;
}

export function WorkspacePageHeader({
  workspace,
  isEditing,
  isSaving,
  onRefresh,
  onEditStart,
  onCancel,
  onSave,
}: WorkspacePageHeaderProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
      <div className="space-y-1">
        <div className="flex items-center gap-3">
          <FolderKanban className="h-8 w-8 text-primary" />
          <h1 className="text-2xl font-bold">{workspace.name}</h1>
          <Badge variant={workspace.is_active ? "default" : "secondary"}>
            {workspace.is_active
              ? t("common.active", "Active")
              : t("common.inactive", "Inactive")}
          </Badge>
        </div>
        {workspace.description && (
          <p className="text-muted-foreground">{workspace.description}</p>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-2 self-start lg:self-auto">
        <Button variant="outline" size="sm" onClick={onRefresh}>
          <RefreshCw className="h-4 w-4 mr-2" />
          {t("common.refresh", "Refresh")}
        </Button>
        {!isEditing ? (
          <Button variant="default" size="sm" onClick={onEditStart}>
            <Settings className="h-4 w-4 mr-2" />
            {t("workspace.editConfig", "Edit Configuration")}
          </Button>
        ) : (
          <>
            <Button variant="outline" size="sm" onClick={onCancel}>
              {t("common.cancel", "Cancel")}
            </Button>
            <Button
              variant="default"
              size="sm"
              onClick={onSave}
              disabled={isSaving}
            >
              <Save className="h-4 w-4 mr-2" />
              {t("common.save", "Save")}
            </Button>
          </>
        )}
      </div>
    </div>
  );
}
