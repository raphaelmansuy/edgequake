/**
 * @module useDocumentKeyboard
 * @description Document-specific keyboard shortcuts for management interface.
 * Extracted from DocumentManager for SRP compliance (OODA-18).
 *
 * WHY: Keyboard event handling was inline in DocumentManager.
 * This hook:
 * - Handles Escape (close preview/clear selection)
 * - Handles Ctrl/Cmd+A (select all)
 * - Handles R (refresh)
 * - Skips shortcuts when in input/textarea/contentEditable
 *
 * @implements FEAT0605 - Keyboard accessibility
 */
"use client";

import type { TFunction } from "i18next";
import { useEffect } from "react";
import { toast } from "sonner";

/**
 * Options for useDocumentKeyboard hook.
 */
export interface UseDocumentKeyboardOptions {
  /** Whether the preview panel is currently open */
  previewPanelOpen: boolean;
  /** Number of selected items */
  selectedCount: number;
  /** Handler to close the preview panel */
  onPreviewClose: () => void;
  /** Handler to select/deselect all items */
  onSelectAll: (selected: boolean) => void;
  /** Handler to clear selection */
  onClearSelection: () => void;
  /** Handler to refresh documents */
  onRefresh: () => void;
  /** i18n translation function */
  t: TFunction;
}

/**
 * Hook for document management keyboard shortcuts.
 *
 * Shortcuts:
 * - Escape: Close preview panel or clear selection
 * - Ctrl/Cmd + A: Select all documents
 * - R: Refresh documents
 *
 * @example
 * ```tsx
 * useDocumentKeyboard({
 *   previewPanelOpen,
 *   selectedCount,
 *   onPreviewClose: handlePreviewClose,
 *   onSelectAll: handleSelectAll,
 *   onClearSelection: handleClearSelection,
 *   onRefresh: refetch,
 *   t,
 * });
 * ```
 */
export function useDocumentKeyboard(options: UseDocumentKeyboardOptions): void {
  const {
    previewPanelOpen,
    selectedCount,
    onPreviewClose,
    onSelectAll,
    onClearSelection,
    onRefresh,
    t,
  } = options;

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if in input field or textarea
      const target = e.target as HTMLElement;
      const tagName = target.tagName.toUpperCase();
      if (
        tagName === "INPUT" ||
        tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }

      // Escape: Clear selection or close preview panel
      if (e.key === "Escape") {
        if (previewPanelOpen) {
          onPreviewClose();
        } else if (selectedCount > 0) {
          onClearSelection();
        }
        return;
      }

      // Ctrl/Cmd + A: Select all documents
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "a") {
        e.preventDefault(); // Prevent browser select all
        onSelectAll(true);
        return;
      }

      // R: Refresh documents (single key, no modifier)
      if (
        e.key.toLowerCase() === "r" &&
        !e.metaKey &&
        !e.ctrlKey &&
        !e.altKey
      ) {
        onRefresh();
        toast.info(
          t("documents.refresh.triggered", "Refreshing documents..."),
          { duration: 1000 },
        );
        return;
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [
    previewPanelOpen,
    selectedCount,
    onPreviewClose,
    onSelectAll,
    onClearSelection,
    onRefresh,
    t,
  ]);
}

export default useDocumentKeyboard;
