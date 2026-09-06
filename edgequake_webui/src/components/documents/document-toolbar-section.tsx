'use client';

import type { StatusCounts } from '@/hooks/use-document-filtering';
import {
    buildIngestionRunViews,
    selectPrimaryRun,
} from '@/lib/pipeline/ingestion-run-view';
import {
    resolvePipelineUiState,
    type PipelineUiState,
} from '@/lib/pipeline/pipeline-document-state';
import type { Document, PipelineStatus } from '@/types';
import { useMemo } from 'react';
import { BatchActionsBar } from './batch-actions-bar';
import { DocumentDropzone, type DocumentDropzoneProps } from './document-dropzone';
import type { DocStatus, SortField, ClassificationFilter } from './document-filters';
import { DocumentFilters } from './document-filters';
import { DocumentSearchBar } from './document-search-bar';
import { ProcessingStatusSummary } from './processing-status-summary';

export interface DocumentToolbarSectionProps {
  // Search
  searchQuery: string;
  onSearchChange: (value: string) => void;
  
  // Filters
  statusFilter: DocStatus;
  onStatusFilterChange: (value: DocStatus) => void;
  sortField: SortField;
  onSortFieldChange: (value: SortField) => void;
  sortDirection: 'asc' | 'desc';
  onSortDirectionChange: (value: 'asc' | 'desc') => void;
  statusCounts: StatusCounts;
  classificationFilter?: ClassificationFilter;
  onClassificationFilterChange?: (value: ClassificationFilter) => void;
  showClassificationFilter?: boolean;
  
  // Pipeline status
  pipelineStatus: PipelineStatus | undefined;
  /** SPEC-099: shared resolve from shell — avoids dual busy detection */
  pipelineUi?: PipelineUiState;
  documents: Document[];
  onOpenPipelineDetails: () => void;
  onReprocessStuckDocuments?: (documents: Document[]) => void;
  isReprocessingStuck?: boolean;
  /**
   * When the unified feedback zone already narrates the same runs, hide the
   * non-stuck processing banner to avoid duplicate headlines. Stuck banner
   * (CTA) always stays visible.
   */
  demotePipelineBanner?: boolean;
  /**
   * SPEC-099: collapse upload slot when feedback zone has live work.
   */
  collapseUploadSlot?: boolean;
  
  // Dropzone
  getRootProps: DocumentDropzoneProps['getRootProps'];
  getInputProps: DocumentDropzoneProps['getInputProps'];
  isDragActive: boolean;
  openFileDialog: () => void;
  pdfParserBackend: DocumentDropzoneProps['pdfParserBackend'];
  onPdfParserBackendChange: DocumentDropzoneProps['onPdfParserBackendChange'];
  /** Workspace inherit label — Vision / EdgeParse resolved default. */
  workspacePdfParserBackend?: DocumentDropzoneProps['workspacePdfParserBackend'];
  visionReasoningEffort?: string;
  onVisionReasoningEffortChange?: (value: string | undefined) => void;
  visionExtract?: DocumentDropzoneProps['visionExtract'];
  onVisionExtractChange?: DocumentDropzoneProps['onVisionExtractChange'];
  visionProvider?: string | null;
  visionModel?: string | null;
  securityFields?: DocumentDropzoneProps['securityFields'];
  onSecurityFieldsChange?: DocumentDropzoneProps['onSecurityFieldsChange'];
  showSecurityFields?: boolean;
  onSecurityExpandedChange?: DocumentDropzoneProps['onSecurityExpandedChange'];

  // Bulk actions
  selectedCount: number;
  onBulkReprocess: () => void;
  onBulkDelete: () => void;
  onClearSelection: () => void;
}

export function DocumentToolbarSection({
  searchQuery,
  onSearchChange,
  statusFilter,
  onStatusFilterChange,
  sortField,
  onSortFieldChange,
  sortDirection,
  onSortDirectionChange,
  statusCounts,
  classificationFilter = 'all',
  onClassificationFilterChange,
  showClassificationFilter = false,
  pipelineStatus,
  pipelineUi: pipelineUiProp,
  documents,
  onOpenPipelineDetails,
  onReprocessStuckDocuments,
  isReprocessingStuck,
  demotePipelineBanner = false,
  collapseUploadSlot = false,
  getRootProps,
  getInputProps,
  isDragActive,
  openFileDialog,
  pdfParserBackend,
  onPdfParserBackendChange,
  workspacePdfParserBackend,
  visionReasoningEffort,
  onVisionReasoningEffortChange,
  visionExtract,
  onVisionExtractChange,
  visionProvider,
  visionModel,
  securityFields,
  onSecurityFieldsChange,
  showSecurityFields,
  onSecurityExpandedChange,
  selectedCount,
  onBulkReprocess,
  onBulkDelete,
  onClearSelection,
}: DocumentToolbarSectionProps) {
  const runViews = useMemo(
    () => buildIngestionRunViews(documents),
    [documents],
  );
  const primaryRun = useMemo(() => selectPrimaryRun(runViews), [runViews]);
  const pipelineUiFallback = useMemo(
    () =>
      resolvePipelineUiState(
        documents,
        pipelineStatus ?? {
          is_busy: Boolean(primaryRun && primaryRun.stageStatus === 'active'),
          running_tasks: primaryRun?.stageStatus === 'active' ? 1 : 0,
          queued_tasks: primaryRun?.stageStatus === 'pending' ? 1 : 0,
          completed_tasks: 0,
          failed_tasks: 0,
          tasks: [],
        },
      ),
    [documents, pipelineStatus, primaryRun],
  );
  const pipelineUi = pipelineUiProp ?? pipelineUiFallback;
  // Hide chrome once every document is terminal (ignore stale pipelineStatus).
  // Demote non-stuck banners when the feedback zone already shows the same runs.
  const showBanner =
    pipelineUi.showPipelineIndicator &&
    (pipelineUi.isStuck || !demotePipelineBanner);
  const quietDropzone =
    pipelineUi.isActivelyProcessing ||
    primaryRun?.stageStatus === 'active';

  const selectionMode = selectedCount > 0;

  return (
    <>
      {/* SPEC-099 F-099-16: selection replaces primary toolbar row (not stacked) */}
      {selectionMode ? (
        <div
          className="pb-3 border-b"
          data-testid="spec099-selection-toolbar"
        >
          <BatchActionsBar
            selectedCount={selectedCount}
            onReprocess={onBulkReprocess}
            onDelete={onBulkDelete}
            onClear={onClearSelection}
          />
        </div>
      ) : (
        <div
          className="flex flex-col sm:flex-row sm:items-center gap-3 pb-3 border-b"
          data-testid="spec099-primary-toolbar"
        >
          <DocumentSearchBar
            value={searchQuery}
            onChange={onSearchChange}
          />
          <DocumentFilters
            status={statusFilter}
            onStatusChange={onStatusFilterChange}
            sortField={sortField}
            onSortFieldChange={onSortFieldChange}
            sortDirection={sortDirection}
            onSortDirectionChange={onSortDirectionChange}
            statusCounts={statusCounts}
            classification={classificationFilter}
            onClassificationChange={onClassificationFilterChange}
            showClassificationFilter={showClassificationFilter}
          />
        </div>
      )}

      {/* Processing Status Summary — stuck CTA always; otherwise demote when zone owns narrative */}
      {showBanner && (
        <ProcessingStatusSummary
          pipelineStatus={
            pipelineStatus ?? {
              is_busy: Boolean(primaryRun && primaryRun.stageStatus === 'active'),
              running_tasks: primaryRun?.stageStatus === 'active' ? 1 : 0,
              queued_tasks: primaryRun?.stageStatus === 'pending' ? 1 : 0,
              completed_tasks: 0,
              failed_tasks: 0,
              tasks: [],
            }
          }
          documents={documents}
          onOpenDetails={onOpenPipelineDetails}
          onReprocessStuck={onReprocessStuckDocuments}
          isReprocessing={isReprocessingStuck}
        />
      )}

      {/* Compact Upload Zone — quieter while a run is active (SPEC-048);
          collapsed when feedback zone owns live narrative (SPEC-099) */}
      <DocumentDropzone
        getRootProps={getRootProps}
        getInputProps={getInputProps}
        isDragActive={isDragActive}
        openFileDialog={openFileDialog}
        pdfParserBackend={pdfParserBackend}
        onPdfParserBackendChange={onPdfParserBackendChange}
        workspacePdfParserBackend={workspacePdfParserBackend}
        visionReasoningEffort={visionReasoningEffort}
        onVisionReasoningEffortChange={onVisionReasoningEffortChange}
        visionExtract={visionExtract}
        onVisionExtractChange={onVisionExtractChange}
        visionProvider={visionProvider}
        visionModel={visionModel}
        securityFields={securityFields}
        onSecurityFieldsChange={onSecurityFieldsChange}
        showSecurityFields={showSecurityFields}
        onSecurityExpandedChange={onSecurityExpandedChange}
        quiet={quietDropzone && !collapseUploadSlot}
        collapsed={collapseUploadSlot}
      />
    </>
  );
}
