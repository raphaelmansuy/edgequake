/**
 * @module RebuildKnowledgeGraphButton
 * @description Button to trigger workspace knowledge graph rebuild with progress tracking
 *
 * @implements SPEC-032: Knowledge graph rebuild on LLM model change
 * @implements OODA 256-280: Workspace-scoped rebuild endpoints
 * @iteration OODA #282 - WebUI for rebuild knowledge graph
 * @iteration OODA #04 - UX improvement for rebuild confirmation dialogs
 *
 * @enforces BR0401 - Users can rebuild knowledge graph when changing models
 * @enforces BR0402 - Clear warning before destructive operations
 */

'use client';

import { PipelineStatusDialog, type ClearStats } from '@/components/documents/pipeline-status-dialog';
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
    AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import { formatEstimatedTime, useWorkspaceStats } from '@/hooks/use-workspace-stats';
import {
    rebuildKnowledgeGraph,
    reprocessAllDocuments,
    type RebuildKnowledgeGraphResponse,
    type ReprocessAllResponse,
} from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, Clock, FileText, Network, RefreshCw } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface RebuildKnowledgeGraphButtonProps {
  /**
   * Whether to show the full card version or just a button
   */
  variant?: 'button' | 'card';
  /**
   * Whether to also rebuild embeddings (default: true)
   */
  rebuildEmbeddings?: boolean;
  /**
   * Callback when rebuild is complete
   */
  onComplete?: (response: RebuildKnowledgeGraphResponse) => void;
}

/**
 * Button/Card to rebuild workspace knowledge graph with progress tracking.
 *
 * This component allows users to trigger a rebuild of the knowledge graph
 * for the currently selected workspace. This is necessary when:
 * - Changing LLM models
 * - Switching LLM providers
 * - Fixing corrupted graph data
 *
 * After clearing the graph, it automatically triggers document reprocessing
 * and displays progress via the PipelineStatusDialog.
 *
 * @param variant - Display variant ('button' or 'card')
 * @param rebuildEmbeddings - Whether to also rebuild embeddings
 * @param onComplete - Callback when rebuild completes
 */
export function RebuildKnowledgeGraphButton({
  variant = 'button',
  rebuildEmbeddings = true,
  onComplete,
}: RebuildKnowledgeGraphButtonProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { selectedTenantId, selectedWorkspaceId, workspaces } = useTenantStore();
  const [isConfirmOpen, setIsConfirmOpen] = useState(false);
  const [isPipelineOpen, setIsPipelineOpen] = useState(false);
  const [reprocessResult, setReprocessResult] = useState<ReprocessAllResponse | null>(null);
  // OODA-26: Store clear stats from rebuild response
  const [clearStats, setClearStats] = useState<ClearStats | null>(null);

  // Fetch workspace stats for impact preview
  const { totalDocuments, estimatedTimeMinutes, isLoading: statsLoading } = useWorkspaceStats();

  // Find selected workspace
  const selectedWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);

  // Reprocess mutation (step 2: queue documents for re-extraction)
  const reprocessMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkspaceId) {
        throw new Error('No workspace selected');
      }
      return reprocessAllDocuments(selectedWorkspaceId, {
        include_completed: true,
        max_documents: 10000,
      });
    },
    onSuccess: (response) => {
      setReprocessResult(response);
      if (response.documents_queued > 0) {
        // WHY: Show both queued and skipped counts for transparency
        // Users need to know if some documents were skipped (e.g., missing content)
        // @implements BR0401 - Clear feedback on rebuild operations
        const skippedInfo = response.documents_skipped > 0
          ? ` (${response.documents_skipped} skipped)`
          : '';
        toast.info(
          t(
            'workspace.rebuild.reprocessing',
            `Queued ${response.documents_queued} documents for reprocessing${skippedInfo}`
          )
        );
        // Open pipeline status dialog to show progress
        setIsPipelineOpen(true);
      } else if (response.documents_found > 0 && response.documents_skipped > 0) {
        // All documents were skipped - show warning with reason
        toast.warning(
          t(
            'workspace.rebuild.allSkipped',
            `Found ${response.documents_found} documents but all were skipped (${response.documents_skipped})`
          ),
          {
            description: t(
              'workspace.rebuild.skippedReason',
              'Documents may be missing content or already processing'
            ),
          }
        );
      } else {
        toast.info(
          t(
            'workspace.rebuild.noDocuments',
            'No documents to reprocess'
          )
        );
      }
      // Invalidate queries to refresh data
      queryClient.invalidateQueries({ queryKey: ['enhanced-pipeline-status'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-status'] });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
      queryClient.invalidateQueries({ queryKey: ['knowledge-graph'] });
    },
    onError: (error: Error) => {
      toast.error(
        t('workspace.rebuild.reprocessError', 'Failed to queue documents'),
        {
          description: error.message,
          action: {
            label: t('common.retry', 'Retry'),
            onClick: () => reprocessMutation.mutate(),
          },
        }
      );
    },
  });

  // Rebuild mutation (step 1: clear graph)
  const rebuildMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkspaceId) {
        throw new Error('No workspace selected');
      }
      return rebuildKnowledgeGraph(selectedWorkspaceId, {
        force: true,
        rebuild_embeddings: rebuildEmbeddings,
      });
    },
    onSuccess: (response) => {
      // OODA-26: Store clear stats for display in pipeline dialog
      setClearStats({
        nodesCleared: response.nodes_cleared,
        edgesCleared: response.edges_cleared,
        vectorsCleared: rebuildEmbeddings ? response.vectors_cleared : undefined,
      });
      
      const clearedInfo = rebuildEmbeddings
        ? `${response.nodes_cleared} nodes, ${response.edges_cleared} edges, ${response.vectors_cleared} vectors cleared`
        : `${response.nodes_cleared} nodes, ${response.edges_cleared} edges cleared`;
      
      // Show chunk count for better insight into processing time
      const chunkInfo = response.chunks_to_process > 0
        ? ` (${response.chunks_to_process} chunks)`
        : '';

      toast.success(
        t(
          'workspace.rebuild.graphSuccess',
          `Knowledge graph cleared! ${clearedInfo}. ${response.documents_to_process} documents${chunkInfo} need reprocessing.`
        )
      );
      setIsConfirmOpen(false);
      onComplete?.(response);

      // Automatically trigger reprocessing
      reprocessMutation.mutate();
    },
    onError: (error: Error) => {
      toast.error(
        t('workspace.rebuild.graphError', 'Failed to rebuild knowledge graph'),
        {
          description: error.message,
          action: {
            label: t('common.retry', 'Retry'),
            onClick: () => rebuildMutation.mutate(),
          },
        }
      );
    },
  });

  const handleRebuild = () => {
    rebuildMutation.mutate();
  };

  const isLoading = rebuildMutation.isPending || reprocessMutation.isPending;

  // Card variant
  if (variant === 'card') {
    return (
      <>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Network className="h-5 w-5" />
              {t('workspace.rebuild.graphTitle', 'Rebuild Knowledge Graph')}
            </CardTitle>
            <CardDescription>
              {t(
                'workspace.rebuild.graphDescription',
                'Clear and rebuild the knowledge graph when changing LLM models or fixing corrupted data. This will delete all entities and relationships.'
              )}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <AlertDialog open={isConfirmOpen} onOpenChange={setIsConfirmOpen}>
              <AlertDialogTrigger asChild>
                <Button
                  variant="outline"
                  disabled={!selectedWorkspaceId || isLoading}
                  className="gap-2 w-full sm:w-auto"
                  data-testid="rebuild-kg-button"
                >
                  {isLoading ? (
                    <RefreshCw className="h-4 w-4 animate-spin" />
                  ) : (
                    <Network className="h-4 w-4" />
                  )}
                  {isLoading
                    ? t('workspace.rebuild.processing', 'Processing...')
                    : t('workspace.rebuild.graphButton', 'Rebuild Knowledge Graph')}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle className="flex items-center gap-2">
                    <AlertTriangle className="h-5 w-5 text-destructive" />
                    {t('workspace.rebuild.graphConfirm.title', 'Rebuild Knowledge Graph?')}
                  </AlertDialogTitle>
                  <AlertDialogDescription asChild>
                    <div className="space-y-2 text-sm text-muted-foreground">
                      <span className="block">
                        {t(
                          'workspace.rebuild.graphConfirm.description',
                          'This will clear all knowledge graph data (entities and relationships) for workspace:'
                        )}
                      </span>
                      <span className="font-semibold text-foreground block">
                        {selectedWorkspace?.name || 'Unknown Workspace'}
                      </span>
                      
                      {/* Impact preview section */}
                      <div className="mt-3 rounded-md border bg-muted/50 p-3" data-testid="rebuild-kg-impact-preview">
                        <div className="flex items-center gap-2 text-foreground font-medium mb-2">
                          <FileText className="h-4 w-4" />
                          {t('workspace.rebuild.confirm.impact', 'Impact:')}
                        </div>
                        <div className="grid grid-cols-2 gap-2 text-xs">
                          <div className="flex items-center gap-1.5">
                            <span className="text-muted-foreground">{t('workspace.rebuild.confirm.documents', 'Documents:')}</span>
                            <span className="font-mono font-medium text-foreground">
                              {statsLoading ? '...' : totalDocuments}
                            </span>
                          </div>
                          <div className="flex items-center gap-1.5">
                            <Clock className="h-3 w-3 text-muted-foreground" />
                            <span className="text-muted-foreground">{t('workspace.rebuild.confirm.eta', 'Estimated:')}</span>
                            <span className="font-medium text-foreground">
                              {statsLoading ? '...' : formatEstimatedTime(estimatedTimeMinutes)}
                            </span>
                          </div>
                        </div>
                      </div>
                      
                      {rebuildEmbeddings && (
                        <span className="block text-yellow-600 dark:text-yellow-500">
                          {t(
                            'workspace.rebuild.graphConfirm.embeddings',
                            '⚠️ This will also clear vector embeddings'
                          )}
                        </span>
                      )}
                      <span className="block mt-2">
                        {t(
                          'workspace.rebuild.graphConfirm.reprocess',
                          'All documents will be automatically queued for reprocessing to rebuild the graph.'
                        )}
                      </span>
                    </div>
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel disabled={isLoading} data-testid="rebuild-kg-cancel">
                    {t('common.cancel', 'Cancel')}
                  </AlertDialogCancel>
                  <AlertDialogAction
                    onClick={handleRebuild}
                    disabled={isLoading}
                    className="bg-destructive hover:bg-destructive/90"
                    data-testid="rebuild-kg-confirm"
                  >
                    {isLoading ? (
                      <>
                        <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                        {t('workspace.rebuild.processing', 'Processing...')}
                      </>
                    ) : (
                      t('workspace.rebuild.graphConfirm.proceed', 'Rebuild Graph')
                    )}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </CardContent>
        </Card>

        {/* Pipeline status dialog */}
        <PipelineStatusDialog
          open={isPipelineOpen}
          onOpenChange={setIsPipelineOpen}
          title={t('workspace.rebuild.pipelineTitle', 'Rebuilding Knowledge Graph')}
          subtitle={
            reprocessResult
              ? t(
                  'workspace.rebuild.pipelineSubtitle',
                  `Processing ${reprocessResult.documents_queued} documents`
                )
              : undefined
          }
          clearStats={clearStats ?? undefined}
          tenantId={selectedTenantId ?? undefined}
          workspaceId={selectedWorkspaceId ?? undefined}
        />
      </>
    );
  }

  // Button variant
  return (
    <>
      <AlertDialog open={isConfirmOpen} onOpenChange={setIsConfirmOpen}>
        <AlertDialogTrigger asChild>
          <Button
            variant="outline"
            disabled={!selectedWorkspaceId || isLoading}
            className="gap-2"
            data-testid="rebuild-kg-button"
          >
            {isLoading ? (
              <RefreshCw className="h-4 w-4 animate-spin" />
            ) : (
              <Network className="h-4 w-4" />
            )}
            {isLoading
              ? t('workspace.rebuild.processing', 'Processing...')
              : t('workspace.rebuild.graphButton', 'Rebuild Knowledge Graph')}
          </Button>
        </AlertDialogTrigger>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2">
              <AlertTriangle className="h-5 w-5 text-destructive" />
              {t('workspace.rebuild.graphConfirm.title', 'Rebuild Knowledge Graph?')}
            </AlertDialogTitle>
            <AlertDialogDescription asChild>
              <div className="space-y-2 text-sm text-muted-foreground">
                <span className="block">
                  {t(
                    'workspace.rebuild.graphConfirm.description',
                    'This will clear all knowledge graph data (entities and relationships) for workspace:'
                  )}
                </span>
                <span className="font-semibold text-foreground block">
                  {selectedWorkspace?.name || 'Unknown Workspace'}
                </span>
                
                {/* Impact preview section */}
                <div className="mt-3 rounded-md border bg-muted/50 p-3" data-testid="rebuild-kg-impact-preview">
                  <div className="flex items-center gap-2 text-foreground font-medium mb-2">
                    <FileText className="h-4 w-4" />
                    {t('workspace.rebuild.confirm.impact', 'Impact:')}
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div className="flex items-center gap-1.5">
                      <span className="text-muted-foreground">{t('workspace.rebuild.confirm.documents', 'Documents:')}</span>
                      <span className="font-mono font-medium text-foreground">
                        {statsLoading ? '...' : totalDocuments}
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      <Clock className="h-3 w-3 text-muted-foreground" />
                      <span className="text-muted-foreground">{t('workspace.rebuild.confirm.eta', 'Estimated:')}</span>
                      <span className="font-medium text-foreground">
                        {statsLoading ? '...' : formatEstimatedTime(estimatedTimeMinutes)}
                      </span>
                    </div>
                  </div>
                </div>
                
                {rebuildEmbeddings && (
                  <span className="block text-yellow-600 dark:text-yellow-500">
                    {t(
                      'workspace.rebuild.graphConfirm.embeddings',
                      '⚠️ This will also clear vector embeddings'
                    )}
                  </span>
                )}
                <span className="block mt-2">
                  {t(
                    'workspace.rebuild.graphConfirm.reprocess',
                    'All documents will be automatically queued for reprocessing to rebuild the graph.'
                  )}
                </span>
              </div>
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isLoading} data-testid="rebuild-kg-cancel">
              {t('common.cancel', 'Cancel')}
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={handleRebuild}
              disabled={isLoading}
              className="bg-destructive hover:bg-destructive/90"
              data-testid="rebuild-kg-confirm"
            >
              {isLoading ? (
                <>
                  <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                  {t('workspace.rebuild.processing', 'Processing...')}
                </>
              ) : (
                t('workspace.rebuild.graphConfirm.proceed', 'Rebuild Graph')
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Pipeline status dialog */}
      <PipelineStatusDialog
        open={isPipelineOpen}
        onOpenChange={setIsPipelineOpen}
        title={t('workspace.rebuild.pipelineTitle', 'Rebuilding Knowledge Graph')}
        subtitle={
          reprocessResult
            ? t(
                'workspace.rebuild.pipelineSubtitle',
                `Processing ${reprocessResult.documents_queued} documents`
              )
            : undefined
        }
        clearStats={clearStats ?? undefined}
        tenantId={selectedTenantId ?? undefined}
        workspaceId={selectedWorkspaceId ?? undefined}
      />
    </>
  );
}
