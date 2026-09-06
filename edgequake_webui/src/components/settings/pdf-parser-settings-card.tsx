'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { getWorkspace, updateWorkspace } from '@/lib/api/edgequake';
import { getWorkspacePdfParserBackend } from '@/lib/workspace/drafts';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Gauge, Pencil, Save, X } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import {
  PdfParserBackendField,
  type PdfParserBackendChoice,
} from './pdf-parser-backend-field';

export function PdfParserSettingsCard() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  const [isEditing, setIsEditing] = useState(false);
  const [backend, setBackend] = useState<PdfParserBackendChoice>('none');

  const { data: workspace, isLoading } = useQuery({
    queryKey: ['workspace', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getWorkspace(selectedTenantId!, selectedWorkspaceId!),
    enabled: !!selectedTenantId && !!selectedWorkspaceId,
    staleTime: 60000,
    retry: 1,
  });

  const updateMutation = useMutation({
    mutationFn: () =>
      updateWorkspace(selectedTenantId!, selectedWorkspaceId!, {
        pdf_parser_backend: backend === 'none' ? 'none' : backend,
      }),
    onSuccess: () => {
      toast.success(
        t('settings.pdfParser.updateSuccess', 'PDF parser default updated'),
      );
      queryClient.invalidateQueries({
        queryKey: ['workspace', selectedTenantId, selectedWorkspaceId],
      });
      setIsEditing(false);
    },
    onError: (error) => {
      toast.error(
        t('settings.pdfParser.updateFailed', 'Failed to update PDF parser default'),
        {
          description: error instanceof Error ? error.message : 'Unknown error',
        },
      );
    },
  });

  if (!selectedTenantId || !selectedWorkspaceId) {
    return null;
  }

  return (
    <Card>
      <CardHeader className="pb-4">
        <div className="flex flex-wrap items-center justify-between gap-2 min-w-0">
          <div className="flex items-center gap-2 min-w-0">
            <Gauge className="h-5 w-5 text-amber-600 shrink-0" />
            <CardTitle className="truncate">{t('settings.pdfParser.title', 'PDF Parser')}</CardTitle>
          </div>
          {!isEditing && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setBackend(getWorkspacePdfParserBackend(workspace));
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
            'settings.pdfParser.subtitle',
            'Choose the default PDF extraction backend for this workspace. EdgeParse is faster and deterministic; Vision is better for scanned or image-heavy PDFs.',
          )}
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        {isLoading ? (
          <Skeleton className="h-14 w-full" />
        ) : isEditing ? (
          <>
            <PdfParserBackendField
              value={backend}
              isEditing
              onChange={setBackend}
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
                onClick={() => {
                  setBackend(getWorkspacePdfParserBackend(workspace));
                  setIsEditing(false);
                }}
                disabled={updateMutation.isPending}
              >
                <X className="h-4 w-4 mr-2" />
                {t('common.cancel', 'Cancel')}
              </Button>
            </div>
          </>
        ) : workspace ? (
          <PdfParserBackendField
            value={getWorkspacePdfParserBackend(workspace)}
            isEditing={false}
            onChange={setBackend}
          />
        ) : null}
      </CardContent>
    </Card>
  );
}
