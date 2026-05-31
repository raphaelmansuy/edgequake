/**
 * @module WorkspacePage (Deeplink)
 * @description Workspace configuration page accessible via /w/[slug]/workspace deeplink.
 *
 * @implements SPEC-032: Workspace configuration via deeplink
 * @implements FEAT0802: Workspace detail view with LLM/embedding configuration (deeplink route)
 * @implements UC0305: User views workspace configuration
 *
 * @enforces BR0305: Workspace config is visible and editable
 * @enforces BR0306: Rebuild action available when model changes
 */
'use client';

import { ProviderIcon } from '@/components/providers/provider-icon';
import { useParams } from 'next/navigation';

import {
  WorkspaceLoading,
  WorkspaceNotFound,
} from '@/components/workspace/workspace-deeplink-states';
import { WorkspaceEntityTypesCard } from '@/components/workspace/workspace-entity-types-card';
import { WorkspacePageHeader } from '@/components/workspace/workspace-page-header';
import { WorkspaceProviderHealthCard } from '@/components/workspace/workspace-provider-health-card';
import { WorkspaceStatsCards } from '@/components/workspace/workspace-stats-cards';
import { ENTITY_PRESETS } from '@/constants/entity-presets';
import { useWorkspaceDetailQueries } from '@/hooks/use-workspace-detail-queries';
import { useWorkspaceSlugResolver } from '@/hooks/use-workspace-slug-resolver';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { EmbeddingModelSelector, type EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import { RebuildEmbeddingsButton } from '@/components/workspace/rebuild-embeddings-button';
import { RebuildKnowledgeGraphButton } from '@/components/workspace/rebuild-knowledge-graph-button';
import { updateWorkspace } from '@/lib/api/edgequake';
import {
  getWorkspaceEmbeddingSelection,
  getWorkspaceLlmSelection,
} from '@/lib/workspace/drafts';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
    AlertTriangle,
    Brain,
    CheckCircle,
    Cloud,
    Cpu,
    Database,
    FileText,
    FolderKanban,
    GitBranch,
    Layers,
    RefreshCw,
    Save,
    Server,
    Settings,
    Sparkles,
    XCircle,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';


export default function WorkspacePage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const params = useParams();
  const slug = params?.slug as string;
  const { isLoading: resolvingSlug, error: slugError, isReady } =
    useWorkspaceSlugResolver(slug);
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // Edit mode state
  const [isEditing, setIsEditing] = useState(false);
  const [selectedLLM, setSelectedLLM] = useState<LLMSelection | undefined>(undefined);
  const [selectedEmbedding, setSelectedEmbedding] = useState<EmbeddingSelection | undefined>(undefined);
  const [selectedEntityTypes, setSelectedEntityTypes] = useState<string[]>([
    ...ENTITY_PRESETS.general.types,
  ]);
  const [selectedEntityTypesStrict, setSelectedEntityTypesStrict] = useState(true);

  const {
    workspace,
    stats,
    providerHealth,
    isLoadingWorkspace,
    isLoadingStats,
    isLoadingHealth,
    refetchWorkspace,
  } = useWorkspaceDetailQueries(selectedTenantId, selectedWorkspaceId, {
    enabled: isReady,
  });

  // Update workspace mutation
  const updateMutation = useMutation({
    mutationFn: (data: {
      llm_model?: string;
      llm_provider?: string;
      embedding_model?: string;
      embedding_provider?: string;
      embedding_dimension?: number;
      entity_types?: string[];
      entity_types_strict?: boolean;
      _embeddingChanged?: boolean;
      _llmChanged?: boolean;
    }) =>
      updateWorkspace(selectedTenantId!, selectedWorkspaceId!, {
        llm_model: data.llm_model,
        llm_provider: data.llm_provider,
        embedding_model: data.embedding_model,
        embedding_provider: data.embedding_provider,
        embedding_dimension: data.embedding_dimension,
        entity_types: data.entity_types,
        entity_types_strict: data.entity_types_strict,
      }),
    onSuccess: (_result, variables) => {
      toast.success(t('workspace.updateSuccess', 'Workspace updated successfully'));
      queryClient.invalidateQueries({ queryKey: ['workspace', selectedTenantId, selectedWorkspaceId] });
      setIsEditing(false);
      
      // Check if model changes require rebuild
      const needsEmbeddingRebuild = variables._embeddingChanged;
      const needsExtractionRebuild = variables._llmChanged;
      
      if (needsEmbeddingRebuild || needsExtractionRebuild) {
        setPendingRebuild({
          embeddings: needsEmbeddingRebuild ?? false,
          extraction: needsExtractionRebuild ?? false,
        });
        
        if (needsEmbeddingRebuild && needsExtractionRebuild) {
          toast.info(
            t('workspace.rebuildRequired', 'Model changes detected'),
            {
              description: t(
                'workspace.rebuildBothHint',
                'Both embedding and LLM models changed. Use "Rebuild Embeddings" to reprocess all documents.'
              ),
              duration: 8000,
            }
          );
        } else if (needsEmbeddingRebuild) {
          toast.info(
            t('workspace.embeddingRebuildRequired', 'Embedding model changed'),
            {
              description: t(
                'workspace.embeddingRebuildHint',
                'Use "Rebuild Embeddings" to regenerate vector embeddings with the new model.'
              ),
              duration: 6000,
            }
          );
        } else if (needsExtractionRebuild) {
          toast.info(
            t('workspace.llmRebuildRequired', 'LLM model changed'),
            {
              description: t(
                'workspace.llmRebuildHint',
                'Use "Rebuild Embeddings" to re-extract entities with the new LLM model.'
              ),
              duration: 6000,
            }
          );
        }
      }
    },
    onError: (error) => {
      toast.error(t('workspace.updateFailed', 'Failed to update workspace'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  const handleSave = () => {
    const data: Parameters<typeof updateMutation.mutate>[0] = {
      _embeddingChanged: embeddingModelChanged ?? false,
      _llmChanged: llmModelChanged ?? false,
      entity_types: selectedEntityTypes,
      entity_types_strict: selectedEntityTypesStrict,
      llm_model: selectedLLM?.model ?? '',
      llm_provider: selectedLLM?.provider ?? '',
    };

    if (selectedEmbedding) {
      data.embedding_model = selectedEmbedding.model;
      data.embedding_provider = selectedEmbedding.provider;
      data.embedding_dimension = selectedEmbedding.dimension;
    } else {
      data.embedding_model = '';
      data.embedding_provider = '';
      data.embedding_dimension = 0;
    }

    updateMutation.mutate(data);
  };

  const syncEntityTypeDrafts = (ws: NonNullable<typeof workspace>) => {
    setSelectedEntityTypes(
      ws.entity_types?.length ? [...ws.entity_types] : [...ENTITY_PRESETS.general.types]
    );
    setSelectedEntityTypesStrict(ws.entity_types_strict ?? true);
  };

  const handleCancel = () => {
    setIsEditing(false);
    if (workspace) {
      setSelectedLLM(getWorkspaceLlmSelection(workspace));
      setSelectedEmbedding(getWorkspaceEmbeddingSelection(workspace));
      syncEntityTypeDrafts(workspace);
    }
  };

  const handleEditStart = () => {
    if (!workspace) return;
    setSelectedLLM(getWorkspaceLlmSelection(workspace));
    setSelectedEmbedding(getWorkspaceEmbeddingSelection(workspace));
    syncEntityTypeDrafts(workspace);
    setIsEditing(true);
  };

  // Check if embedding model changed (needs rebuild)
  const embeddingModelChanged = Boolean(
    workspace && selectedEmbedding && (
      workspace.embedding_model !== selectedEmbedding.model ||
      workspace.embedding_provider !== selectedEmbedding.provider
    )
  );

  // Check if LLM model changed (needs extraction rebuild)
  const llmModelChanged = Boolean(
    workspace && selectedLLM && (
      workspace.llm_model !== selectedLLM.model ||
      workspace.llm_provider !== selectedLLM.provider
    )
  );

  // Track if rebuild is needed after save
  const [pendingRebuild, setPendingRebuild] = useState<{
    embeddings: boolean;
    extraction: boolean;
  } | null>(null);

  if (resolvingSlug || !isReady) {
    return <WorkspaceLoading context="workspace configuration" />;
  }

  if (slugError) {
    return (
      <WorkspaceNotFound
        slug={slug}
        fallbackHref="/workspace"
        fallbackLabel={t('workspace.goToSettings', 'Go to Workspace Settings')}
      />
    );
  }

  if (!selectedTenantId || !selectedWorkspaceId) {
    return (
      <div className="container mx-auto p-6">
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <FolderKanban className="h-12 w-12 text-muted-foreground mb-4" />
            <h2 className="text-lg font-medium text-muted-foreground">
              {t('workspace.noWorkspaceSelected', 'No Workspace Selected')}
            </h2>
            <p className="text-sm text-muted-foreground mt-2">
              {t('workspace.selectWorkspaceHint', 'Please select a workspace from the sidebar.')}
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (isLoadingWorkspace) {
    return (
      <div className="container mx-auto p-6 space-y-6">
        <Skeleton className="h-8 w-64" />
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {[...Array(4)].map((_, i) => (
            <Skeleton key={i} className="h-32" />
          ))}
        </div>
        <Skeleton className="h-64" />
      </div>
    );
  }

  if (!workspace) {
    return (
      <div className="container mx-auto p-6">
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <AlertTriangle className="h-12 w-12 text-destructive mb-4" />
            <h2 className="text-lg font-medium">
              {t('workspace.notFound', 'Workspace Not Found')}
            </h2>
            <p className="text-sm text-muted-foreground mt-2">
              {t('workspace.notFoundHint', 'The selected workspace could not be loaded.')}
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6 space-y-6">
      <WorkspacePageHeader
        workspace={workspace}
        isEditing={isEditing}
        isSaving={updateMutation.isPending}
        onRefresh={() => refetchWorkspace()}
        onEditStart={handleEditStart}
        onCancel={handleCancel}
        onSave={handleSave}
      />

      <Separator />

      <WorkspaceStatsCards
        workspace={workspace}
        stats={stats}
        isLoadingStats={isLoadingStats}
      />

      {/* Model Configuration */}      {/* Model Configuration */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* LLM Configuration */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Brain className="h-5 w-5 text-blue-600" />
              {t('workspace.llmConfig', 'LLM Configuration')}
            </CardTitle>
            <CardDescription>
              {t('workspace.llmConfigDesc', 'Model used for entity extraction and summarization during document ingestion.')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {isEditing ? (
              <>
                <LLMModelSelector
                  value={selectedLLM}
                  onChange={setSelectedLLM}
                  showUsageHint
                />
                {llmModelChanged && (
                  <div className="flex items-center gap-2 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                    <AlertTriangle className="h-4 w-4 text-blue-600" />
                    <span className="text-sm text-blue-700 dark:text-blue-300">
                      {t('workspace.llmChangeWarning', 'Changing LLM model requires re-extracting entities from all documents.')}
                    </span>
                  </div>
                )}
              </>
            ) : (
              <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
                {<ProviderIcon providerId={workspace.llm_provider} />}
                <div>
                  <div className="font-medium">
                    {workspace.llm_model || t('workspace.serverDefault', 'Server Default')}
                  </div>
                  <div className="text-sm text-muted-foreground capitalize">
                    {workspace.llm_provider || t('workspace.autoDetect', 'Auto-detected')}
                  </div>
                </div>
                {workspace.llm_full_id && (
                  <Badge variant="outline" className="ml-auto">
                    {workspace.llm_full_id}
                  </Badge>
                )}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Embedding Configuration */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Layers className="h-5 w-5 text-purple-600" />
              {t('workspace.embeddingConfig', 'Embedding Configuration')}
            </CardTitle>
            <CardDescription>
              {t('workspace.embeddingConfigDesc', 'Model used for vector embeddings of document chunks.')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {isEditing ? (
              <>
                <EmbeddingModelSelector
                  value={selectedEmbedding}
                  onChange={setSelectedEmbedding}
                />
                {embeddingModelChanged && (
                  <div className="flex items-center gap-2 p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
                    <AlertTriangle className="h-4 w-4 text-amber-600" />
                    <span className="text-sm text-amber-700 dark:text-amber-300">
                      {t('workspace.embeddingChangeWarning', 'Changing embedding model requires rebuilding all document embeddings.')}
                    </span>
                  </div>
                )}
              </>
            ) : (
              <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
                {<ProviderIcon providerId={workspace.embedding_provider} />}
                <div>
                  <div className="font-medium">
                    {workspace.embedding_model || t('workspace.serverDefault', 'Server Default')}
                  </div>
                  <div className="text-sm text-muted-foreground capitalize">
                    {workspace.embedding_provider || t('workspace.autoDetect', 'Auto-detected')}
                    {workspace.embedding_dimension && (
                      <span className="ml-2">• {workspace.embedding_dimension} dims</span>
                    )}
                  </div>
                </div>
                {workspace.embedding_full_id && (
                  <Badge variant="outline" className="ml-auto">
                    {workspace.embedding_full_id}
                  </Badge>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <WorkspaceEntityTypesCard
        isEditing={isEditing}
        workspace={workspace}
        selectedTypes={selectedEntityTypes}
        onTypesChange={setSelectedEntityTypes}
        strictLimit={selectedEntityTypesStrict}
        onStrictLimitChange={setSelectedEntityTypesStrict}
      />

      <WorkspaceProviderHealthCard
        providerHealth={providerHealth}
        isLoadingHealth={isLoadingHealth}
      />

      {/* Actions Section */}      {/* Actions Section */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings className="h-5 w-5" />
            {t('workspace.actions', 'Workspace Actions')}
          </CardTitle>
          <CardDescription>
            {t('workspace.actionsDesc', 'Manage workspace data and re-process documents.')}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Pending rebuild alert */}
          {pendingRebuild && (pendingRebuild.embeddings || pendingRebuild.extraction) && (
            <div className="flex items-start gap-3 p-4 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
              <AlertTriangle className="h-5 w-5 text-amber-600 mt-0.5 flex-shrink-0" />
              <div className="flex-1">
                <p className="font-medium text-amber-800 dark:text-amber-200">
                  {t('workspace.rebuildPending', 'Rebuild Required')}
                </p>
                <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
                  {pendingRebuild.embeddings && pendingRebuild.extraction ? (
                    t('workspace.rebuildBothPending', 'You changed both LLM and embedding models. Click "Rebuild Embeddings" to reprocess all documents with the new configuration.')
                  ) : pendingRebuild.embeddings ? (
                    t('workspace.rebuildEmbeddingsPending', 'You changed the embedding model. Click "Rebuild Embeddings" to regenerate vector embeddings.')
                  ) : (
                    t('workspace.rebuildExtractionPending', 'You changed the LLM model. Click "Rebuild Embeddings" to re-extract entities from all documents.')
                  )}
                </p>
              </div>
            </div>
          )}
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Rebuild Embeddings */}
            <RebuildEmbeddingsButton
              variant="card"
              onComplete={() => {
                queryClient.invalidateQueries({ queryKey: ['workspaceStats', selectedWorkspaceId] });
                // Clear pending rebuild state after successful rebuild
                setPendingRebuild(null);
              }}
            />

            {/* Rebuild Knowledge Graph */}
            <RebuildKnowledgeGraphButton
              variant="card"
              rebuildEmbeddings={true}
              onComplete={() => {
                queryClient.invalidateQueries({ queryKey: ['workspaceStats', selectedWorkspaceId] });
                queryClient.invalidateQueries({ queryKey: ['documents'] });
                // Clear pending rebuild state after successful rebuild
                setPendingRebuild(null);
              }}
            />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
            {/* Workspace Info Card */}
            <Card className="border-dashed">
              <CardContent className="pt-6">
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">{t('workspace.id', 'Workspace ID')}</span>
                    <code className="text-xs bg-muted px-2 py-1 rounded">{workspace.id}</code>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">{t('workspace.slug', 'Slug')}</span>
                    <code className="text-xs bg-muted px-2 py-1 rounded">{workspace.slug || '-'}</code>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">{t('workspace.created', 'Created')}</span>
                    <span className="text-sm">{new Date(workspace.created_at).toLocaleDateString()}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">{t('workspace.updated', 'Updated')}</span>
                    <span className="text-sm">
                      {workspace.updated_at
                        ? new Date(workspace.updated_at).toLocaleDateString()
                        : '-'}
                    </span>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </CardContent>
      </Card>

      {/* Status Indicator */}
      <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
        <CheckCircle className="h-4 w-4 text-green-500" />
        {t('workspace.statusReady', 'Workspace ready for queries and document ingestion')}
      </div>
    </div>
  );
}
