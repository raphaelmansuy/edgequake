/**
 * @module WorkspacePage
 * @description Current workspace detail page showing configuration, stats, and actions.
 *
 * @implements SPEC-032: Workspace configuration display
 * @implements SPEC-101 LAW-101-12: Reconfigure via guided wizard (read-only overview)
 * @implements FEAT0801: Workspace detail view with LLM/embedding configuration
 * @implements UC0305: User views workspace configuration
 *
 * @enforces BR0305: Workspace config is visible and editable
 * @enforces BR0306: Rebuild action available when model changes
 */
'use client';

import { ReconfigureWorkspaceWizard } from '@/components/onboarding/reconfigure-workspace-wizard';
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
import { WorkspaceActionsCard } from '@/components/workspace/workspace-actions-card';
import { WorkspaceExtendedModelConfig } from '@/components/workspace/workspace-extended-model-config';
import { WorkspaceModelConfigGrid } from '@/components/workspace/workspace-model-config-grid';
import { WorkspaceStatusFooter } from '@/components/workspace/workspace-status-footer';
import { WorkspaceEntityTypesCard } from '@/components/workspace/workspace-entity-types-card';
import { WorkspaceRelationTypesCard } from '@/components/workspace/workspace-relation-types-card';
import { WorkspaceExtractionLanguageCard } from '@/components/workspace/workspace-extraction-language-card';
import { AuthzSettingsSection } from '@/components/settings/authz-settings-section';
import { WorkspaceChunkingCard } from '@/components/workspace/workspace-chunking-card';
import { WorkspaceExtractBudgetCard } from '@/components/workspace/workspace-extract-budget-card';
import { WorkspacePageHeader } from '@/components/workspace/workspace-page-header';
import { WorkspaceStatsCards } from '@/components/workspace/workspace-stats-cards';
import { ENTITY_PRESETS } from '@/constants/entity-presets';
import { useWorkspaceDetailQueries } from '@/hooks/use-workspace-detail-queries';
import { useWorkspaceTenantValidator } from '@/hooks/use-workspace-tenant-validator';
import { deleteWorkspace } from '@/lib/api/edgequake';
import type { WorkspaceRebuildHints } from '@/lib/onboarding/workspace-config-diff';
import {
  getWorkspacePdfParserBackend,
} from '@/lib/workspace/drafts';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { Workspace } from '@/types';
import { useQueryClient } from '@tanstack/react-query';
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

  useWorkspaceTenantValidator({
    onValidationFailed: (result) => {
      console.error('[Workspace] Workspace-tenant mismatch detected:', result.reason);
      toast.error('Workspace context corrected', {
        description: 'Your workspace selection was updated to match the current tenant.',
      });
    },
  });

  const [reconfigureOpen, setReconfigureOpen] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [pendingRebuild, setPendingRebuild] = useState<WorkspaceRebuildHints | null>(null);

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
      toast.error(
        `Failed to delete workspace: ${err instanceof Error ? err.message : 'Unknown error'}`,
      );
    } finally {
      setIsDeleting(false);
      setShowDeleteConfirm(false);
    }
  };

  const {
    workspace,
    stats,
    isLoadingWorkspace,
    isLoadingStats,
    refetchWorkspace,
  } = useWorkspaceDetailQueries(selectedTenantId, selectedWorkspaceId);

  const documentCount = stats?.document_count ?? workspace?.document_count ?? 0;

  const handleReconfigureApplied = (result: {
    workspace: Workspace;
    pendingRebuild: WorkspaceRebuildHints | null;
    extractionLanguageChanged: boolean;
  }) => {
    toast.success(t('workspace.updateSuccess', 'Workspace updated successfully'));
    queryClient.setQueryData(
      ['workspace', selectedTenantId, selectedWorkspaceId],
      result.workspace,
    );
    queryClient.invalidateQueries({
      queryKey: ['workspace', selectedTenantId, selectedWorkspaceId],
    });
    if (result.extractionLanguageChanged) {
      toast.info(
        t(
          'workspace.extractionLanguage.changedToast',
          'Extraction language updated. Reprocess documents to refresh the graph.',
        ),
        { duration: 6000 },
      );
    }
    if (result.pendingRebuild) {
      setPendingRebuild(result.pendingRebuild);
      const { embeddings, extraction, vision } = result.pendingRebuild;
      if (embeddings && extraction) {
        toast.info(t('workspace.rebuildRequired', 'Model changes detected'), {
          description: t(
            'workspace.rebuildBothHint',
            'Both embedding and LLM models changed. Use "Rebuild Embeddings" to reprocess all documents.',
          ),
          duration: 8000,
        });
      } else if (embeddings) {
        toast.info(t('workspace.embeddingRebuildRequired', 'Embedding model changed'), {
          description: t(
            'workspace.embeddingRebuildHint',
            'Use "Rebuild Embeddings" to regenerate vector embeddings with the new model.',
          ),
          duration: 6000,
        });
      } else if (extraction) {
        toast.info(t('workspace.llmRebuildRequired', 'LLM model changed'), {
          description: t(
            'workspace.llmRebuildHint',
            'Use "Rebuild Knowledge Graph" to re-extract entities with the new LLM model.',
          ),
          duration: 6000,
        });
      } else if (vision) {
        toast.info(t('workspace.visionRebuildRequired', 'Vision LLM model changed'), {
          description: t(
            'workspace.visionRebuildHint',
            'Use "Rebuild Knowledge Graph" to re-extract PDF documents with the new vision model from original files.',
          ),
          duration: 6000,
        });
      }
    }
  };

  if (!selectedTenantId || !selectedWorkspaceId) {
    return (
      <ScrollArea className="h-full">
        <div className="container mx-auto p-page">
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12">
              <FolderKanban className="h-12 w-12 text-muted-foreground mb-4" />
              <h2 className="text-lg font-medium text-muted-foreground">
                {t('workspace.noWorkspaceSelected', 'No Workspace Selected')}
              </h2>
              <p className="text-sm text-muted-foreground mt-2">
                {t(
                  'workspace.selectWorkspaceHint',
                  'Please select a workspace from the sidebar.',
                )}
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
        <div
          className="container mx-auto space-y-page p-page"
          data-testid="spec100-workspace-skeleton"
        >
          <Skeleton className="h-10 w-72" />
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4">
            {[...Array(4)].map((_, i) => (
              <Skeleton key={i} className="h-32" />
            ))}
          </div>
          <Skeleton className="min-h-[16rem] w-full" />
          <Skeleton className="min-h-[12rem] w-full" />
        </div>
      </ScrollArea>
    );
  }

  if (!workspace) {
    return (
      <ScrollArea className="h-full">
        <div className="container mx-auto p-page">
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12">
              <AlertTriangle className="h-12 w-12 text-destructive mb-4" />
              <h2 className="text-lg font-medium">
                {t('workspace.notFound', 'Workspace Not Found')}
              </h2>
              <p className="text-sm text-muted-foreground mt-2 mb-4">
                {t(
                  'workspace.notFoundHint',
                  'The selected workspace could not be loaded.',
                )}
              </p>
              <Button variant="outline" onClick={() => refetchWorkspace()}>
                <RefreshCw className="h-4 w-4 mr-2" />
                {t('common.retry', 'Retry')}
              </Button>
            </CardContent>
          </Card>
        </div>
      </ScrollArea>
    );
  }

  const entityTypes = workspace.entity_types?.length
    ? workspace.entity_types
    : [...ENTITY_PRESETS.general.types];

  return (
    <ScrollArea className="h-full">
      <div className="container mx-auto space-y-page p-page">
        <WorkspacePageHeader
          workspace={workspace}
          onRefresh={() => refetchWorkspace()}
          onEditStart={() => setReconfigureOpen(true)}
        />

        <Separator />

        <WorkspaceStatsCards
          workspace={workspace}
          stats={stats}
          isLoadingStats={isLoadingStats}
        />

        <WorkspaceModelConfigGrid
          workspace={workspace}
          isEditing={false}
          selectedLLM={undefined}
          selectedEmbedding={undefined}
          onLlmChange={() => {}}
          onEmbeddingChange={() => {}}
          llmModelChanged={false}
          embeddingModelChanged={false}
        />

        <WorkspaceExtendedModelConfig
          workspace={workspace}
          isEditing={false}
          selectedVisionLLM={undefined}
          selectedPdfParserBackend={getWorkspacePdfParserBackend(workspace)}
          onVisionLlmChange={() => {}}
          onPdfParserBackendChange={() => {}}
          visionLLMChanged={false}
        />

        <WorkspaceExtractionLanguageCard
          isEditing={false}
          workspace={workspace}
          selectedLanguage={workspace.extraction_language ?? null}
          onLanguageChange={() => {}}
        />

        <WorkspaceChunkingCard isEditing={false} workspace={workspace} />
        <WorkspaceExtractBudgetCard isEditing={false} workspace={workspace} />

        <div
          className="grid grid-cols-1 gap-4 lg:grid-cols-2 lg:items-start"
          data-testid="workspace-kg-schema-row"
        >
          <WorkspaceEntityTypesCard
            isEditing={false}
            workspace={workspace}
            selectedTypes={entityTypes}
            onTypesChange={() => {}}
            strictLimit={workspace.entity_types_strict ?? true}
            onStrictLimitChange={() => {}}
            extractionLanguage={workspace.extraction_language ?? null}
          />

          <WorkspaceRelationTypesCard workspace={workspace} />
        </div>

        <WorkspaceActionsCard
          workspace={workspace}
          pendingRebuild={pendingRebuild}
          includeVisionPending
          onRebuildComplete={() => {
            queryClient.invalidateQueries({
              queryKey: ['workspaceStats', selectedWorkspaceId],
            });
            queryClient.invalidateQueries({ queryKey: ['documents'] });
            setPendingRebuild(null);
          }}
        />

        {/* SPEC-146 M1b: ABAC PAP cards (gated by /health doc_abac) */}
        <AuthzSettingsSection />

        <WorkspaceStatusFooter />

        <Card className="gap-2 border-destructive/50 py-4">
          <CardHeader className="flex flex-col gap-3 px-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="min-w-0 space-y-1">
              <CardTitle className="flex items-center gap-2 text-base text-destructive">
                <Trash2 className="h-4 w-4" />
                {t('workspace.dangerZone', 'Danger Zone')}
              </CardTitle>
              <CardDescription className="text-xs">
                {t(
                  'workspace.deleteWarning',
                  'Deleting a workspace permanently removes all documents, entities, relationships, and embeddings. This action cannot be undone.',
                )}
              </CardDescription>
            </div>
            <Button
              variant="destructive"
              className="w-full shrink-0 sm:w-auto"
              aria-label={t('workspace.deleteButtonAria', 'Delete workspace {{name}}', {
                name: workspace.name,
              })}
              onClick={() => setShowDeleteConfirm(true)}
            >
              <Trash2 className="h-4 w-4 mr-2" />
              {t('workspace.deleteButton', 'Delete this workspace')}
            </Button>
          </CardHeader>
        </Card>
      </div>

      <ReconfigureWorkspaceWizard
        open={reconfigureOpen}
        onOpenChange={setReconfigureOpen}
        tenantId={selectedTenantId}
        workspace={workspace}
        documentCount={documentCount}
        onApplied={handleReconfigureApplied}
      />

      <AlertDialog open={showDeleteConfirm} onOpenChange={setShowDeleteConfirm}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t('workspace.deleteConfirmTitle', 'Delete Workspace')}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                'workspace.deleteConfirmDesc',
                'Are you sure you want to delete workspace "{name}"? This will permanently remove all documents, entities, relationships, and embeddings.',
                { name: workspace?.name || '' },
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel autoFocus disabled={isDeleting}>
              {t('common.cancel', 'Cancel')}
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteWorkspace}
              disabled={isDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {isDeleting
                ? t('workspace.deleting', 'Deleting...')
                : t('workspace.deleteConfirmButton', 'Delete')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </ScrollArea>
  );
}
