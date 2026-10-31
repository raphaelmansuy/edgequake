"use client";

import { capabilitiesOf } from "@/constants/rag-profile";
import { useLlmCapabilityModels } from "@/hooks/use-llm-capability-models";
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
import { ReasoningEffortSelect } from "@/components/settings/reasoning-effort-select";
import { useInheritedModelDefaults } from "@/hooks/use-inherited-model-defaults";
import {
  effectiveEmbeddingFromWorkspace,
  effectiveLlmFromWorkspace,
} from "@/lib/config/resolve-model-choice";
import {
  effectiveEffortWhenAuto,
  formatEffectiveBestPracticeHint,
  modelSupportsThinking,
  supportedReasoningEffortsForModel,
} from "@/lib/settings/reasoning-effort-supported";
import { useTenantStore } from "@/stores/use-tenant-store";
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
  /** SPEC-109: extract role effort (edit draft). */
  extractReasoningEffort?: string;
  queryReasoningEffort?: string;
  onExtractReasoningChange?: (value: string | undefined) => void;
  onQueryReasoningChange?: (value: string | undefined) => void;
}

function ModelDisplayRow({
  providerId,
  model,
  fullId,
  dimension,
  /** When model unset — never-silent resolved default e.g. ollama/gemma4:latest */
  resolvedDefaultId,
  /** SPEC-123: resolved provider/model + provenance for honest UI */
  resolvesToLabel,
  resolutionSource,
  hasWorkspaceOverride,
  testId,
}: {
  providerId?: string;
  model?: string;
  fullId?: string;
  dimension?: number;
  resolvedDefaultId?: string;
  resolvesToLabel?: string;
  resolutionSource?: string;
  hasWorkspaceOverride?: boolean;
  testId?: string;
}) {
  const { t } = useTranslation();
  const usingDefault = !hasWorkspaceOverride && !model;
  const title = usingDefault
    ? t("workspace.serverDefaultWithValue", "Server Default ({{value}})", {
        value:
          resolvesToLabel ||
          resolvedDefaultId ||
          t("workspace.notConfigured", "not configured"),
      })
    : model || resolvesToLabel;

  const subtitle = hasWorkspaceOverride
    ? t("workspace.workspaceOverride", "Workspace override")
    : resolutionSource
      ? t("workspace.resolvesVia", "Resolves via {{source}}", {
          source: resolutionSource,
        })
      : providerId ||
        (usingDefault
          ? t("workspace.inheritedDefault", "Inherited default")
          : t("workspace.autoDetect", "Auto-detected"));

  const badgeValue = resolvesToLabel || fullId || (usingDefault ? resolvedDefaultId : undefined);

  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 p-3 bg-muted/50 rounded-lg">
      <ProviderIcon
        providerId={
          providerId ||
          (usingDefault
            ? resolvesToLabel?.split("/")[0] || resolvedDefaultId?.split("/")[0]
            : undefined)
        }
      />
      <div className="min-w-0 flex-1 basis-40">
        <div className="font-medium truncate" title={typeof title === "string" ? title : undefined}>{title}</div>
        <div className="text-sm text-muted-foreground capitalize truncate">
          {subtitle}
          {dimension != null && (
            <span className="ml-2">• {dimension} dims</span>
          )}
        </div>
      </div>
      {badgeValue && (
        <Badge
          variant="outline"
          className="ml-auto min-w-0 max-w-full shrink font-mono text-xs"
          title={resolutionSource ? `source=${resolutionSource}` : badgeValue}
          data-testid={testId}
        >
          <span className="truncate">
            {t("settings.pdfParser.resolvesTo", "Resolves to {{value}}", {
              value: badgeValue,
            })}
          </span>
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
  extractReasoningEffort,
  queryReasoningEffort,
  onExtractReasoningChange,
  onQueryReasoningChange,
}: WorkspaceModelConfigGridProps) {
  const { t } = useTranslation();
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const inherited = useInheritedModelDefaults(tenantId);
  const llmModels = useLlmCapabilityModels();
  const llmDefaultId =
    inherited.defaultLlmProvider && inherited.defaultLlmModel
      ? `${inherited.defaultLlmProvider}/${inherited.defaultLlmModel}`
      : undefined;
  const embeddingDefaultId =
    inherited.defaultEmbeddingProvider && inherited.defaultEmbeddingModel
      ? `${inherited.defaultEmbeddingProvider}/${inherited.defaultEmbeddingModel}`
      : undefined;
  const provider = selectedLLM?.provider ?? workspace.llm_provider;
  const model = selectedLLM?.model ?? workspace.llm_model;
  const supported = supportedReasoningEffortsForModel(
    llmModels,
    provider,
    model,
  );
  const thinkingSupported = modelSupportsThinking(
    llmModels,
    provider,
    model,
  );
  const extractEffectiveAuto = effectiveEffortWhenAuto(
    llmModels,
    provider,
    model,
    "structured",
  );
  const queryEffectiveAuto = effectiveEffortWhenAuto(
    llmModels,
    provider,
    model,
    "query",
  );
  const extractDisplay =
    extractReasoningEffort ??
    workspace.llm_roles?.extract?.reasoning_effort ??
    workspace.default_reasoning_effort ??
    undefined;
  const queryDisplay =
    queryReasoningEffort ??
    workspace.llm_roles?.query?.reasoning_effort ??
    workspace.default_reasoning_effort ??
    undefined;

  const resolvedLlm = effectiveLlmFromWorkspace(workspace);
  const resolvedEmbedding = effectiveEmbeddingFromWorkspace(workspace);
  // SPEC-180: without a graph the LLM only answers; it extracts nothing at ingest.
  const showsGraph = capabilitiesOf(workspace).shows_graph_ui;
  const llmResolvesTo = `${resolvedLlm.provider}/${resolvedLlm.model}`;
  const embeddingResolvesTo = `${resolvedEmbedding.provider}/${resolvedEmbedding.model}`;
  // Prefer API provenance (LAW-123-8); fall back to source from FE mirror.
  const hasLlmOverride = resolvedLlm.source === "workspace";
  const hasEmbeddingOverride = resolvedEmbedding.source === "workspace";
  const currentDimension = resolvedEmbedding.dimension;
  const nextDimension = selectedEmbedding?.dimension;
  const embeddingChangeMessage =
    currentDimension && nextDimension && currentDimension !== nextDimension
      ? t(
          "workspace.embeddingDimensionChangeWarning",
          "Saving changes the embedding dimension from {{from}} to {{to}}. Existing vectors must be rebuilt.",
          { from: currentDimension, to: nextDimension },
        )
      : t(
          "workspace.embeddingChangeWarning",
          "Saving changes the embedding model. Existing vectors must be rebuilt to match it.",
        );

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Brain className="h-5 w-5 text-blue-600" />
            {t("workspace.llmConfig", "LLM Configuration")}
          </CardTitle>
          <CardDescription>
            {showsGraph
              ? t(
                  "workspace.llmConfigDesc",
                  "Model used for entity extraction and summarization during document ingestion.",
                )
              : t(
                  "workspace.llmConfigDescNormal",
                  "Model that writes the answers. Nothing is extracted when documents are added.",
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
              {llmModelChanged && showsGraph && (
                <ChangeWarning
                  tone="blue"
                  message={t(
                    "workspace.llmChangeWarning",
                    "Changing LLM model requires re-extracting entities from all documents.",
                  )}
                />
              )}
              <div
                className="space-y-3 pt-2 border-t"
                data-testid="workspace-role-reasoning"
              >
                {showsGraph ? (
                <ReasoningEffortSelect
                  id="workspace-extract-effort"
                  data-testid="workspace-extract-reasoning"
                  label={t(
                    "workspace.extractReasoning",
                    "Extract reasoning effort",
                  )}
                  value={extractDisplay}
                  supported={supported}
                  thinkingSupported={thinkingSupported}
                  effectiveWhenAuto={extractEffectiveAuto}
                  onChange={(v) => onExtractReasoningChange?.(v)}
                />
                ) : null}
                <ReasoningEffortSelect
                  id="workspace-query-effort"
                  data-testid="workspace-query-reasoning"
                  label={t(
                    "workspace.queryReasoning",
                    "Query reasoning effort",
                  )}
                  value={queryDisplay}
                  supported={supported}
                  thinkingSupported={thinkingSupported}
                  effectiveWhenAuto={queryEffectiveAuto}
                  onChange={(v) => onQueryReasoningChange?.(v)}
                />
              </div>
            </>
          ) : (
            <ModelDisplayRow
              providerId={workspace.llm_provider || resolvedLlm.provider}
              model={
                hasLlmOverride ? workspace.llm_model : undefined
              }
              fullId={workspace.llm_full_id}
              resolvedDefaultId={llmDefaultId}
              resolvesToLabel={llmResolvesTo}
              resolutionSource={resolvedLlm.source}
              hasWorkspaceOverride={hasLlmOverride}
              testId="llm-resolves-to"
            />
          )}
          {!isEditing && (
            <div
              className="text-xs text-muted-foreground space-y-1"
              data-testid="workspace-role-reasoning-readonly"
            >
              {showsGraph ? (
                <>
              <div>
                Extract effort:{" "}
                {extractDisplay?.trim()
                  ? extractDisplay
                  : `Auto → ${extractEffectiveAuto}`}
              </div>
              <div
                className="text-xs"
                data-testid="workspace-extract-effective-hint"
              >
                {formatEffectiveBestPracticeHint(extractEffectiveAuto)}
              </div>
                </>
              ) : null}
              <div>
                Query effort:{" "}
                {queryDisplay?.trim()
                  ? queryDisplay
                  : `Auto → ${queryEffectiveAuto}`}
              </div>
              <div
                className="text-xs"
                data-testid="workspace-query-effective-hint"
              >
                {formatEffectiveBestPracticeHint(queryEffectiveAuto)}
              </div>
            </div>
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
                <ChangeWarning tone="amber" message={embeddingChangeMessage} />
              )}
            </>
          ) : (
            <ModelDisplayRow
              providerId={
                workspace.embedding_provider || resolvedEmbedding.provider
              }
              model={
                hasEmbeddingOverride ? workspace.embedding_model : undefined
              }
              fullId={workspace.embedding_full_id}
              dimension={
                resolvedEmbedding.dimension || workspace.embedding_dimension
              }
              resolvedDefaultId={embeddingDefaultId}
              resolvesToLabel={embeddingResolvesTo}
              resolutionSource={resolvedEmbedding.source}
              hasWorkspaceOverride={hasEmbeddingOverride}
              testId="embedding-resolves-to"
            />
          )}
        </CardContent>
      </Card>
    </div>
  );
}
