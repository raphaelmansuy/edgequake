/**
 * @module DocumentHeader
 * @description Header section for document management page.
 * Extracted from DocumentManager for SRP compliance (OODA-23).
 * 
 * WHY: Header JSX was inline in DocumentManager causing bloat.
 * This component displays:
 * - Page title with document count badge
 * - WebSocket connection status
 * - Pipeline status button and dialog
 * - Reprocess failed button
 * - Refresh and clear buttons
 * 
 * @implements FEAT0001 - Document management header
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Loader2, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ClearDocumentsDialog } from './clear-documents-dialog';
import { ConnectionBanner } from './connection-banner';
import { ConnectionStatus } from './connection-status';
import { PipelineStatusDialog } from './pipeline-status-dialog';
import { ReprocessFailedButton } from './reprocess-failed-button';

/**
 * Props for DocumentHeader component.
 */
export interface DocumentHeaderProps {
  /** Total document count */
  totalCount: number;
  /** Number of failed documents */
  failedCount: number;
  /** Whether pipeline is busy */
  pipelineIsBusy: boolean;
  /** Whether pipeline dialog is open */
  pipelineDialogOpen: boolean;
  /** Handler to set pipeline dialog state */
  onPipelineDialogChange: (open: boolean) => void;
  /** Handler to refresh documents */
  onRefresh: () => void;
  /** Tenant ID for pipeline dialog */
  tenantId?: string;
  /** Workspace ID for pipeline dialog */
  workspaceId?: string;
}

/**
 * Document management page header with status and actions.
 */
export function DocumentHeader({
  totalCount,
  failedCount,
  pipelineIsBusy,
  pipelineDialogOpen,
  onPipelineDialogChange,
  onRefresh,
  tenantId,
  workspaceId,
}: DocumentHeaderProps) {
  const { t } = useTranslation();

  return (
    <>
      {/* OODA-02: Connection status banner when disconnected */}
      <ConnectionBanner />
      
      {/* Header - Compact */}
      <header className="flex items-center justify-between gap-3 flex-wrap">
        <div className="space-y-0.5">
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold tracking-tight">{t('documents.title')}</h1>
            {/* OODA-39: Document count badge */}
            {totalCount > 0 && (
              <Badge variant="secondary" className="text-xs font-normal">
                {totalCount}
              </Badge>
            )}
            {/* OODA-30: WebSocket connection status indicator */}
            <ConnectionStatus compact={true} />
          </div>
          <p className="text-sm text-muted-foreground">
            {t('documents.subtitle')}
          </p>
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          {/* Pipeline Status */}
          {pipelineIsBusy && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => onPipelineDialogChange(true)}
              className="gap-1 text-orange-500"
            >
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('pipeline.busy')}
            </Button>
          )}
          <PipelineStatusDialog
            open={pipelineDialogOpen}
            onOpenChange={onPipelineDialogChange}
            tenantId={tenantId}
            workspaceId={workspaceId}
          />
          
          {/* Reprocess Failed Button (GAP-UI-002) */}
          <ReprocessFailedButton
            failedCount={failedCount}
            onReprocessStarted={() => {
              onPipelineDialogChange(true);
            }}
          />
        
          <Button variant="outline" size="sm" onClick={onRefresh}>
            <RefreshCw className="h-4 w-4 mr-1" />
            {t('documents.refresh')}
          </Button>
          
          {/* Clear Documents Dialog (GAP-UI-009) */}
          <ClearDocumentsDialog
            documentCount={totalCount}
            onCleared={onRefresh}
          />
        </div>
      </header>
    </>
  );
}

export default DocumentHeader;
