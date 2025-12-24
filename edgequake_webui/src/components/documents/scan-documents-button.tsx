'use client';

import { Button } from '@/components/ui/button';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { scanDocuments } from '@/lib/api/edgequake';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { FolderSearch, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface ScanDocumentsButtonProps {
  /**
   * Callback when scan is started successfully.
   * @param trackId The track ID for monitoring scan progress
   */
  onScanStarted?: (trackId: string) => void;
  /**
   * Whether to show a compact version (icon only)
   */
  compact?: boolean;
  /**
   * Variant for the button style
   */
  variant?: 'default' | 'outline' | 'ghost';
  /**
   * Size of the button
   */
  size?: 'default' | 'sm' | 'lg' | 'icon';
}

/**
 * Button component to trigger scanning of the input directory for new documents.
 * Connects to POST /api/v1/documents/scan endpoint.
 */
export function ScanDocumentsButton({
  onScanStarted,
  compact = false,
  variant = 'outline',
  size = 'sm',
}: ScanDocumentsButtonProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const scanMutation = useMutation({
    mutationFn: () => scanDocuments(undefined),
    onSuccess: (data) => {
      toast.success(
        t('documents.scan.started', 'Scan started'),
        {
          description: data.message || t('documents.scan.startedDesc', 'Scanning input directory for new documents...'),
          duration: 5000,
          action: {
            label: t('documents.viewStatus', 'View Status'),
            onClick: () => {
              // The parent component can handle showing the pipeline status dialog
            },
          },
        }
      );
      // Refresh documents list
      queryClient.invalidateQueries({ queryKey: ['documents'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-status'] });
      // Notify parent
      onScanStarted?.(data.track_id);
    },
    onError: (error) => {
      toast.error(
        t('documents.scan.failed', 'Scan failed'),
        {
          description: error instanceof Error ? error.message : t('common.unknownError', 'An error occurred'),
          action: {
            label: t('common.retry', 'Retry'),
            onClick: () => scanMutation.mutate(),
          },
        }
      );
    },
  });

  const handleScan = () => {
    scanMutation.mutate();
  };

  const button = (
    <Button
      variant={variant}
      size={size}
      onClick={handleScan}
      disabled={scanMutation.isPending}
    >
      {scanMutation.isPending ? (
        <Loader2 className={`h-4 w-4 ${compact ? '' : 'mr-2'} animate-spin`} />
      ) : (
        <FolderSearch className={`h-4 w-4 ${compact ? '' : 'mr-2'}`} />
      )}
      {!compact && (
        scanMutation.isPending
          ? t('documents.scan.scanning', 'Scanning...')
          : t('documents.scan.scanDirectory', 'Scan Directory')
      )}
    </Button>
  );

  if (compact) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            {button}
          </TooltipTrigger>
          <TooltipContent>
            <p>{t('documents.scan.scanDirectory', 'Scan Directory')}</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  return button;
}
