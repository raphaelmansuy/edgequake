"use client";

import { ProviderIcon } from "@/components/providers/provider-icon";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  EmbeddingModelSelector,
  type EmbeddingSelection,
} from "@/components/workspace/embedding-model-selector";
import {
  LLMModelSelector,
  type LLMSelection,
} from "@/components/workspace/llm-model-selector";
import type { Workspace } from "@/types";
import { AlertTriangle, Brain, Layers } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface WorkspaceModelConfigGridProps {
  workspace: Workspace;
  isEditing: boolean;
  selectedLLM: LLMSelection | undefined;
  selectedEmbedding: EmbeddingSelection | undefined;
  onLlmChange: (value: LLMSelection | undefined) => void;
  onEmbeddingChange: (value: EmbeddingSelection | undefined) => void;
  llmModelChanged: boolean;
  embeddingModelChanged: boolean;
}

function ModelDisplayRow({
  providerId,
  model,
  fullId,
  dimension,
}: {
  providerId?: string;
  model?: string;
  fullId?: string;
  dimension?: number;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
      <ProviderIcon providerId={providerId} />
      <div>
        <div className="font-medium">
          {model || t("workspace.serverDefault", "Server Default")}
        </div>
        <div className="text-sm text-muted-foreground capitalize">
          {providerId || t("workspace.autoDetect", "Auto-detected")}
          {dimension != null && (
            <span className="ml-2">• {dimension} dims</span>
          )}
        </div>
      </div>
      {fullId && (
        <Badge variant="outline" className="ml-auto">
          {fullId}
        </Badge>
      )}
    </div>
  );
}

function ChangeWarning({
  tone,
  message,
}: {
  tone: "blue" | "amber";
  message: string;
}) {
  const styles =
    tone === "blue"
      ? "bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800 text-blue-700 dark:text-blue-300"
      : "bg-amber-50 dark:bg-amber-900/20 border-amber-200 dark:border-amber-800 text-amber-700 dark:text-amber-300";
  const iconClass = tone === "blue" ? "text-blue-600" : "text-amber-600";

  return (
    <div
      className={`flex items-center gap-2 p-3 border rounded-lg ${styles}`}
    >
      <AlertTriangle className={`h-4 w-4 ${iconClass}`} />
      <span className="text-sm">{message}</span>
    </div>
  );
}

/** LLM + embedding configuration cards shared by workspace routes (SPEC-017 UI-P3-002). */
export function WorkspaceModelConfigGrid({
  workspace,
  isEditing,
  selectedLLM,
  selectedEmbedding,
  onLlmChange,
  onEmbeddingChange,
  llmModelChanged,
  embeddingModelChanged,
}: WorkspaceModelConfigGridProps) {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Brain className="h-5 w-5 text-blue-600" />
            {t("workspace.llmConfig", "LLM Configuration")}
          </CardTitle>
          <CardDescription>
            {t(
              "workspace.llmConfigDesc",
              "Model used for entity extraction and summarization during document ingestion.",
            )}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isEditing ? (
            <>
              <LLMModelSelector
                value={selectedLLM}
                onChange={onLlmChange}
                showUsageHint
              />
              {llmModelChanged && (
                <ChangeWarning
                  tone="blue"
                  message={t(
                    "workspace.llmChangeWarning",
                    "Changing LLM model requires re-extracting entities from all documents.",
                  )}
                />
              )}
            </>
          ) : (
            <ModelDisplayRow
              providerId={workspace.llm_provider}
              model={workspace.llm_model}
              fullId={workspace.llm_full_id}
            />
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Layers className="h-5 w-5 text-purple-600" />
            {t("workspace.embeddingConfig", "Embedding Configuration")}
          </CardTitle>
          <CardDescription>
            {t(
              "workspace.embeddingConfigDesc",
              "Model used for vector embeddings of document chunks.",
            )}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isEditing ? (
            <>
              <EmbeddingModelSelector
                value={selectedEmbedding}
                onChange={onEmbeddingChange}
              />
              {embeddingModelChanged && (
                <ChangeWarning
                  tone="amber"
                  message={t(
                    "workspace.embeddingChangeWarning",
                    "Changing embedding model requires rebuilding all document embeddings.",
                  )}
                />
              )}
            </>
          ) : (
            <ModelDisplayRow
              providerId={workspace.embedding_provider}
              model={workspace.embedding_model}
              fullId={workspace.embedding_full_id}
              dimension={workspace.embedding_dimension}
            />
          )}
        </CardContent>
      </Card>
    </div>
  );
}
