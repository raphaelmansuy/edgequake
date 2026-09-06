'use client';

/**
 * Vision LLM Settings Card
 *
 * Allows users to configure the default Vision LLM model for the current workspace
 * directly from the global Settings page.
 *
 * @implements SPEC-040: Vision LLM workspace override for PDF extraction
 */

import { ProviderIcon } from '@/components/providers/provider-icon';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import { getWorkspace, updateWorkspace } from '@/lib/api/edgequake';
import { effectiveVisionFromWorkspace } from '@/lib/config/resolve-model-choice';
import { getWorkspaceVisionSelection } from '@/lib/workspace/drafts';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Eye, Pencil, Save, X } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

/**
 * Return a small icon for each LLM provider.
 */

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
    setSelectedVisionLLM(getWorkspaceVisionSelection(workspace));
  };

  // Don't render if no workspace context
  if (!selectedTenantId || !selectedWorkspaceId) {
    return null;
  }

  return (
    <Card>
      <CardHeader className="pb-4">
        <div className="flex flex-wrap items-center justify-between gap-2 min-w-0">
          <div className="flex items-center gap-2 min-w-0">
            <Eye className="h-5 w-5 text-orange-600 shrink-0" />
            <CardTitle className="truncate">{t('settings.vision.title', 'Vision LLM Default')}</CardTitle>
          </div>
          {!isEditing && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setSelectedVisionLLM(getWorkspaceVisionSelection(workspace));
                setIsEditing(true);
              }}
              aria-label={t('common.edit', 'Edit')}
            >
              <Pencil className="h-4 w-4" />
            </Button>
          )}
        </div>
        <CardDescription>
          {t(
            'settings.vision.subtitle',
            'Default Vision LLM (VLM) for PDF page extraction — not an embedding model. Text embeddings stay on the embedding stack.',
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
          (() => {
            const resolved = effectiveVisionFromWorkspace(workspace);
            const hasOverride = Boolean(
              workspace.vision_llm_provider?.trim() ||
                workspace.vision_llm_model?.trim(),
            );
            return (
              <div className="flex items-center gap-3 p-3 bg-muted/50 rounded-lg">
                <ProviderIcon providerId={resolved.provider} />
                <div className="min-w-0 flex-1">
                  <div className="font-medium truncate">
                    {hasOverride
                      ? workspace.vision_llm_model
                      : t(
                          'settings.vision.serverDefaultWithValue',
                          'Server Default ({{value}})',
                          { value: `${resolved.provider}/${resolved.model}` },
                        )}
                  </div>
                  <div className="text-sm text-muted-foreground capitalize truncate">
                    {hasOverride
                      ? t('workspace.workspaceOverride', 'Workspace override')
                      : t('workspace.resolvesVia', 'Resolves via {{source}}', {
                          source: resolved.source,
                        })}
                  </div>
                </div>
                <Badge
                  variant="outline"
                  className="ml-auto font-mono text-xs"
                  data-testid="settings-vision-resolves-to"
                  title={`source=${resolved.source}`}
                >
                  {t('settings.pdfParser.resolvesTo', 'Resolves to {{value}}', {
                    value: `${resolved.provider}/${resolved.model}`,
                  })}
                </Badge>
              </div>
            );
          })()
        ) : (
          <p className="text-sm text-muted-foreground">
            {t('settings.vision.noVisionModelDesc', 'Configure a vision model to enable image-aware document processing')}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
