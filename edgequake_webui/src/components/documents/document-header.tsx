/**
 * @module DocumentHeader
 * @description Header section for document management page.
 * Extracted from DocumentManager for SRP compliance (OODA-23).
 *
 * SPEC-099 LAW-099-5: Clear All is demoted to overflow (not peer of Refresh).
 *
 * @implements FEAT0001 - Document management header
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { AlertTriangle, Clock, Loader2, MoreHorizontal, RefreshCw, Trash2 } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { Document } from '@/types';
import type { WorkspacePresetId } from '@/lib/documents/workspace-layout';
import { ClearDocumentsDialog } from './clear-documents-dialog';
import { ConnectionBanner } from './connection-banner';
import { ConnectionStatus } from './connection-status';
import { WorkspaceLayoutMenu } from './workspace/workspace-layout-menu';
import { PipelineStatusDialog } from './pipeline-status-dialog';
import { ReprocessFailedButton } from './reprocess-failed-button';

/**
 * Props for DocumentHeader component.
 */
export interface DocumentHeaderProps {
  /** Total document count */
  totalCount: number;
  /** Optional overflow honesty label (e.g. "17 of 240") */
  countLabel?: string;
  /** Number of failed documents */
  failedCount: number;
  /** Show pipeline status shortcut in header */
  showPipelineIndicator: boolean;
  /**
   * SPEC-099 CLS: keep Working pill geometry reserved while feedback zone is
   * expected (skeleton) so the header does not grow when live work paints.
   */
  reservePipelineSlot?: boolean;
  /** Ingestion alert mode for header shortcut styling */
  pipelineAlertMode?: 'working' | 'queued' | 'stuck' | 'mixed';
  /** Active working document count (for Working · N pill) */
  activeDocCount?: number;
  /** Queued / waiting document count (Working · W · Queued · Q) */
  waitingDocCount?: number;
  /** @deprecated Use pipelineAlertMode */
  pipelineWaitingOnly: boolean;
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
  /** Documents for pipeline waiting-state details */
  documents?: Document[];
  /** Optional columns control slot (e.g. show Cost) */
  columnsMenu?: ReactNode;
  /** SPEC-155 docking workspace layout menu. */
  layoutPresetId?: WorkspacePresetId | null;
  onApplyLayoutPreset?: (id: WorkspacePresetId) => void;
  onResetLayout?: () => void;
}

/**
 * Document management page header with status and actions.
 */
export function DocumentHeader({
  totalCount,
  countLabel,
  failedCount,
  showPipelineIndicator,
  reservePipelineSlot = false,
  pipelineAlertMode,
  activeDocCount,
  waitingDocCount,
  pipelineWaitingOnly,
  pipelineDialogOpen,
  onPipelineDialogChange,
  onRefresh,
  tenantId,
  workspaceId,
  documents,
  columnsMenu,
  layoutPresetId = null,
  onApplyLayoutPreset,
  onResetLayout,
}: DocumentHeaderProps) {
  const { t } = useTranslation();
  const [clearAllOpen, setClearAllOpen] = useState(false);
  const alertMode = pipelineAlertMode ?? (pipelineWaitingOnly ? 'queued' : 'working');
  const working = activeDocCount ?? 0;
  const queued = waitingDocCount ?? 0;

  const pipelineButtonClass =
    alertMode === 'stuck'
      ? 'text-rose-700 border-rose-300 hover:bg-rose-50 dark:hover:bg-rose-950/40'
      : alertMode === 'queued' || (working === 0 && queued > 0)
        ? 'text-amber-700 border-amber-300 hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-950/40'
        : 'text-sky-700 border-sky-300 hover:bg-sky-50 dark:text-sky-300 dark:border-sky-800 dark:hover:bg-sky-950/40';

  const pipelineButtonLabel =
    alertMode === 'stuck'
      ? t('pipeline.stuckBadge', 'Needs attention')
      : working > 0 && queued > 0
        ? t('pipeline.workingAndQueued', 'Working · {{working}} · Queued · {{queued}}', {
            working,
            queued,
          })
        : working > 0
          ? t('pipeline.workingCount', 'Working · {{count}}', {
              count: working,
            })
          : queued > 0
            ? t('pipeline.queuedCount', 'Queued · {{count}}', { count: queued })
            : t('pipeline.busy', 'Working');

  const badgeText = countLabel ?? (totalCount > 0 ? String(totalCount) : null);

  return (
    <>
      <ConnectionBanner />
      
      <header className="eq-toolbar justify-between gap-3">
        <div className="space-y-0.5">
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold leading-tight tracking-tight">{t('documents.title')}</h1>
            {badgeText && (
              <Badge
                variant="secondary"
                className="min-w-[2.25rem] justify-center text-xs font-normal tabular-nums"
                data-testid="spec099-documents-count"
              >
                {badgeText}
              </Badge>
            )}
            <ConnectionStatus compact={true} />
          </div>
          <p className="text-[0.8125rem] text-muted-foreground [@media(max-height:820px)]:hidden">
            {t('documents.subtitle')}
          </p>
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          {(showPipelineIndicator || reservePipelineSlot) && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => onPipelineDialogChange(true)}
              className={`${pipelineButtonClass} min-w-0 max-w-full ${
                showPipelineIndicator
                  ? ''
                  : 'invisible pointer-events-none'
              }`}
              tabIndex={showPipelineIndicator ? 0 : -1}
              aria-hidden={!showPipelineIndicator}
              data-testid="pipeline-header-button"
            >
              {alertMode === 'stuck' ? (
                <AlertTriangle className="h-4 w-4" />
              ) : alertMode === 'queued' ? (
                <Clock className="h-4 w-4" />
              ) : (
                <Loader2 className="h-4 w-4 animate-spin" />
              )}
              {pipelineButtonLabel}
            </Button>
          )}
          <PipelineStatusDialog
            open={pipelineDialogOpen}
            onOpenChange={onPipelineDialogChange}
            tenantId={tenantId}
            workspaceId={workspaceId}
            documents={documents}
          />
          
          <ReprocessFailedButton
            failedCount={failedCount}
            onReprocessStarted={() => {
              onPipelineDialogChange(true);
            }}
          />

          {onApplyLayoutPreset && onResetLayout ? (
            <WorkspaceLayoutMenu
              presetId={layoutPresetId}
              onApplyPreset={onApplyLayoutPreset}
              onReset={onResetLayout}
            />
          ) : null}
        
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={onRefresh}
            data-testid="documents-refresh-button"
            aria-label={t('documents.refresh')}
            title={t('documents.refresh')}
          >
            <RefreshCw className="h-4 w-4" />
          </Button>

          {/* SPEC-099: Clear All demoted to overflow — not peer of Refresh */}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                aria-label={t('documents.moreActions', 'More actions')}
                data-testid="spec099-documents-overflow"
              >
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-56">
              {columnsMenu}
              {columnsMenu ? <DropdownMenuSeparator /> : null}
              <DropdownMenuLabel className="text-xs text-muted-foreground font-normal">
                {t('documents.dangerZone', 'Danger zone')}
              </DropdownMenuLabel>
              <button
                type="button"
                className="relative flex w-full cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none text-destructive focus:bg-destructive/10"
                data-testid="spec099-clear-all-menu-item"
                aria-label={t('documents.clearAll.button', 'Clear All')}
                onClick={() => setClearAllOpen(true)}
              >
                <Trash2 className="h-3.5 w-3.5" />
                {t('documents.clearAll.button', 'Clear All')}
              </button>
            </DropdownMenuContent>
          </DropdownMenu>
          <ClearDocumentsDialog
            documentCount={totalCount}
            onCleared={onRefresh}
            showTrigger={false}
            open={clearAllOpen}
            onOpenChange={setClearAllOpen}
          />
        </div>
      </header>
    </>
  );
}

export default DocumentHeader;
