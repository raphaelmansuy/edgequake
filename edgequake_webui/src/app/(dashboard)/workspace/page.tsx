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

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { EmbeddingModelSelector, type EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import { RebuildEmbeddingsButton } from '@/components/workspace/rebuild-embeddings-button';
import { getWorkspace, getWorkspaceStats, updateWorkspace } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { Workspace, WorkspaceStats } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
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
  Settings,
  Sparkles,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

/**
 * Get icon component for a provider.
 */
function getProviderIcon(providerId: string | undefined) {
  switch (providerId?.toLowerCase()) {
    case 'openai':
      return <Cloud className="h-4 w-4 text-green-600" />;
    case 'ollama':
      return <Cpu className="h-4 w-4 text-blue-600" />;
    case 'lmstudio':
      return <Brain className="h-4 w-4 text-purple-600" />;
    default:
      return <Sparkles className="h-4 w-4 text-muted-foreground" />;
  }
}

export default function WorkspacePage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // Edit mode state
  const [isEditing, setIsEditing] = useState(false);
  const [selectedLLM, setSelectedLLM] = useState<LLMSelection | undefined>(undefined);
  const [selectedEmbedding, setSelectedEmbedding] = useState<EmbeddingSelection | undefined>(undefined);

  // Fetch workspace data
  const {
    data: workspace,
    isLoading: isLoadingWorkspace,
    refetch: refetchWorkspace,
  } = useQuery({
    queryKey: ['workspace', selectedTenantId, selectedWorkspaceId],
    queryFn: () =>
      selectedTenantId && selectedWorkspaceId
        ? getWorkspace(selectedTenantId, selectedWorkspaceId)
        : Promise.reject(new Error('No workspace selected')),
    enabled: !!selectedTenantId && !!selectedWorkspaceId,
    staleTime: 30000,
  });

  // Fetch workspace stats
  const {
    data: stats,
    isLoading: isLoadingStats,
  } = useQuery({
    queryKey: ['workspaceStats', selectedWorkspaceId],
    queryFn: () =>
      selectedWorkspaceId
        ? getWorkspaceStats(selectedWorkspaceId)
        : Promise.reject(new Error('No workspace selected')),
    enabled: !!selectedWorkspaceId,
    staleTime: 30000,
  });

  // Initialize edit state from workspace data
  useEffect(() => {
    if (workspace && !isEditing) {
      if (workspace.llm_provider && workspace.llm_model) {
        setSelectedLLM({
          model: workspace.llm_model,
          provider: workspace.llm_provider,
          fullId: `${workspace.llm_provider}/${workspace.llm_model}`,
        });
      }
      if (workspace.embedding_provider && workspace.embedding_model) {
        setSelectedEmbedding({
          model: workspace.embedding_model,
          provider: workspace.embedding_provider,
          dimension: workspace.embedding_dimension ?? 768,
        });
      }
    }
  }, [workspace, isEditing]);

  // Update workspace mutation
  const updateMutation = useMutation({
    mutationFn: (data: {
      llm_model?: string;
      llm_provider?: string;
      embedding_model?: string;
      embedding_provider?: string;
      embedding_dimension?: number;
    }) =>
      updateWorkspace(selectedTenantId!, selectedWorkspaceId!, data),
    onSuccess: () => {
      toast.success(t('workspace.updateSuccess', 'Workspace updated successfully'));
      queryClient.invalidateQueries({ queryKey: ['workspace', selectedTenantId, selectedWorkspaceId] });
      setIsEditing(false);
    },
    onError: (error) => {
      toast.error(t('workspace.updateFailed', 'Failed to update workspace'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  const handleSave = () => {
    const data: Record<string, string | number | undefined> = {};

    if (selectedLLM) {
      data.llm_model = selectedLLM.model;
      data.llm_provider = selectedLLM.provider;
    }

    if (selectedEmbedding) {
      data.embedding_model = selectedEmbedding.model;
      data.embedding_provider = selectedEmbedding.provider;
      data.embedding_dimension = selectedEmbedding.dimension;
    }

    updateMutation.mutate(data);
  };

  const handleCancel = () => {
    setIsEditing(false);
    // Reset to workspace values
    if (workspace) {
      if (workspace.llm_provider && workspace.llm_model) {
        setSelectedLLM({
          model: workspace.llm_model,
          provider: workspace.llm_provider,
          fullId: `${workspace.llm_provider}/${workspace.llm_model}`,
        });
      } else {
        setSelectedLLM(undefined);
      }
      if (workspace.embedding_provider && workspace.embedding_model) {
        setSelectedEmbedding({
          model: workspace.embedding_model,
          provider: workspace.embedding_provider,
          dimension: workspace.embedding_dimension ?? 768,
        });
      } else {
        setSelectedEmbedding(undefined);
      }
    }
  };

  // Check if embedding model changed (needs rebuild)
  const embeddingModelChanged = workspace && selectedEmbedding && (
    workspace.embedding_model !== selectedEmbedding.model ||
    workspace.embedding_provider !== selectedEmbedding.provider
  );

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
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-3">
            <FolderKanban className="h-8 w-8 text-primary" />
            <h1 className="text-2xl font-bold">{workspace.name}</h1>
            <Badge variant={workspace.is_active ? 'default' : 'secondary'}>
              {workspace.is_active ? t('common.active', 'Active') : t('common.inactive', 'Inactive')}
            </Badge>
          </div>
          {workspace.description && (
            <p className="text-muted-foreground">{workspace.description}</p>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => refetchWorkspace()}
          >
            <RefreshCw className="h-4 w-4 mr-2" />
            {t('common.refresh', 'Refresh')}
          </Button>
          {!isEditing ? (
            <Button
              variant="default"
              size="sm"
              onClick={() => setIsEditing(true)}
            >
              <Settings className="h-4 w-4 mr-2" />
              {t('workspace.editConfig', 'Edit Configuration')}
            </Button>
          ) : (
            <>
              <Button
                variant="outline"
                size="sm"
                onClick={handleCancel}
              >
                {t('common.cancel', 'Cancel')}
              </Button>
              <Button
                variant="default"
                size="sm"
                onClick={handleSave}
                disabled={updateMutation.isPending}
              >
                <Save className="h-4 w-4 mr-2" />
                {t('common.save', 'Save')}
              </Button>
            </>
          )}
        </div>
      </div>

      <Separator />

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <FileText className="h-4 w-4" />
              {t('workspace.documents', 'Documents')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {isLoadingStats ? (
                <Skeleton className="h-8 w-16" />
              ) : (
                stats?.document_count ?? workspace.document_count ?? 0
              )}
            </div>
            {workspace.max_documents && (
              <p className="text-xs text-muted-foreground mt-1">
                {t('workspace.maxDocuments', 'Max')}: {workspace.max_documents.toLocaleString()}
              </p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <GitBranch className="h-4 w-4" />
              {t('workspace.entities', 'Entities')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {isLoadingStats ? (
                <Skeleton className="h-8 w-16" />
              ) : (
                stats?.entity_count ?? workspace.entity_count ?? 0
              )}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <Layers className="h-4 w-4" />
              {t('workspace.relationships', 'Relationships')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {isLoadingStats ? (
                <Skeleton className="h-8 w-16" />
              ) : (
                stats?.relationship_count ?? 0
              )}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <Database className="h-4 w-4" />
              {t('workspace.chunks', 'Chunks')}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {isLoadingStats ? (
                <Skeleton className="h-8 w-16" />
              ) : (
                stats?.chunk_count ?? 0
              )}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Model Configuration */}
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
              <LLMModelSelector
                value={selectedLLM}
                onChange={setSelectedLLM}
                showUsageHint
              />
            ) : (
              <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
                {getProviderIcon(workspace.llm_provider)}
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
                {getProviderIcon(workspace.embedding_provider)}
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

      {/* Actions Section */}
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
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Rebuild Embeddings */}
            <RebuildEmbeddingsButton
              variant="card"
              onComplete={() => {
                queryClient.invalidateQueries({ queryKey: ['workspaceStats', selectedWorkspaceId] });
              }}
            />

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
