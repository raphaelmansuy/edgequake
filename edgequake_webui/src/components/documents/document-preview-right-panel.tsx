/**
 * @module DocumentPreviewRightPanel
 * @description Right panel wrapper for document preview.
 * Extracted from DocumentManager for SRP compliance (OODA-27).
 * 
 * WHY: Right panel JSX was inline in DocumentManager.
 * This component wraps RightPanel with DocumentPreviewPanel.
 * 
 * @implements FEAT0402 - Document preview sidebar
 */
'use client';

import { RightPanel } from '@/components/layout/right-panel';
import type { Document } from '@/types';
import { FileText } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { DocumentPreviewPanel } from './document-preview-panel';
import { DocumentViewerDialog } from './document-viewer-dialog';

/**
 * Props for DocumentPreviewRightPanel component.
 */
export interface DocumentPreviewRightPanelProps {
  /** Whether the panel is open */
  isOpen: boolean;
  /** Handler to toggle panel state */
  onToggle: () => void;
  /** Handler to close panel */
  onClose: () => void;
  /** Selected document for preview */
  selectedDocument: Document | null;
  /** Handler for delete action */
  onDelete: (id: string) => void;
  /** Handler for reprocess action */
  onReprocess: (id: string) => void;
  /** Handler for view in graph action */
  onViewInGraph: (doc: Document) => void;
  /** Handler for view full document action */
  onViewFull: (doc: Document) => void;
  /** Whether delete is in progress */
  isDeleting: boolean;
  /** Whether reprocess is in progress */
  isReprocessing: boolean;
  /** Whether viewer dialog is open */
  viewerDialogOpen: boolean;
  /** Handler for viewer dialog state */
  onViewerDialogChange: (open: boolean) => void;
  /** PDF ID for viewer dialog */
  viewerPdfId: string | null;
}

/**
 * Right panel for document preview with viewer dialog.
 */
export function DocumentPreviewRightPanel({
  isOpen,
  onToggle,
  onClose,
  selectedDocument,
  onDelete,
  onReprocess,
  onViewInGraph,
  onViewFull,
  isDeleting,
  isReprocessing,
  viewerDialogOpen,
  onViewerDialogChange,
  viewerPdfId,
}: DocumentPreviewRightPanelProps) {
  const { t } = useTranslation();

  const title = selectedDocument 
    ? (selectedDocument.title || selectedDocument.file_name || `Document ${selectedDocument.id.slice(0, 8)}`) 
    : t('documents.preview.title', 'Document Preview');
    
  const subtitle = selectedDocument?.id 
    ? `ID: ${selectedDocument.id.slice(0, 12)}...` 
    : undefined;

  return (
    <>
      <RightPanel
        isOpen={isOpen}
        onToggle={onToggle}
        onClose={onClose}
        title={title}
        subtitle={subtitle}
        width="wide"
        showCollapsedBar={true}
        collapsedLabel={t('documents.preview.panelLabel', 'Preview')}
        headerIcon={<FileText className="h-4 w-4" />}
        resizable={true}
        defaultWidth={480}
        minWidth={400}
        maxWidth={900}
        storageKey="document-preview-panel-width"
      >
        <DocumentPreviewPanel
          document={selectedDocument}
          onDelete={(id) => {
            onDelete(id);
            onClose();
          }}
          onReprocess={onReprocess}
          onViewFull={onViewFull}
          onViewInGraph={onViewInGraph}
          isDeleting={isDeleting}
          isReprocessing={isReprocessing}
        />
      </RightPanel>

      {/* SPEC-002: PDF/Markdown Viewer Dialog */}
      <DocumentViewerDialog
        open={viewerDialogOpen}
        onOpenChange={onViewerDialogChange}
        pdfId={viewerPdfId}
      />
    </>
  );
}

export default DocumentPreviewRightPanel;
