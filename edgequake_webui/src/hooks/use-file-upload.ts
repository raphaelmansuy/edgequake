/**
 * useFileUpload - File upload state and handlers
 *
 * @fileoverview Extracted from DocumentManager (OODA-13)
 * WHY: SRP - Upload orchestration is a distinct responsibility
 *
 * @module edgequake_webui/hooks/use-file-upload
 */
"use client";

import type { UploadingFile } from "@/components/documents/types";
import {
  uploadDocument,
  uploadPdfDocument,
  type DocumentsListResult,
} from "@/lib/api/edgequake";
import type { Document } from "@/types";
import { useQueryClient } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

export interface UseFileUploadOptions {
  /** Tenant ID for multi-tenancy */
  tenantId?: string | null;
  /** Workspace ID for isolation */
  workspaceId?: string | null;
  /** Callback when upload starts (e.g., to switch filter) */
  onUploadStart?: () => void;
}

export interface UseFileUploadReturn {
  /** Files currently being uploaded with progress */
  uploadingFiles: UploadingFile[];
  /** Whether any upload is in progress */
  isUploading: boolean;
  /** Upload files handler */
  handleFilesUpload: (files: File[]) => Promise<void>;
  /** Remove a file from upload list */
  removeUploadingFile: (index: number) => void;
  /** Mark upload as complete (for PdfUploadProgress) */
  handleUploadComplete: (index: number) => void;
  /** Mark upload as failed (for PdfUploadProgress) */
  handleUploadFailed: (index: number, error: string) => void;
}

/**
 * useFileUpload - Manages file upload state and orchestration
 *
 * Handles:
 * - Sequential file upload with progress tracking
 * - PDF vs text file routing
 * - Optimistic cache updates
 * - Duplicate detection
 * - Success/error toast notifications
 */
export function useFileUpload(
  options: UseFileUploadOptions = {},
): UseFileUploadReturn {
  const { tenantId, workspaceId, onUploadStart } = options;

  const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
  const [isUploading, setIsUploading] = useState(false);

  const queryClient = useQueryClient();
  const router = useRouter();
  const { t } = useTranslation();

  /**
   * Main upload handler with progress tracking
   * WHY: Process files sequentially for better feedback and error isolation
   */
  const handleFilesUpload = useCallback(
    async (files: File[]) => {
      if (files.length === 0) return;

      // Notify parent (e.g., to switch status filter)
      onUploadStart?.();

      setIsUploading(true);

      // Generate a shared track_id for this batch
      const trackId = `upload_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;

      // Initialize upload state for all files
      const initialFiles: UploadingFile[] = files.map((file) => ({
        file,
        progress: 0,
        status: "pending" as const,
        phase: "Waiting...",
      }));
      setUploadingFiles(initialFiles);

      // Show loading toast
      const toastId = toast.loading(
        t("documents.upload.inProgress", { count: files.length }) ||
          `Uploading ${files.length} file(s)...`,
        { duration: Infinity },
      );

      let successCount = 0;
      let errorCount = 0;

      // Process files sequentially for better feedback
      for (let i = 0; i < files.length; i++) {
        const file = files[i];

        // Phase 1: Reading file
        setUploadingFiles((prev) =>
          prev.map((f, idx) =>
            idx === i
              ? {
                  ...f,
                  status: "reading" as const,
                  progress: 10,
                  phase: t("documents.upload.reading", "Reading file..."),
                }
              : f,
          ),
        );

        try {
          // Phase 2: Uploading to server
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? {
                    ...f,
                    status: "uploading" as const,
                    progress: 40,
                    phase: t(
                      "documents.upload.uploading",
                      "Uploading to server...",
                    ),
                  }
                : f,
            ),
          );

          let response: {
            document_id?: string;
            pdf_id?: string;
            duplicate_of?: string;
            task_id?: string;
            track_id?: string;
          };

          // Check if file is PDF - route to PDF upload endpoint
          const isPdfFile = file.type === "application/pdf";

          if (isPdfFile) {
            // Upload PDF file directly (multipart/form-data)
            const pdfResponse = await uploadPdfDocument(file, {
              title: file.name,
              enable_vision: true, // Enable vision extraction by default for PDFs
              track_id: trackId,
            });

            response = {
              document_id: pdfResponse.document_id,
              pdf_id: pdfResponse.pdf_id,
              duplicate_of: pdfResponse.duplicate_of,
              task_id: pdfResponse.task_id,
              track_id: pdfResponse.track_id,
            };

            // Optimistic update for PDF upload
            // WHY: PDFs must appear immediately in documents panel
            // FIX: Use predicate-based filter for reliable query matching
            if (pdfResponse.pdf_id && !pdfResponse.duplicate_of) {
              const optimisticDoc: Document = {
                id: pdfResponse.pdf_id,
                title: file.name,
                file_name: file.name,
                file_size: file.size,
                source_type: "pdf",
                status: "processing",
                mime_type: "application/pdf",
                created_at: new Date().toISOString(),
                pdf_id: pdfResponse.pdf_id,
                track_id: pdfResponse.track_id,
                tenant_id: tenantId ?? undefined,
                workspace_id: workspaceId ?? undefined,
              };

              // Add to query cache for instant visibility
              // Use predicate to match ANY documents query regardless of pagination params
              queryClient.setQueriesData<DocumentsListResult>(
                { predicate: (query) => query.queryKey[0] === "documents" },
                (old) => {
                  if (!old || !old.items || !Array.isArray(old.items))
                    return old;
                  const exists = old.items.some(
                    (d) =>
                      d.pdf_id === pdfResponse.pdf_id ||
                      d.id === pdfResponse.pdf_id,
                  );
                  if (exists) return old;
                  return {
                    ...old,
                    items: [optimisticDoc, ...old.items],
                    total: (old.total ?? 0) + 1,
                  };
                },
              );
            }

            // Store track_id and isPdf flag for progress tracking
            setUploadingFiles((prev) =>
              prev.map((f, idx) =>
                idx === i
                  ? {
                      ...f,
                      trackId: pdfResponse.track_id,
                      isPdf: true,
                    }
                  : f,
              ),
            );
          } else {
            // Read text file content
            const text = await file.text();

            // Upload text document with async processing
            const textResponse = await uploadDocument({
              content: text,
              source_type: "text",
              title: file.name,
              async_processing: true,
              track_id: trackId,
            });

            response = textResponse;

            // Optimistic update for text/markdown files
            // FIX: Use predicate-based filter for reliable query matching
            if (textResponse.document_id && !textResponse.duplicate_of) {
              const optimisticDoc: Document = {
                id: textResponse.document_id,
                title: file.name,
                file_name: file.name,
                file_size: file.size,
                source_type: "text",
                status: "processing",
                mime_type: file.type || "text/plain",
                created_at: new Date().toISOString(),
                track_id: textResponse.track_id,
                tenant_id: tenantId ?? undefined,
                workspace_id: workspaceId ?? undefined,
              };

              queryClient.setQueriesData<DocumentsListResult>(
                { predicate: (query) => query.queryKey[0] === "documents" },
                (old) => {
                  if (!old || !old.items || !Array.isArray(old.items))
                    return old;
                  const exists = old.items.some(
                    (d) => d.id === textResponse.document_id,
                  );
                  if (exists) return old;
                  return {
                    ...old,
                    items: [optimisticDoc, ...old.items],
                    total: (old.total ?? 0) + 1,
                  };
                },
              );
            }
          }

          // Check for duplicate
          if (response.duplicate_of) {
            toast.warning(
              t(
                "documents.upload.duplicate",
                "{{name}} is a duplicate (existing: {{id}})",
                {
                  name: file.name,
                  id: response.duplicate_of.slice(0, 8),
                },
              ),
              { duration: 4000 },
            );

            // Mark as duplicate (treat as success with warning)
            setUploadingFiles((prev) =>
              prev.map((f, idx) =>
                idx === i
                  ? {
                      ...f,
                      status: "success" as const,
                      progress: 100,
                      phase: t(
                        "documents.upload.duplicateSkipped",
                        "Duplicate (skipped)",
                      ),
                    }
                  : f,
              ),
            );
            successCount++;
            continue;
          }

          // Phase 3: Extraction queued
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? {
                    ...f,
                    status: "extracting" as const,
                    progress: 80,
                    phase: response.task_id
                      ? t(
                          "documents.upload.queued",
                          "Queued for extraction (Task: {{taskId}})",
                          {
                            taskId: response.task_id.slice(0, 8),
                          },
                        )
                      : t("documents.upload.extracting", "Processing..."),
                  }
                : f,
            ),
          );

          // Brief delay to show extraction phase
          await new Promise((resolve) => setTimeout(resolve, 300));

          // Mark as complete
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? {
                    ...f,
                    status: "success" as const,
                    progress: 100,
                    phase: t("documents.upload.complete", "Complete!"),
                  }
                : f,
            ),
          );

          successCount++;
        } catch (error) {
          const errorMessage =
            error instanceof Error ? error.message : "Upload failed";
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? {
                    ...f,
                    status: "error" as const,
                    progress: 100,
                    error: errorMessage,
                    phase: t("common.failed", "Failed"),
                  }
                : f,
            ),
          );

          errorCount++;
        }
      }

      // Update toast with final result
      if (errorCount === 0) {
        toast.success(
          t("documents.upload.success", { count: successCount }) ||
            `Successfully uploaded ${successCount} file(s)`,
          {
            id: toastId,
            duration: 5000,
            action: {
              label: t("documents.upload.viewInGraph", "View in Graph"),
              onClick: () => router.push("/graph"),
            },
          },
        );
      } else if (successCount === 0) {
        toast.error(
          t("documents.upload.allFailed", { count: errorCount }) ||
            `All ${errorCount} file(s) failed to upload`,
          {
            id: toastId,
            duration: 5000,
            action: {
              label: t("common.retry", "Retry"),
              onClick: () => {
                setUploadingFiles([]);
              },
            },
          },
        );
      } else {
        toast.warning(
          t("documents.upload.partial", {
            success: successCount,
            failed: errorCount,
          }) || `Uploaded ${successCount} file(s), ${errorCount} failed`,
          {
            id: toastId,
            duration: 5000,
            action: {
              label: t("documents.upload.viewInGraph", "View in Graph"),
              onClick: () => router.push("/graph"),
            },
          },
        );
      }

      // Refresh documents list - invalidate AND refetch immediately
      // WHY: Ensures the document panel shows newly uploaded files immediately
      // even if WebSocket updates are delayed or miss the initial document
      await queryClient.invalidateQueries({ queryKey: ["documents"] });
      // Force immediate refetch of all documents queries
      queryClient.refetchQueries({ 
        queryKey: ["documents"],
        type: "active",
      });
      
      setIsUploading(false);

      // Clear upload list after delay
      setTimeout(() => {
        setUploadingFiles([]);
      }, 3000);
    },
    [queryClient, t, router, tenantId, workspaceId, onUploadStart],
  );

  /**
   * Remove a file from the upload list
   */
  const removeUploadingFile = useCallback((index: number) => {
    setUploadingFiles((prev) => prev.filter((_, i) => i !== index));
  }, []);

  /**
   * Mark PDF upload as successful (called by PdfUploadProgress)
   */
  const handleUploadComplete = useCallback((index: number) => {
    setUploadingFiles((prev) =>
      prev.map((f, idx) =>
        idx === index ? { ...f, status: "success" as const, progress: 100 } : f,
      ),
    );
  }, []);

  /**
   * Mark PDF upload as failed (called by PdfUploadProgress)
   */
  const handleUploadFailed = useCallback((index: number, error: string) => {
    setUploadingFiles((prev) =>
      prev.map((f, idx) =>
        idx === index ? { ...f, status: "error" as const, error } : f,
      ),
    );
  }, []);

  return {
    uploadingFiles,
    isUploading,
    handleFilesUpload,
    removeUploadingFile,
    handleUploadComplete,
    handleUploadFailed,
  };
}

export default useFileUpload;
