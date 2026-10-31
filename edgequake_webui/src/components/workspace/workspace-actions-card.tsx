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
import { capabilitiesOf } from "@/constants/rag-profile";
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
  const graph = capabilitiesOf(workspace);

  const messageKey =
    pendingRebuild && hasPendingRebuild(pendingRebuild)
      ? getPendingRebuildMessageKey(pendingRebuild, {
          includeVision: includeVisionPending,
        })
      : null;
  const showRebuildBanner = Boolean(messageKey && pendingRebuild);

  return (
    <Card className="gap-3 py-4" data-testid="workspace-actions-card">
      <CardHeader className="px-4 pb-0 gap-1">
        <CardTitle className="flex items-center gap-2 text-base">
          <Settings className="h-4 w-4" />
          {t("workspace.actions", "Workspace Actions")}
        </CardTitle>
        <CardDescription className="text-xs">
          {t(
            "workspace.actionsDesc",
            "Manage workspace data and re-process documents.",
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3 px-4">
        {/*
          SPEC-100: slot always mounted. Empty state collapses (no 5.5rem blank).
          Banner appears after user Apply (interaction window) — avoid permanent dead air.
        */}
        <div
          className="min-h-0"
          data-testid="spec100-workspace-rebuild-slot"
          data-reserved={showRebuildBanner ? "banner" : "collapsed"}
        >
          {showRebuildBanner && pendingRebuild && messageKey ? (
            <div className="flex items-start gap-2.5 rounded-lg border border-amber-200 bg-amber-50 p-3 dark:border-amber-800 dark:bg-amber-900/20">
              <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-600" />
              <div className="min-w-0 flex-1 space-y-0.5">
                <p className="text-sm font-medium text-amber-800 dark:text-amber-200">
                  {t("workspace.rebuildPending", "Rebuild Required")}
                </p>
                <p className="text-xs text-amber-700 dark:text-amber-300">
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
          ) : null}
        </div>

        <div className="grid grid-cols-1 gap-3 md:grid-cols-2 md:items-stretch">
          <RebuildEmbeddingsButton
            variant="card"
            onComplete={onRebuildComplete}
          />
          {graph.extracts_graph ? (
            <RebuildKnowledgeGraphButton
              variant="card"
              rebuildEmbeddings={true}
              onComplete={onRebuildComplete}
            />
          ) : null}
        </div>

        <dl
          className="eq-metric-grid gap-x-4 gap-y-1.5 rounded-lg border border-dashed bg-muted/20 px-3 py-2.5 text-xs"
          data-testid="workspace-metadata"
        >
          <div className="min-w-0 space-y-0.5">
            <dt className="text-muted-foreground">
              {t("workspace.id", "Workspace ID")}
            </dt>
            <dd>
              <code className="block truncate font-mono text-xs" title={workspace.id}>
                {workspace.id}
              </code>
            </dd>
          </div>
          <div className="min-w-0 space-y-0.5">
            <dt className="text-muted-foreground">
              {t("workspace.slug", "Slug")}
            </dt>
            <dd>
              <code className="font-mono text-xs">
                {workspace.slug || "-"}
              </code>
            </dd>
          </div>
          <div className="min-w-0 space-y-0.5">
            <dt className="text-muted-foreground">
              {t("workspace.created", "Created")}
            </dt>
            <dd>{new Date(workspace.created_at).toLocaleDateString()}</dd>
          </div>
          <div className="min-w-0 space-y-0.5">
            <dt className="text-muted-foreground">
              {t("workspace.updated", "Updated")}
            </dt>
            <dd>
              {workspace.updated_at
                ? new Date(workspace.updated_at).toLocaleDateString()
                : "-"}
            </dd>
          </div>
        </dl>
      </CardContent>
    </Card>
  );
}
