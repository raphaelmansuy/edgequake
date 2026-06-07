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

import { useParams } from 'next/navigation';

import {
  WorkspaceLoading,
  WorkspaceNotFound,
} from '@/components/workspace/workspace-deeplink-states';
import { WorkspaceEntityTypesCard } from '@/components/workspace/workspace-entity-types-card';
import { WorkspacePageHeader } from '@/components/workspace/workspace-page-header';
import { WorkspaceProviderHealthCard } from '@/components/workspace/workspace-provider-health-card';
import { WorkspaceActionsCard } from '@/components/workspace/workspace-actions-card';
import { WorkspaceModelConfigGrid } from '@/components/workspace/workspace-model-config-grid';
import { WorkspaceStatusFooter } from '@/components/workspace/workspace-status-footer';
import { WorkspaceStatsCards } from '@/components/workspace/workspace-stats-cards';
import { ENTITY_PRESETS } from '@/constants/entity-presets';
import { useWorkspaceDetailQueries } from '@/hooks/use-workspace-detail-queries';
import { useWorkspaceSlugResolver } from '@/hooks/use-workspace-slug-resolver';
import { Card, CardContent } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import type { EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import type { LLMSelection } from '@/components/workspace/llm-model-selector';
import { updateWorkspace } from '@/lib/api/edgequake';
import {
  getWorkspaceEmbeddingSelection,
  getWorkspaceLlmSelection,
} from '@/lib/workspace/drafts';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  FolderKanban,
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

      
      <WorkspaceModelConfigGrid
        workspace={workspace}
        isEditing={isEditing}
        selectedLLM={selectedLLM}
        selectedEmbedding={selectedEmbedding}
        onLlmChange={setSelectedLLM}
        onEmbeddingChange={setSelectedEmbedding}
        llmModelChanged={llmModelChanged ?? false}
        embeddingModelChanged={embeddingModelChanged ?? false}
      />

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

      <WorkspaceActionsCard
        workspace={workspace}
        pendingRebuild={pendingRebuild}
        onRebuildComplete={() => {
          queryClient.invalidateQueries({ queryKey: ['workspaceStats', selectedWorkspaceId] });
          queryClient.invalidateQueries({ queryKey: ['documents'] });
          setPendingRebuild(null);
        }}
      />

      <WorkspaceStatusFooter />
    </div>
  );
}
