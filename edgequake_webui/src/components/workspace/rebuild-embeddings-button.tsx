/**
 * @module RebuildEmbeddingsButton
 * @description Button to trigger workspace embedding rebuild with progress tracking
 *
 * @implements SPEC-032: Vector database rebuild on embedding model change
 * @implements SPEC-032 Focus Area 5: Rebuild with progress display
 * @iteration OODA #22 - WebUI for rebuild embeddings
 *
 * @enforces BR0401 - Users can rebuild embeddings when changing models
 * @enforces BR0402 - Clear warning before destructive operations
 */

'use client';

import { PipelineStatusDialog } from '@/components/documents/pipeline-status-dialog';
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
import {
  rebuildEmbeddings,
  reprocessAllDocuments,
  type RebuildEmbeddingsResponse,
  type ReprocessAllResponse,
} from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, RefreshCw, RotateCcw } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface RebuildEmbeddingsButtonProps {
  /**
   * Whether to show the full card version or just a button
   */
  variant?: 'button' | 'card';
  /**
   * Callback when rebuild is complete
   */
  onComplete?: (response: RebuildEmbeddingsResponse) => void;
}

/**
 * Button/Card to rebuild workspace embeddings with progress tracking.
 *
 * This component allows users to trigger a rebuild of all vector embeddings
 * for the currently selected workspace. This is necessary when:
 * - Changing embedding models
 * - Switching embedding providers
 * - Fixing corrupted embeddings
 *
 * After clearing embeddings, it automatically triggers document reprocessing
 * and displays progress via the PipelineStatusDialog.
 *
 * @param variant - Display variant ('button' or 'card')
 * @param onComplete - Callback when rebuild completes
 */
export function RebuildEmbeddingsButton({
  variant = 'button',
  onComplete,
}: RebuildEmbeddingsButtonProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { selectedWorkspaceId, workspaces } = useTenantStore();
  const [isConfirmOpen, setIsConfirmOpen] = useState(false);
  const [isPipelineOpen, setIsPipelineOpen] = useState(false);
  const [reprocessResult, setReprocessResult] = useState<ReprocessAllResponse | null>(null);

  // Find selected workspace
  const selectedWorkspace = workspaces.find((w) => w.id === selectedWorkspaceId);

  // Reprocess mutation (step 2: queue documents for re-embedding)
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
        toast.info(
          t(
            'workspace.rebuild.reprocessing',
            `Queued ${response.documents_queued} documents for reprocessing`
          )
        );
        // Open pipeline status dialog to show progress
        setIsPipelineOpen(true);
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
    },
    onError: (error: Error) => {
      toast.error(
        t('workspace.rebuild.reprocessError', `Failed to queue documents: ${error.message}`)
      );
    },
  });

  // Rebuild mutation (step 1: clear embeddings)
  const rebuildMutation = useMutation({
    mutationFn: async () => {
      if (!selectedWorkspaceId) {
        throw new Error('No workspace selected');
      }
      return rebuildEmbeddings(selectedWorkspaceId, { force: true });
    },
    onSuccess: (response) => {
      toast.success(
        t(
          'workspace.rebuild.success',
          `Embeddings cleared! ${response.documents_to_process} documents need reprocessing.`
        )
      );
      setIsConfirmOpen(false);
      onComplete?.(response);
      
      // Automatically trigger reprocessing
      if (response.documents_to_process > 0) {
        reprocessMutation.mutate();
      }
    },
    onError: (error: Error) => {
      toast.error(
        t('workspace.rebuild.error', `Failed to rebuild embeddings: ${error.message}`)
      );
    },
  });

  const handleRebuild = () => {
    rebuildMutation.mutate();
  };

  const isLoading = rebuildMutation.isPending || reprocessMutation.isPending;

  // Button content
  const buttonContent = (
    <Button
      variant="outline"
      disabled={!selectedWorkspaceId || isLoading}
      className="gap-2"
    >
      {isLoading ? (
        <RefreshCw className="h-4 w-4 animate-spin" />
      ) : (
        <RotateCcw className="h-4 w-4" />
      )}
      {isLoading
        ? t('workspace.rebuild.processing', 'Processing...')
        : t('workspace.rebuild.button', 'Rebuild Embeddings')}
    </Button>
  );

  // Confirmation dialog
  const confirmDialog = (
    <AlertDialog open={isConfirmOpen} onOpenChange={setIsConfirmOpen}>
      <AlertDialogTrigger asChild>{buttonContent}</AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-destructive" />
            {t('workspace.rebuild.confirm.title', 'Rebuild Embeddings?')}
          </AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-2 text-sm text-muted-foreground">
              <span className="block">
                {t(
                  'workspace.rebuild.confirm.description',
                  'This will clear all vector embeddings for workspace:'
                )}
              </span>
              <span className="block font-medium text-foreground">
                {selectedWorkspace?.name || selectedWorkspaceId}
              </span>
              <div className="mt-4 rounded-md bg-amber-50 p-3 text-sm text-amber-800 dark:bg-amber-950 dark:text-amber-200">
                <span className="block font-medium">
                  {t('workspace.rebuild.confirm.warning', 'Warning:')}
                </span>
                <ul className="mt-1 list-inside list-disc space-y-1">
                  <li>
                    {t(
                      'workspace.rebuild.confirm.warning1',
                      'All existing embeddings will be deleted'
                    )}
                  </li>
                  <li>
                    {t(
                      'workspace.rebuild.confirm.warning2',
                      'Documents will be automatically reprocessed'
                    )}
                  </li>
                  <li>
                    {t(
                      'workspace.rebuild.confirm.warning3',
                      'Queries may return empty results until reprocessing completes'
                    )}
                  </li>
                </ul>
              </div>
              <div className="mt-2 rounded-md bg-blue-50 p-3 text-sm text-blue-800 dark:bg-blue-950 dark:text-blue-200">
                <span className="block">
                  {t(
                    'workspace.rebuild.confirm.info',
                    'A progress dialog will appear to track reprocessing status.'
                  )}
                </span>
              </div>
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>
            {t('common.cancel', 'Cancel')}
          </AlertDialogCancel>
          <AlertDialogAction
            onClick={handleRebuild}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {isLoading ? (
              <>
                <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                {t('workspace.rebuild.rebuilding', 'Rebuilding...')}
              </>
            ) : (
              t('workspace.rebuild.confirm.action', 'Yes, Rebuild')
            )}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );

  // Card variant
  if (variant === 'card') {
    return (
      <>
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="flex items-center gap-2 text-base">
              <RotateCcw className="h-4 w-4 text-primary" />
              {t('workspace.rebuild.card.title', 'Workspace Embeddings')}
            </CardTitle>
            <CardDescription className="text-sm">
              {t(
                'workspace.rebuild.card.description',
                'Rebuild vector embeddings when changing embedding models or providers.'
              )}
            </CardDescription>
          </CardHeader>
          <CardContent className="pt-0">
            <div className="space-y-3">
              {selectedWorkspace && (
                <div className="rounded-md border p-3 text-sm">
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">
                      {t('workspace.rebuild.card.current', 'Current Model:')}
                    </span>
                    <span className="font-mono text-xs">
                      {selectedWorkspace.embedding_provider}/{selectedWorkspace.embedding_model}
                    </span>
                  </div>
                  <div className="flex items-center justify-between mt-1">
                    <span className="text-muted-foreground">
                      {t('workspace.rebuild.card.dimension', 'Dimension:')}
                    </span>
                    <span className="font-mono text-xs">
                      {selectedWorkspace.embedding_dimension}
                    </span>
                  </div>
                </div>
              )}
              {confirmDialog}
            </div>
          </CardContent>
        </Card>
        
        {/* Pipeline Status Dialog for progress tracking */}
        <PipelineStatusDialog
          open={isPipelineOpen}
          onOpenChange={setIsPipelineOpen}
        />
      </>
    );
  }

  // Button variant
  return (
    <>
      {confirmDialog}
      
      {/* Pipeline Status Dialog for progress tracking */}
      <PipelineStatusDialog
        open={isPipelineOpen}
        onOpenChange={setIsPipelineOpen}
      />
    </>
  );
}

export default RebuildEmbeddingsButton;
