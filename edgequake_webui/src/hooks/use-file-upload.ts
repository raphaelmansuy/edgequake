/**
 * useFileUpload - File upload state and handlers
 *
 * @fileoverview Extracted from DocumentManager (OODA-13)
 * WHY: SRP - Upload orchestration is a distinct responsibility
 *
 * @module edgequake_webui/hooks/use-file-upload
 */
"use client";

import type {
  DuplicateResolutions,
  PendingDuplicate,
} from "@/components/documents/duplicate-upload-dialog";
import type { UploadingFile } from "@/components/documents/types";
import {
  deleteDocument,
  uploadPdfDocument,
  type DocumentsListResult,
} from "@/lib/api/edgequake";
import {
  pinDocumentShell,
  scheduleDeferredUnpin,
} from "@/lib/documents/progress-admit";
import { performFileUpload } from "@/lib/upload/perform-file-upload";
import {
  ADMIT_PROGRESS_PERCENT,
  formatUploadMegabytes,
  transferProgressPercent,
} from "@/lib/upload/upload-timeout";
import type { MultipartUploadProgress } from "@/lib/upload/multipart-upload-client";
import { isImageUploadFile, isPdfUploadFile } from "@/lib/upload/file-kind";
import type { Document } from "@/types";
import {
  getDocumentDisplayStatus,
  isTerminalStatus,
} from "@/components/documents/status-badge";
import { useIngestionStore } from "@/stores/use-ingestion-store";
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
  /** Optional per-upload PDF parser backend override. */
  pdfParserBackend?: "vision" | "edgeparse";
}

export interface UseFileUploadReturn {
  /** Files currently being uploaded with progress */
  uploadingFiles: UploadingFile[];
  /** Whether any upload is in progress */
  isUploading: boolean;
  /** Upload files handler */
  handleFilesUpload: (
    files: File[],
    uploadOptions?: { pdfParserBackend?: "vision" | "edgeparse" },
  ) => Promise<void>;
  /** Remove a file from upload list */
  removeUploadingFile: (index: number) => void;
  /** Mark upload as complete (for PdfUploadProgress) */
  handleUploadComplete: (index: number) => void;
  /** Mark upload as failed (for PdfUploadProgress) */
  handleUploadFailed: (index: number, error: string) => void;
  /** Drop client upload rows once matching documents are terminal (SPEC-048). */
  pruneTerminalUploads: (docs: Document[]) => void;
  /** Duplicates that need user resolution (drives DuplicateUploadDialog). */
  pendingDuplicates: PendingDuplicate[];
  /**
   * Called when the user confirms decisions in DuplicateUploadDialog.
   * Iterates resolutions: "replace" deletes the old document then re-uploads
   * the new file as a fresh document; "skip" is a no-op.
   * Clears pendingDuplicates afterwards.
   */
  resolvePendingDuplicates: (resolutions: DuplicateResolutions) => void;
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
  const { pdfParserBackend } = options;

  const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
  const [isUploading, setIsUploading] = useState(false);
  // WHY: Duplicates are collected during the upload loop and shown to the
  // user in a single DuplicateUploadDialog after all files are processed.
  const [pendingDuplicates, setPendingDuplicates] = useState<
    PendingDuplicate[]
  >([]);

  const queryClient = useQueryClient();
  const router = useRouter();
  const { t } = useTranslation();

  /**
   * Main upload handler with progress tracking
   * WHY: Process files sequentially for better feedback and error isolation
   */
  const handleFilesUpload = useCallback(
    async (
      files: File[],
      uploadOptions?: { pdfParserBackend?: "vision" | "edgeparse" },
    ) => {
      if (files.length === 0) return;

      // FIX-DUPLICATE-BUG: Prevent double-submit when upload is already in progress.
      // WHY: Without this guard, rapid clicks or drag-and-drop events can trigger
      // multiple concurrent uploads of the same file, resulting in duplicate documents
      // with different IDs, both stuck in "processing" state.
      if (isUploading) {
        console.warn(
          "[useFileUpload] Upload already in progress, ignoring duplicate submission",
        );
        return;
      }

      // Notify parent (e.g., to switch status filter)
      onUploadStart?.();

      setIsUploading(true);

      // Client batch correlation id (shared across files). Progress keys are per-task.
      const batchTrackId = `upload_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;

      // Initialize upload state for all files
      const initialFiles: UploadingFile[] = files.map((file) => ({
        file,
        progress: 0,
        status: "pending" as const,
        phase: t("common.waiting", "Waiting..."),
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
          const applyUploadProgress = (progress: MultipartUploadProgress) => {
            const { loaded, total, phase } = progress;
            const bytesTotal = total > 0 ? total : file.size;
            const bytesSent = Math.min(loaded, bytesTotal);
            const progressPercent =
              phase === "admit"
                ? ADMIT_PROGRESS_PERCENT
                : transferProgressPercent(bytesSent, bytesTotal);
            const phaseLabel =
              phase === "admit"
                ? t("documents.upload.saving", "Saving to workspace...")
                : t("documents.upload.sending", "Sending {{sent}} / {{total}} MB", {
                    sent: formatUploadMegabytes(bytesSent),
                    total: formatUploadMegabytes(bytesTotal),
                  });

            setUploadingFiles((prev) =>
              prev.map((f, idx) =>
                idx === i
                  ? {
                      ...f,
                      status: "uploading" as const,
                      progress: progressPercent,
                      phase: phaseLabel,
                      bytesSent,
                      bytesTotal,
                      uploadPhase: phase,
                    }
                  : f,
              ),
            );
          };

          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? {
                    ...f,
                    status: "uploading" as const,
                    progress: 5,
                    phase: t("documents.upload.sending", "Sending {{sent}} / {{total}} MB", {
                      sent: "0.0",
                      total: formatUploadMegabytes(file.size),
                    }),
                    bytesSent: 0,
                    bytesTotal: file.size,
                    uploadPhase: "transfer" as const,
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
            isPdf?: boolean;
            source_type?: "pdf" | "image" | "text";
          };

          const uploadResult = await performFileUpload(file, {
            expectedBatchCount: files.length,
            batchTrackId,
            pdfParserBackend: uploadOptions?.pdfParserBackend ?? pdfParserBackend,
            onUploadProgress: applyUploadProgress,
          });
          response = {
            document_id: uploadResult.document_id,
            pdf_id: uploadResult.pdf_id,
            duplicate_of: uploadResult.duplicate_of,
            task_id: uploadResult.task_id,
            track_id: uploadResult.track_id,
            isPdf: uploadResult.isPdf,
            source_type: uploadResult.source_type,
          };

          const isPdfDuplicate =
            !!uploadResult.duplicate_of ||
            uploadResult.status === "duplicate" ||
            uploadResult.status === "duplicate_processing";

          if (uploadResult.isPdf && uploadResult.pdf_id && !isPdfDuplicate) {
            const optimisticId =
              uploadResult.document_id ?? uploadResult.pdf_id;
            const optimisticDoc: Document = {
              id: optimisticId,
              title: file.name,
              file_name: file.name,
              file_size: file.size,
              source_type: "pdf",
              status:
                uploadResult.status === "queued" ? "pending" : "processing",
              current_stage:
                uploadResult.status === "queued" ? "queued" : "converting",
              stage_message:
                uploadResult.status === "queued"
                  ? t(
                      "pipeline.waitingForSlot",
                      "Waiting for a processing slot",
                    )
                  : undefined,
              mime_type: "application/pdf",
              created_at: new Date().toISOString(),
              pdf_id: uploadResult.pdf_id,
              track_id: uploadResult.track_id,
              tenant_id: tenantId ?? undefined,
              workspace_id: workspaceId ?? undefined,
            };

            pinDocumentShell(optimisticDoc);
            scheduleDeferredUnpin(optimisticId);
            queryClient.setQueriesData<DocumentsListResult>(
              { predicate: (query) => query.queryKey[0] === "documents" },
              (old) => {
                if (!old || !old.items || !Array.isArray(old.items))
                  return old;
                const exists = old.items.some(
                  (d) =>
                    d.pdf_id === uploadResult.pdf_id ||
                    d.id === optimisticId ||
                    (uploadResult.document_id != null &&
                      d.id === uploadResult.document_id),
                );
                if (exists) return old;
                return {
                  ...old,
                  items: [optimisticDoc, ...old.items],
                  total: (old.total ?? 0) + 1,
                };
              },
            );
          } else if (
            !uploadResult.isPdf &&
            uploadResult.document_id &&
            !uploadResult.duplicate_of
          ) {
            const optimisticDoc: Document = {
              id: uploadResult.document_id,
              title: file.name,
              file_name: file.name,
              file_size: file.size,
              source_type: uploadResult.source_type ?? "text",
              status: "processing",
              current_stage: "chunking",
              stage_message: t("documents.upload.extracting", "Processing..."),
              mime_type: file.type || "text/plain",
              created_at: new Date().toISOString(),
              track_id: uploadResult.track_id,
              tenant_id: tenantId ?? undefined,
              workspace_id: workspaceId ?? undefined,
            };

            pinDocumentShell(optimisticDoc);
            scheduleDeferredUnpin(uploadResult.document_id);
            queryClient.setQueriesData<DocumentsListResult>(
              { predicate: (query) => query.queryKey[0] === "documents" },
              (old) => {
                if (!old || !old.items || !Array.isArray(old.items))
                  return old;
                const exists = old.items.some(
                  (d) => d.id === uploadResult.document_id,
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

          if (uploadResult.isPdf) {
            setUploadingFiles((prev) =>
              prev.map((f, idx) =>
                idx === i
                  ? {
                      ...f,
                      isPdf: true,
                    }
                  : f,
              ),
            );
          }

          // Check for duplicate — collect for dialog instead of showing a toast.
          // WHY: A dialog gives the user clear choices (replace / skip) and
          // handles bulk uploads in one interaction rather than N toasts.
          if (response.duplicate_of) {
            setPendingDuplicates((prev) => [
              ...prev,
              {
                fileName: file.name,
                existingDocId: response.duplicate_of!,
                file,
              },
            ]);

            // Mark the file entry as "duplicate/pending decision"
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

          // Pipeline tracking: PDF + text/markdown/image share track_id progress (FEAT0602 parity)
          if (uploadResult.track_id) {
            const documentId =
              uploadResult.document_id ?? uploadResult.pdf_id ?? "";
            useIngestionStore.getState().startTracking(
              uploadResult.track_id,
              documentId,
              file.name,
            );

            setUploadingFiles((prev) =>
              prev.map((f, idx) =>
                idx === i
                  ? {
                      ...f,
                      trackId: uploadResult.track_id,
                      status: "extracting" as const,
                      progress: uploadResult.isPdf ? f.progress : 85,
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

            successCount++;
            continue;
          }

          // No track_id — mark complete immediately (sync path)
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? {
                    ...f,
                    status: "extracting" as const,
                    progress: 80,
                    phase: t("documents.upload.extracting", "Processing..."),
                  }
                : f,
            ),
          );

          await new Promise((resolve) => setTimeout(resolve, 300));

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
            error instanceof Error
              ? error.message
              : t("documents.upload.uploadFailed", "Upload failed");
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

      // Drop finished HTTP uploads; keep pipeline-tracked rows until onComplete
      setTimeout(() => {
        setUploadingFiles((prev) =>
          prev.filter((f) => f.trackId && f.status === "extracting"),
        );
      }, 3000);
    },
    [isUploading, onUploadStart, pdfParserBackend, queryClient, router, t, tenantId, workspaceId],
  );

  /**
   * Remove a file from the upload list
   */
  const removeUploadingFile = useCallback((index: number) => {
    setUploadingFiles((prev) => prev.filter((_, i) => i !== index));
  }, []);

  /**
   * Resolve pending duplicate decisions.
   * WHY: Called by DuplicateUploadDialog after user clicks Confirm.
   *
   * For PDF files: re-upload with force_reindex=true so the backend clears
   * old graph/vector data and re-processes the PDF, without a separate DELETE.
   * WHY (OODA-08): The backend's force_reindex flag atomically clears old data
   * and triggers fresh extraction — safer than a frontend DELETE + re-upload
   * which would race with the duplicate-hash check and 404 on pdf_id.
   *
   * For non-PDF files: the backend's text upload handler already auto-deletes
   * on duplicate (FIX-4), so we just re-upload. A delete is attempted first
   * for completeness but failures are non-fatal.
   *
   * "skip" decisions are no-ops.
   * @implements BR-dup-replace - Replace = force_reindex for PDFs
   */
  const resolvePendingDuplicates = useCallback(
    (resolutions: DuplicateResolutions) => {
      const replaceEntries = pendingDuplicates.filter(
        (d) => resolutions[d.existingDocId] === "replace",
      );
      setPendingDuplicates([]);

      if (replaceEntries.length === 0) return;

      // Close dialog immediately; async replace runs in the background.
      const doReplaceAll = async () => {
        let replaceErrors = 0;
        for (const entry of replaceEntries) {
          const isPdf = isPdfUploadFile(entry.file);
          const isImage = isImageUploadFile(entry.file);

          const failReplace = (err: unknown) => {
            replaceErrors += 1;
            const message =
              err instanceof Error ? err.message : t("common.unknownError", "Unknown error");
            toast.error(
              t("documents.upload.replaceFailed", "Failed to replace {{name}}", {
                name: entry.fileName,
              }),
              { description: message },
            );
          };

          if (isPdf) {
            try {
              const batchTrackId = `upload_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
              await uploadPdfDocument(entry.file, {
                title: entry.file.name,
                enable_vision: true,
                track_id: batchTrackId,
                force_reindex: true,
              });
              queryClient.invalidateQueries({ queryKey: ["documents"] });
            } catch (err) {
              failReplace(err);
            }
          } else if (isImage) {
            try {
              await performFileUpload(entry.file, {
                batchTrackId: `upload_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`,
              });
              queryClient.invalidateQueries({ queryKey: ["documents"] });
            } catch (err) {
              failReplace(err);
            }
          } else {
            try {
              await deleteDocument(entry.existingDocId);
            } catch {
              // Non-fatal — backend may recycle orphan hash keys.
            }
            try {
              await handleFilesUpload([entry.file]);
              queryClient.invalidateQueries({ queryKey: ["documents"] });
            } catch (err) {
              failReplace(err);
            }
          }
        }
        if (replaceErrors === 0 && replaceEntries.length > 0) {
          toast.success(
            t("documents.upload.replaceStarted", "Re-upload started for {{count}} file(s)", {
              count: replaceEntries.length,
            }),
          );
        }
      };

      doReplaceAll();
    },
    [pendingDuplicates, handleFilesUpload, queryClient],
  );

  /**
   * Mark PDF upload as successful (called by PdfUploadProgress)
   */
  const handleUploadComplete = useCallback((index: number) => {
    setUploadingFiles((prev) => {
      const completedTrackId = prev[index]?.trackId;
      const next = prev.map((f, idx) =>
        idx === index
          ? {
              ...f,
              status: "success" as const,
              progress: 100,
              phase: t("documents.upload.complete", "Complete!"),
            }
          : f,
      );
      if (completedTrackId) {
        setTimeout(() => {
          setUploadingFiles((current) =>
            current.filter((f) => f.trackId !== completedTrackId),
          );
        }, 2500);
      }
      return next;
    });
  }, [t]);

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

  /**
   * SPEC-048: clear client upload rows once the matching document is terminal.
   * Prevents leftover progress chrome after ingest completes.
   */
  const pruneTerminalUploads = useCallback((docs: Document[]) => {
    if (!docs.length) return;
    setUploadingFiles((prev) => {
      if (prev.length === 0) return prev;
      const next = prev.filter((f) => {
        if (f.status === "error") return true;
        const match = docs.find(
          (d) =>
            (f.trackId && d.track_id === f.trackId) ||
            (f.file?.name &&
              (d.file_name === f.file.name || d.title === f.file.name)),
        );
        if (!match) return true;
        const status = getDocumentDisplayStatus(match);
        return !isTerminalStatus(status);
      });
      return next.length === prev.length ? prev : next;
    });
  }, []);

  return {
    uploadingFiles,
    isUploading,
    handleFilesUpload,
    removeUploadingFile,
    handleUploadComplete,
    handleUploadFailed,
    pruneTerminalUploads,
    pendingDuplicates,
    resolvePendingDuplicates,
  };
}

export default useFileUpload;
