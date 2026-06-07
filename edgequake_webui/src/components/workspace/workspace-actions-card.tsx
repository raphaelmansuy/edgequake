"use client";

import { RebuildEmbeddingsButton } from "@/components/workspace/rebuild-embeddings-button";
import { RebuildKnowledgeGraphButton } from "@/components/workspace/rebuild-knowledge-graph-button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  getPendingRebuildDefaultMessage,
  getPendingRebuildMessageKey,
  hasPendingRebuild,
  type WorkspacePendingRebuild,
} from "@/lib/workspace/pending-rebuild-messages";
import type { Workspace } from "@/types";
import { AlertTriangle, Settings } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface WorkspaceActionsCardProps {
  workspace: Workspace;
  pendingRebuild: WorkspacePendingRebuild | null;
  includeVisionPending?: boolean;
  onRebuildComplete: () => void;
}

export function WorkspaceActionsCard({
  workspace,
  pendingRebuild,
  includeVisionPending = false,
  onRebuildComplete,
}: WorkspaceActionsCardProps) {
  const { t } = useTranslation();

  const messageKey =
    pendingRebuild && hasPendingRebuild(pendingRebuild)
      ? getPendingRebuildMessageKey(pendingRebuild, {
          includeVision: includeVisionPending,
        })
      : null;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Settings className="h-5 w-5" />
          {t("workspace.actions", "Workspace Actions")}
        </CardTitle>
        <CardDescription>
          {t(
            "workspace.actionsDesc",
            "Manage workspace data and re-process documents.",
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {messageKey && pendingRebuild && (
          <div className="flex items-start gap-3 p-4 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
            <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-amber-600" />
            <div className="flex-1">
              <p className="font-medium text-amber-800 dark:text-amber-200">
                {t("workspace.rebuildPending", "Rebuild Required")}
              </p>
              <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
                {t(
                  messageKey,
                  getPendingRebuildDefaultMessage(
                    messageKey,
                    includeVisionPending,
                  ),
                )}
              </p>
            </div>
          </div>
        )}

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <RebuildEmbeddingsButton
            variant="card"
            onComplete={onRebuildComplete}
          />
          <RebuildKnowledgeGraphButton
            variant="card"
            rebuildEmbeddings={true}
            onComplete={onRebuildComplete}
          />
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
          <Card className="border-dashed">
            <CardContent className="pt-6">
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    {t("workspace.id", "Workspace ID")}
                  </span>
                  <code className="max-w-[60%] break-all rounded bg-muted px-2 py-1 text-right text-xs">
                    {workspace.id}
                  </code>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    {t("workspace.slug", "Slug")}
                  </span>
                  <code className="max-w-[60%] break-all rounded bg-muted px-2 py-1 text-right text-xs">
                    {workspace.slug || "-"}
                  </code>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    {t("workspace.created", "Created")}
                  </span>
                  <span className="text-sm">
                    {new Date(workspace.created_at).toLocaleDateString()}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    {t("workspace.updated", "Updated")}
                  </span>
                  <span className="text-sm">
                    {workspace.updated_at
                      ? new Date(workspace.updated_at).toLocaleDateString()
                      : "-"}
                  </span>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </CardContent>
    </Card>
  );
}
