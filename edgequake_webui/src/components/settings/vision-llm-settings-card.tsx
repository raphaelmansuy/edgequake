'use client';

/**
 * Vision LLM Settings Card
 *
 * Allows users to configure the default Vision LLM model for the current workspace
 * directly from the global Settings page.
 *
 * @implements SPEC-040: Vision LLM workspace override for PDF extraction
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import { getWorkspace, updateWorkspace } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Brain, Cloud, Cpu, Eye, Pencil, Save, Sparkles, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

/**
 * Return a small icon for each LLM provider.
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

/**
 * VisionLLMSettingsCard
 *
 * Reads the current workspace's `vision_llm_provider` / `vision_llm_model`
 * and lets the user pick a new one via `LLMModelSelector`.
 */
export function VisionLLMSettingsCard() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  const [isEditing, setIsEditing] = useState(false);
  const [selectedVisionLLM, setSelectedVisionLLM] = useState<LLMSelection | undefined>(undefined);

  // Fetch the workspace to get current vision LLM config
  const { data: workspace, isLoading } = useQuery({
    queryKey: ['workspace', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getWorkspace(selectedTenantId!, selectedWorkspaceId!),
    enabled: !!selectedTenantId && !!selectedWorkspaceId,
    staleTime: 60000,
    retry: 1,
  });

  // Sync local state from workspace when not editing
  useEffect(() => {
    if (workspace && !isEditing) {
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
  }, [workspace, isEditing]);

  // Save mutation
  const updateMutation = useMutation({
    mutationFn: () =>
      updateWorkspace(selectedTenantId!, selectedWorkspaceId!, {
        vision_llm_provider: selectedVisionLLM?.provider ?? '',
        vision_llm_model: selectedVisionLLM?.model ?? '',
      }),
    onSuccess: () => {
      toast.success(t('settings.vision.updateSuccess', 'Vision LLM configuration updated'));
      queryClient.invalidateQueries({ queryKey: ['workspace', selectedTenantId, selectedWorkspaceId] });
      setIsEditing(false);
    },
    onError: (error) => {
      toast.error(t('settings.vision.updateFailed', 'Failed to update vision LLM configuration'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  const handleCancel = () => {
    setIsEditing(false);
    // Reset to saved workspace values
    if (workspace) {
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

  // Don't render if no workspace context
  if (!selectedTenantId || !selectedWorkspaceId) {
    return null;
  }

  return (
    <Card>
      <CardHeader className="pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Eye className="h-5 w-5 text-orange-600" />
            <CardTitle>{t('settings.vision.title', 'Vision LLM Default')}</CardTitle>
          </div>
          {!isEditing && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setIsEditing(true)}
              aria-label={t('common.edit', 'Edit')}
            >
              <Pencil className="h-4 w-4" />
            </Button>
          )}
        </div>
        <CardDescription>
          {t(
            'settings.vision.subtitle',
            'Default Vision LLM model for document image analysis and PDF visual extraction'
          )}
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        {isLoading ? (
          <Skeleton className="h-14 w-full" />
        ) : isEditing ? (
          <>
            <LLMModelSelector
              value={selectedVisionLLM}
              onChange={setSelectedVisionLLM}
              showUsageHint
            />
            <div className="flex items-center gap-2 pt-2">
              <Button
                size="sm"
                onClick={() => updateMutation.mutate()}
                disabled={updateMutation.isPending}
              >
                <Save className="h-4 w-4 mr-2" />
                {t('common.save', 'Save')}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={handleCancel}
                disabled={updateMutation.isPending}
              >
                <X className="h-4 w-4 mr-2" />
                {t('common.cancel', 'Cancel')}
              </Button>
            </div>
          </>
        ) : workspace ? (
          <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
            {getProviderIcon(workspace.vision_llm_provider)}
            <div>
              <div className="font-medium">
                {workspace.vision_llm_model ||
                  t('settings.vision.serverDefault', 'Server Default')}
              </div>
              <div className="text-sm text-muted-foreground capitalize">
                {workspace.vision_llm_provider || t('workspace.autoDetect', 'Auto-detected')}
              </div>
            </div>
            {workspace.vision_llm_provider && workspace.vision_llm_model && (
              <Badge variant="outline" className="ml-auto">
                {workspace.vision_llm_provider}/{workspace.vision_llm_model}
              </Badge>
            )}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            {t('settings.vision.noVisionModelDesc', 'Configure a vision model to enable image-aware document processing')}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
