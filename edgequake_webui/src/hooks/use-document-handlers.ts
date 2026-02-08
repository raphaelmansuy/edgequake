"use client";

import type { Document } from "@/types";
import { useRouter } from "next/navigation";
import { useCallback } from "react";

/**
 * OODA-28: Document event handlers hook
 *
 * WHY: Single Responsibility Principle - isolate navigation and dialog handlers
 * from DocumentManager UI state management.
 *
 * Handlers:
 * - handleDocumentClick: Select document for preview panel
 * - handleDocumentDoubleClick: Navigate to document detail page
 * - handleViewDetails: Navigate to document detail page (button action)
 * - handlePreviewClose: Close preview panel and clear selection
 * - handleViewInGraph: Navigate to graph view with entity filter
 * - handleViewPdf: Open PDF viewer dialog or fallback to detail page
 */

export interface UseDocumentHandlersOptions {
  /** Callback to set selected document */
  setSelectedDocument: (doc: Document | null) => void;
  /** Callback to set preview panel open state */
  setPreviewPanelOpen: (open: boolean) => void;
  /** Callback to set viewer dialog open state */
  setViewerDialogOpen: (open: boolean) => void;
  /** Callback to set viewer PDF ID */
  setViewerPdfId: (id: string | null) => void;
}

export interface UseDocumentHandlersReturn {
  /** Select document for preview panel */
  handleDocumentClick: (doc: Document) => void;
  /** Navigate to document detail page (double-click) */
  handleDocumentDoubleClick: (doc: Document) => void;
  /** Navigate to document detail page (button action) */
  handleViewDetails: (doc: Document) => void;
  /** Close preview panel and clear selection */
  handlePreviewClose: () => void;
  /** Navigate to graph view with entity filter */
  handleViewInGraph: (doc: Document) => void;
  /** Open PDF viewer dialog or fallback to detail page */
  handleViewPdf: (doc: Document) => void;
}

export function useDocumentHandlers({
  setSelectedDocument,
  setPreviewPanelOpen,
  setViewerDialogOpen,
  setViewerPdfId,
}: UseDocumentHandlersOptions): UseDocumentHandlersReturn {
  const router = useRouter();

  /** Select document for preview panel */
  const handleDocumentClick = useCallback(
    (doc: Document) => {
      setSelectedDocument(doc);
      setPreviewPanelOpen(true);
    },
    [setSelectedDocument, setPreviewPanelOpen],
  );

  /**
   * OODA-41: Double-click to navigate to document detail page
   * WHY: Power users expect double-click for primary navigation action
   * SPEC-002: Navigate to dedicated document detail page, not dialog
   */
  const handleDocumentDoubleClick = useCallback(
    (doc: Document) => {
      router.push(`/documents/${doc.id}`);
    },
    [router],
  );

  /**
   * OODA-41: Navigate to document detail page (for View Details button)
   * WHY: Users need explicit link to dedicated document view
   */
  const handleViewDetails = useCallback(
    (doc: Document) => {
      router.push(`/documents/${doc.id}`);
    },
    [router],
  );

  /** Close preview panel and clear selection */
  const handlePreviewClose = useCallback(() => {
    setSelectedDocument(null);
    setPreviewPanelOpen(false);
  }, [setSelectedDocument, setPreviewPanelOpen]);

  /** Navigate to graph view with entity filter */
  const handleViewInGraph = useCallback(
    (doc: Document) => {
      router.push(`/graph?entity=${encodeURIComponent(doc.id)}`);
    },
    [router],
  );

  /**
   * SPEC-002: Open PDF viewer dialog for PDF documents
   * WHY: Users need to view original PDF alongside extracted markdown
   */
  const handleViewPdf = useCallback(
    (doc: Document) => {
      // Use pdf_id if available, otherwise try to derive from source_type
      const pdfId = doc.pdf_id || (doc.source_type === "pdf" ? doc.id : null);
      if (pdfId) {
        setViewerPdfId(pdfId);
        setViewerDialogOpen(true);
      } else {
        // Fallback to standard document view
        router.push(`/documents/${doc.id}`);
      }
    },
    [router, setViewerPdfId, setViewerDialogOpen],
  );

  return {
    handleDocumentClick,
    handleDocumentDoubleClick,
    handleViewDetails,
    handlePreviewClose,
    handleViewInGraph,
    handleViewPdf,
  };
}
