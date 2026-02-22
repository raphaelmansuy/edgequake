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
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { EmbeddingModelSelector, type EmbeddingSelection } from '@/components/workspace/embedding-model-selector';
import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import { RebuildEmbeddingsButton } from '@/components/workspace/rebuild-embeddings-button';
import { RebuildKnowledgeGraphButton } from '@/components/workspace/rebuild-knowledge-graph-button';
import { useWorkspaceTenantValidator } from '@/hooks/use-workspace-tenant-validator';
import { checkHealth, getWorkspace, getWorkspaceStats, updateWorkspace } from '@/lib/api/edgequake';
import { fetchProvidersHealth } from '@/lib/api/models';
import { useTenantStore } from '@/stores/use-tenant-store';
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
    Server,
    Settings,
    Sparkles,
    XCircle,
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
  // OODA-ITERATION-03-CACHE-FIX: Reduced staleTime from 30s to 0 to force fresh fetches
  // This ensures stats are always current, especially after document uploads
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
    staleTime: 0, // Always fetch fresh stats to reflect latest document processing
    refetchOnMount: 'always', // Always refetch when component mounts
  });

  // Fetch health to get current active provider configuration
  // WHY: When workspace has 0 documents, show current active provider from environment
  // instead of stale workspace record (which may show old provider like "ollama/embeddinggemma"
  // even when OpenAI is now active)
  const {
    data: healthData,
  } = useQuery({
    queryKey: ['health'],
    queryFn: checkHealth,
    staleTime: 10000, // Cache for 10 seconds
    retry: 1,
  });

  // Fetch provider health status (SPEC-032: OODA 201-210)
  const {
    data: providerHealth,
    isLoading: isLoadingHealth,
  } = useQuery({
    queryKey: ['providersHealth'],
    queryFn: fetchProvidersHealth,
    staleTime: 60000, // Cache for 1 minute
    retry: 1, // Only retry once since providers may be down
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
      if (workspace.vision_llm_provider && workspace.vision_llm_model) {
        setSelectedVisionLLM({
          model: workspace.vision_llm_model,
          provider: workspace.vision_llm_provider,
          fullId: `${workspace.vision_llm_provider}/${workspace.vision_llm_model}`,
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
      vision_llm_provider?: string;
      vision_llm_model?: string;
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
    const data: Record<string, string | number | boolean | undefined> = {};

    if (selectedLLM) {
      data.llm_model = selectedLLM.model;
      data.llm_provider = selectedLLM.provider;
    }

    if (selectedEmbedding) {
      data.embedding_model = selectedEmbedding.model;
      data.embedding_provider = selectedEmbedding.provider;
      data.embedding_dimension = selectedEmbedding.dimension;
    }

    // Vision LLM config (SPEC-040: empty string clears workspace override)
    data.vision_llm_provider = selectedVisionLLM?.provider ?? '';
    data.vision_llm_model = selectedVisionLLM?.model ?? '';

    // Track which models changed for post-save rebuild notification
    data._embeddingChanged = embeddingModelChanged ?? false;
    data._llmChanged = llmModelChanged ?? false;
    data._visionChanged = visionLLMChanged ?? false;

    updateMutation.mutate(data as Parameters<typeof updateMutation.mutate>[0]);
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
      if (workspace.vision_llm_provider && workspace.vision_llm_model) {
        setSelectedVisionLLM({
          model: workspace.vision_llm_model,
          provider: workspace.vision_llm_provider,
          fullId: `${workspace.vision_llm_provider}/${workspace.vision_llm_model}`,
        });
      } else {
        setSelectedVisionLLM(undefined);
      }
    }
  };

  // Check if embedding model changed (needs rebuild)
  const embeddingModelChanged = workspace && selectedEmbedding && (
    workspace.embedding_model !== selectedEmbedding.model ||
    workspace.embedding_provider !== selectedEmbedding.provider
  );

  // Check if LLM model changed (needs extraction rebuild)
  const llmModelChanged = workspace && selectedLLM && (
    workspace.llm_model !== selectedLLM.model ||
    workspace.llm_provider !== selectedLLM.provider
  );

  // Check if Vision LLM changed (triggers full re-extraction of existing PDF documents from originals)
  const visionLLMChanged = workspace && selectedVisionLLM && (
    workspace.vision_llm_model !== selectedVisionLLM.model ||
    workspace.vision_llm_provider !== selectedVisionLLM.provider
  );

  // Track if rebuild is needed after save
  const [pendingRebuild, setPendingRebuild] = useState<{
    embeddings: boolean;
    extraction: boolean;
    vision: boolean;
  } | null>(null);

  if (!selectedTenantId || !selectedWorkspaceId) {
    return (
      <ScrollArea className="h-[calc(100vh-theme(spacing.20))]">
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
      <ScrollArea className="h-[calc(100vh-theme(spacing.20))]">
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
      <ScrollArea className="h-[calc(100vh-theme(spacing.20))]">
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
    <ScrollArea className="h-[calc(100vh-theme(spacing.20))]">
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
            ) : (() => {
              // FIXED: Always show workspace's saved LLM configuration
              // Do not override with environment defaults even when workspace has 0 documents
              const displayProvider = workspace.llm_provider;
              const displayModel = workspace.llm_model;
              const displayFullId = workspace.llm_full_id;

              return (
                <>
                  <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
                    {getProviderIcon(displayProvider)}
                    <div>
                      <div className="font-medium">
                        {displayModel || t('workspace.serverDefault', 'Server Default')}
                      </div>
                      <div className="text-sm text-muted-foreground capitalize">
                        {displayProvider || t('workspace.autoDetect', 'Auto-detected')}
                      </div>
                    </div>
                    {displayFullId && (
                      <Badge variant="outline" className="ml-auto">
                        {displayFullId}
                      </Badge>
                    )}
                  </div>
                </>
              );
            })()}
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
            ) : (() => {
              // FIXED: Always show workspace's saved embedding configuration
              // Do not override with environment defaults even when workspace has 0 documents
              const displayProvider = workspace.embedding_provider;
              const displayModel = workspace.embedding_model;
              const displayDimension = workspace.embedding_dimension;
              const displayFullId = workspace.embedding_full_id;

              return (
                <>
                  <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
                    {getProviderIcon(displayProvider)}
                    <div>
                      <div className="font-medium">
                        {displayModel || t('workspace.serverDefault', 'Server Default')}
                      </div>
                      <div className="text-sm text-muted-foreground capitalize">
                        {displayProvider || t('workspace.autoDetect', 'Auto-detected')}
                        {displayDimension && (
                          <span className="ml-2">• {displayDimension} dims</span>
                        )}
                      </div>
                    </div>
                    {displayFullId && (
                      <Badge variant="outline" className="ml-auto">
                        {displayFullId}
                      </Badge>
                    )}
                  </div>
                </>
              );
            })()}
          </CardContent>
        </Card>
      </div>

      {/* Vision LLM Configuration - SPEC-040: PDF-to-Markdown vision model */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-orange-600" />
            {t('workspace.visionLlmConfig', 'Vision LLM (PDF Extraction)')}
          </CardTitle>
          <CardDescription>
            {t('workspace.visionLlmConfigDesc', 'Multimodal model used for PDF page rendering and text extraction. Overrides server default for this workspace.')}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isEditing ? (
            <>
              <LLMModelSelector
                value={selectedVisionLLM}
                onChange={setSelectedVisionLLM}
                showUsageHint
              />
              {visionLLMChanged && (
                <div className="flex items-center gap-2 p-3 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-lg">
                  <AlertTriangle className="h-4 w-4 text-orange-600" />
                  <span className="text-sm text-orange-700 dark:text-orange-300">
                    {t('workspace.visionLlmChangeWarning', 'New Vision LLM will be used for all subsequent PDF uploads.')}
                  </span>
                </div>
              )}
            </>
          ) : (
            <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
              {getProviderIcon(workspace.vision_llm_provider)}
              <div>
                <div className="font-medium">
                  {workspace.vision_llm_model || t('workspace.serverDefault', 'Server Default')}
                </div>
                <div className="text-sm text-muted-foreground capitalize">
                  {workspace.vision_llm_provider || t('workspace.autoDetect', 'Auto-detected')}
                </div>
              </div>
              {workspace.vision_llm_provider && workspace.vision_llm_model && (
                <Badge variant="outline" className="ml-auto">
                  {`${workspace.vision_llm_provider}/${workspace.vision_llm_model}`}
                </Badge>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Provider Health Status - SPEC-032: OODA 201-210 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5 text-slate-600" />
            {t('workspace.providerHealth', 'Provider Status')}
          </CardTitle>
          <CardDescription>
            {t('workspace.providerHealthDesc', 'Real-time availability of configured LLM and embedding providers.')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoadingHealth ? (
            <div className="flex gap-2">
              {[...Array(3)].map((_, i) => (
                <Skeleton key={i} className="h-8 w-24" />
              ))}
            </div>
          ) : providerHealth && providerHealth.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {providerHealth.filter(p => p.enabled).map((provider) => {
                const isAvailable = provider.health?.available ?? provider.enabled;
                return (
                  <Badge
                    key={provider.name}
                    variant={isAvailable ? 'default' : 'secondary'}
                    className={`flex items-center gap-1.5 px-3 py-1.5 ${
                      isAvailable 
                        ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300 border-green-200 dark:border-green-800' 
                        : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-300 border-red-200 dark:border-red-800'
                    }`}
                  >
                    {isAvailable ? (
                      <CheckCircle className="h-3 w-3" />
                    ) : (
                      <XCircle className="h-3 w-3" />
                    )}
                    <span className="capitalize">{provider.display_name || provider.name}</span>
                    {provider.models && provider.models.length > 0 && (
                      <span className="text-xs opacity-70">({provider.models.length})</span>
                    )}
                  </Badge>
                );
              })}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              {t('workspace.noProvidersConfigured', 'No providers configured')}
            </p>
          )}
        </CardContent>
      </Card>

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
        <CardContent className="space-y-4">
          {/* Pending rebuild alert */}
          {pendingRebuild && (pendingRebuild.embeddings || pendingRebuild.extraction || pendingRebuild.vision) && (
            <div className="flex items-start gap-3 p-4 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
              <AlertTriangle className="h-5 w-5 text-amber-600 mt-0.5 flex-shrink-0" />
              <div className="flex-1">
                <p className="font-medium text-amber-800 dark:text-amber-200">
                  {t('workspace.rebuildPending', 'Rebuild Required')}
                </p>
                <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
                  {pendingRebuild.embeddings && pendingRebuild.extraction ? (
                    t('workspace.rebuildBothPending', 'You changed both LLM and embedding models. Click "Rebuild Knowledge Graph" to reprocess all documents from original files with the new configuration.')
                  ) : pendingRebuild.embeddings ? (
                    t('workspace.rebuildEmbeddingsPending', 'You changed the embedding model. Click "Rebuild Embeddings" to regenerate vector embeddings.')
                  ) : pendingRebuild.vision ? (
                    t('workspace.rebuildVisionPending', 'You changed the Vision LLM model. Click "Rebuild Knowledge Graph" to re-extract all PDF documents from their original files using the new vision model.')
                  ) : (
                    t('workspace.rebuildExtractionPending', 'You changed the LLM model. Click "Rebuild Knowledge Graph" to re-extract entities from all documents.')
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
    </ScrollArea>
  );
}
