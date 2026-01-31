/**
 * @module documents/index
 * @description Barrel export for document management components.
 *
 * @implements OODA-31: Component barrel exports
 *
 * Components are organized by function:
 * - Core: DocumentManager (main entry point)
 * - Progress: PdfUploadProgress, BatchProgressCard, IngestionProgressPanel
 * - History: UploadHistory
 * - Status: ConnectionStatus, StatusBadge, ErrorBanner
 * - Dialogs: PipelineStatusDialog, ClearDocumentsDialog, DocumentDetailDialog
 * - Controls: DocumentFilters, PaginationControls, ReprocessFailedButton
 * - Display: CostBadge, CostCell, ErrorMessagePopover
 */

// Core component
export { DocumentManager } from './document-manager';

// Progress tracking components
export { PdfUploadProgress } from './pdf-upload-progress';
export { BatchProgressCard } from './batch-progress-card';
export { IngestionProgressPanel } from './ingestion-progress-panel';

// History and status
export { UploadHistory } from './upload-history';
export { ConnectionStatus } from './connection-status';
export { StatusBadge } from './status-badge';
export { ErrorBanner, type PdfError, type ErrorSeverity } from './error-banner';

// Dialogs
export { PipelineStatusDialog } from './pipeline-status-dialog';
export { ClearDocumentsDialog } from './clear-documents-dialog';
export { DocumentDetailDialog } from './document-detail-dialog';
export { DocumentPreviewPanel } from './document-preview-panel';

// Controls and filters
export { DocumentFilters } from './document-filters';
export { PaginationControls } from './pagination-controls';
export { ReprocessFailedButton } from './reprocess-failed-button';
export { ResetDocumentStatusButton } from './reset-document-status-button';
export { ScanDocumentsButton } from './scan-documents-button';

// Display components
export { CostBadge } from './cost-badge';
export { CostCell } from './cost-cell';
export { ErrorMessagePopover } from './error-message-popover';
export { FailedChunksCard } from './failed-chunks-card';

// Types
export type { UploadingFile } from './types';
