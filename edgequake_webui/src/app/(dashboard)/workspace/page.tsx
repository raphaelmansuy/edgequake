/**
 * @module WorkspacePage
 * @description Current workspace detail page showing configuration, stats, and actions.
 *
 * @implements SPEC-032: Workspace configuration display
 * @implements FEAT0801: Workspace detail view with LLM/embedding configuration
 * @implements UC0305: User views workspace configuration
 *
 * @enforces BR0305: Workspace config is visible and editable
 * @enforces BR0306: Rebuild action available when model changes
 */
'use client';

import {
  type PdfParserBackendChoice,
} from '@/components/settings/pdf-parser-backend-field';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import type { EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import type { LLMSelection } from '@/components/workspace/llm-model-selector';
import { WorkspaceActionsCard } from '@/components/workspace/workspace-actions-card';
import { WorkspaceExtendedModelConfig } from '@/components/workspace/workspace-extended-model-config';
import { WorkspaceModelConfigGrid } from '@/components/workspace/workspace-model-config-grid';
import { WorkspaceStatusFooter } from '@/components/workspace/workspace-status-footer';
import { WorkspaceEntityTypesCard } from '@/components/workspace/workspace-entity-types-card';
import { WorkspacePageHeader } from '@/components/workspace/workspace-page-header';
import { WorkspaceProviderHealthCard } from '@/components/workspace/workspace-provider-health-card';
import { WorkspaceStatsCards } from '@/components/workspace/workspace-stats-cards';
import { ENTITY_PRESETS } from '@/constants/entity-presets';
import { useWorkspaceDetailQueries } from '@/hooks/use-workspace-detail-queries';
import { useWorkspaceTenantValidator } from '@/hooks/use-workspace-tenant-validator';
import { deleteWorkspace, updateWorkspace } from '@/lib/api/edgequake';
import {
  getWorkspaceEmbeddingSelection,
  getWorkspaceLlmSelection,
  getWorkspacePdfParserBackend,
  getWorkspaceVisionSelection,
} from '@/lib/workspace/drafts';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  FolderKanban,
  RefreshCw,
  Trash2,
} from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';


export default function WorkspacePage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const router = useRouter();
  const { selectedTenantId, selectedWorkspaceId, selectWorkspace } = useTenantStore();

  // Auto-validate workspace-tenant consistency and fix mismatches
  useWorkspaceTenantValidator({
    onValidationFailed: (result) => {
      console.error('[Workspace] Workspace-tenant mismatch detected:', result.reason);
      toast.error('Workspace context corrected', {
        description: 'Your workspace selection was updated to match the current tenant.',
      });
    },
  });

  // Edit mode state
  const [isEditing, setIsEditing] = useState(false);
  const [selectedLLM, setSelectedLLM] = useState<LLMSelection | undefined>(undefined);
  const [selectedEmbedding, setSelectedEmbedding] = useState<EmbeddingSelection | undefined>(undefined);
  const [selectedVisionLLM, setSelectedVisionLLM] = useState<LLMSelection | undefined>(undefined);
  const [selectedPdfParserBackend, setSelectedPdfParserBackend] =
    useState<PdfParserBackendChoice>('none');
  const [selectedEntityTypes, setSelectedEntityTypes] = useState<string[]>([
    ...ENTITY_PRESETS.general.types,
  ]);
  const [selectedEntityTypesStrict, setSelectedEntityTypesStrict] = useState(true);
  // FIX #171: Delete workspace state
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);

  // FIX #171: Delete workspace handler
  const handleDeleteWorkspace = async () => {
    if (!selectedWorkspaceId) return;
    setIsDeleting(true);
    try {
      await deleteWorkspace(selectedWorkspaceId);
      selectWorkspace(null);
      queryClient.invalidateQueries({ queryKey: ['workspaces'] });
      toast.success(t('workspace.deleted', 'Workspace deleted'));
      router.push('/');
    } catch (err) {
      toast.error(`Failed to delete workspace: ${err instanceof Error ? err.message : 'Unknown error'}`);
    } finally {
      setIsDeleting(false);
      setShowDeleteConfirm(false);
    }
  };

  const {
    workspace,
    stats,
    providerHealth,
    isLoadingWorkspace,
    isLoadingStats,
    isLoadingHealth,
    refetchWorkspace,
  } = useWorkspaceDetailQueries(selectedTenantId, selectedWorkspaceId);

  // Update workspace mutation
  const updateMutation = useMutation({
    mutationFn: (data: {
      llm_model?: string;
      llm_provider?: string;
      embedding_model?: string;
      embedding_provider?: string;
      embedding_dimension?: number;
      vision_llm_provider?: string;
      vision_llm_model?: string;
      pdf_parser_backend?: PdfParserBackendChoice;
      entity_types?: string[];
      entity_types_strict?: boolean;
      _embeddingChanged?: boolean;
      _llmChanged?: boolean;
      _visionChanged?: boolean;
    }) =>
      updateWorkspace(selectedTenantId!, selectedWorkspaceId!, {
        llm_model: data.llm_model,
        llm_provider: data.llm_provider,
        embedding_model: data.embedding_model,
        embedding_provider: data.embedding_provider,
        embedding_dimension: data.embedding_dimension,
        vision_llm_provider: data.vision_llm_provider,
        vision_llm_model: data.vision_llm_model,
        pdf_parser_backend: data.pdf_parser_backend,
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
      const needsVisionRebuild = variables._visionChanged;
      
      if (needsEmbeddingRebuild || needsExtractionRebuild || needsVisionRebuild) {
        setPendingRebuild({
          embeddings: needsEmbeddingRebuild ?? false,
          extraction: needsExtractionRebuild ?? false,
          vision: needsVisionRebuild ?? false,
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
                'Use "Rebuild Knowledge Graph" to re-extract entities with the new LLM model.'
              ),
              duration: 6000,
            }
          );
        } else if (needsVisionRebuild) {
          toast.info(
            t('workspace.visionRebuildRequired', 'Vision LLM model changed'),
            {
              description: t(
                'workspace.visionRebuildHint',
                'Use "Rebuild Knowledge Graph" to re-extract PDF documents with the new vision model from original files.'
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
      _visionChanged: visionLLMChanged ?? false,
      entity_types: selectedEntityTypes,
      entity_types_strict: selectedEntityTypesStrict,
    };

    // SPEC-013: empty strings clear workspace override → server/env defaults (same as vision)
    data.llm_model = selectedLLM?.model ?? '';
    data.llm_provider = selectedLLM?.provider ?? '';

    if (selectedEmbedding) {
      data.embedding_model = selectedEmbedding.model;
      data.embedding_provider = selectedEmbedding.provider;
      data.embedding_dimension = selectedEmbedding.dimension;
    } else {
      data.embedding_model = '';
      data.embedding_provider = '';
      data.embedding_dimension = 0;
    }

    // Vision LLM config (SPEC-040: empty string clears workspace override)
    data.vision_llm_provider = selectedVisionLLM?.provider ?? '';
    data.vision_llm_model = selectedVisionLLM?.model ?? '';
    data.pdf_parser_backend = selectedPdfParserBackend;
    updateMutation.mutate(data);
  };

  const handleCancel = () => {
    setIsEditing(false);
    setSelectedLLM(getWorkspaceLlmSelection(workspace));
    setSelectedEmbedding(getWorkspaceEmbeddingSelection(workspace));
    setSelectedVisionLLM(getWorkspaceVisionSelection(workspace));
    setSelectedPdfParserBackend(getWorkspacePdfParserBackend(workspace));
    setSelectedEntityTypes(
      workspace?.entity_types?.length
        ? [...workspace.entity_types]
        : [...ENTITY_PRESETS.general.types]
    );
    setSelectedEntityTypesStrict(workspace?.entity_types_strict ?? true);
  };

  const handleEditStart = () => {
    setSelectedLLM(getWorkspaceLlmSelection(workspace));
    setSelectedEmbedding(getWorkspaceEmbeddingSelection(workspace));
    setSelectedVisionLLM(getWorkspaceVisionSelection(workspace));
    setSelectedPdfParserBackend(getWorkspacePdfParserBackend(workspace));
    setSelectedEntityTypes(
      workspace?.entity_types?.length
        ? [...workspace.entity_types]
        : [...ENTITY_PRESETS.general.types]
    );
    setSelectedEntityTypesStrict(workspace?.entity_types_strict ?? true);
    setIsEditing(true);
  };

  // Check if embedding model changed (needs rebuild)
  const embeddingModelChanged = Boolean(
    workspace && (
      selectedEmbedding
        ? workspace.embedding_model !== selectedEmbedding.model ||
          workspace.embedding_provider !== selectedEmbedding.provider
        : Boolean(workspace.embedding_provider || workspace.embedding_model)
    )
  );

  // Check if LLM model changed (needs extraction rebuild)
  const llmModelChanged = Boolean(
    workspace && (
      selectedLLM
        ? workspace.llm_model !== selectedLLM.model ||
          workspace.llm_provider !== selectedLLM.provider
        : Boolean(workspace.llm_provider || workspace.llm_model)
    )
  );

  // Check if Vision LLM changed (triggers full re-extraction of existing PDF documents from originals)
  const visionLLMChanged = Boolean(
    workspace && selectedVisionLLM && (
      workspace.vision_llm_model !== selectedVisionLLM.model ||
      workspace.vision_llm_provider !== selectedVisionLLM.provider
    )
  );

  // Track if rebuild is needed after save
  const [pendingRebuild, setPendingRebuild] = useState<{
    embeddings: boolean;
    extraction: boolean;
    vision: boolean;
  } | null>(null);

  if (!selectedTenantId || !selectedWorkspaceId) {
    return (
      <ScrollArea className="h-full">
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
      </ScrollArea>
    );
  }

  if (isLoadingWorkspace) {
    return (
      <ScrollArea className="h-full">
        <div className="container mx-auto p-6 space-y-6">
          <Skeleton className="h-8 w-64" />
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {[...Array(4)].map((_, i) => (
              <Skeleton key={i} className="h-32" />
            ))}
          </div>
          <Skeleton className="h-64" />
        </div>
      </ScrollArea>
    );
  }

  if (!workspace) {
    return (
      <ScrollArea className="h-full">
        <div className="container mx-auto p-6">
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12">
              <AlertTriangle className="h-12 w-12 text-destructive mb-4" />
              <h2 className="text-lg font-medium">
                {t('workspace.notFound', 'Workspace Not Found')}
              </h2>
              <p className="text-sm text-muted-foreground mt-2 mb-4">
                {t('workspace.notFoundHint', 'The selected workspace could not be loaded.')}
              </p>
              <Button
                variant="outline"
                onClick={() => refetchWorkspace()}
              >
                <RefreshCw className="h-4 w-4 mr-2" />
                {t('common.retry', 'Retry')}
              </Button>
            </CardContent>
          </Card>
        </div>
      </ScrollArea>
    );
  }

  return (
    <ScrollArea className="h-full">
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

      <WorkspaceExtendedModelConfig
        workspace={workspace}
        isEditing={isEditing}
        selectedVisionLLM={selectedVisionLLM}
        selectedPdfParserBackend={selectedPdfParserBackend}
        onVisionLlmChange={setSelectedVisionLLM}
        onPdfParserBackendChange={setSelectedPdfParserBackend}
        visionLLMChanged={visionLLMChanged ?? false}
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
        includeVisionPending
        onRebuildComplete={() => {
          queryClient.invalidateQueries({ queryKey: ['workspaceStats', selectedWorkspaceId] });
          queryClient.invalidateQueries({ queryKey: ['documents'] });
          setPendingRebuild(null);
        }}
      />

      <WorkspaceStatusFooter />

        {/* FIX #171: Danger Zone — Delete Workspace */}
        <Card className="border-destructive/50">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-destructive">
              <Trash2 className="h-5 w-5" />
              {t('workspace.dangerZone', 'Danger Zone')}
            </CardTitle>
            <CardDescription>
              {t('workspace.deleteWarning', 'Deleting a workspace permanently removes all documents, entities, relationships, and embeddings. This action cannot be undone.')}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button
              variant="destructive"
              className="w-full sm:w-auto"
              aria-label={t('workspace.deleteButtonAria', 'Delete workspace {{name}}', { name: workspace.name })}
              onClick={() => setShowDeleteConfirm(true)}
            >
              <Trash2 className="h-4 w-4 mr-2" />
              {t('workspace.deleteButton', 'Delete this workspace')}
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* Delete Workspace Confirmation */}
      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t('workspace.deleteConfirmTitle', 'Delete Workspace')}</AlertDialogTitle>
            <AlertDialogDescription>
              {t('workspace.deleteConfirmDesc', 'Are you sure you want to delete workspace "{name}"? This will permanently remove all documents, entities, relationships, and embeddings.', { name: workspace?.name || '' })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel autoFocus disabled={isDeleting}>{t('common.cancel', 'Cancel')}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteWorkspace}
              disabled={isDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {isDeleting ? t('workspace.deleting', 'Deleting...') : t('workspace.deleteConfirmButton', 'Delete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </ScrollArea>
  );
}
