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
  type DocumentsListResult,
  uploadPdfDocument,
} from "@/lib/api/edgequake";
import {
  pinDocumentShell,
  scheduleDeferredUnpin,
} from "@/lib/documents/progress-admit";
import {
  admitPartialMessage,
  admitSuccessMessage,
} from "@/lib/documents/admit-copy";
import { performFileUpload } from "@/lib/upload/perform-file-upload";
import {
  createBoundedExecutor,
  createUploadId,
  fileUploadFingerprint,
  MAX_CONCURRENT_FILE_UPLOADS,
  perFileUploadErrorMessage,
  updateByUploadId,
} from "@/lib/upload/bounded-file-upload";
import {
  ADMIT_PROGRESS_PERCENT,
  formatUploadMegabytes,
  transferProgressPercent,
} from "@/lib/upload/upload-timeout";
import type { MultipartUploadProgress } from "@/lib/upload/multipart-upload-client";
import { isImageUploadFile, isPdfUploadFile } from "@/lib/upload/file-kind";
import { waitForDocumentAbsent } from "@/lib/upload/wait-for-document-absent";
import type { Document } from "@/types";
import {
  getDocumentDisplayStatus,
  isTerminalStatus,
} from "@/lib/documents/status-domain";
import { useIngestionStore } from "@/stores/use-ingestion-store";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useRef, useState } from "react";
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
  pdfParserBackend?: "vision" | "edgeparse" | "auto";
  /** SPEC-109: vision convert reasoning effort (multipart). */
  visionReasoningEffort?: string;
  /** SPEC-015V */
  visionExtractImages?: boolean;
  visionExtractCharts?: boolean;
  visionExtractFigures?: boolean;
  visionPageSystemPrompt?: string;
  visionImageSystemPrompt?: string;
  visionChartSystemPrompt?: string;
  visionFigureSystemPrompt?: string;
  /**
   * SPEC-099 LAW-099-6: when true, skip persistent "Uploading N…" loading toast
   * because the feedback zone owns the upload session narrative.
   */
  demoteLoadingToast?: boolean;
  /** SPEC-146 security labels applied to each upload. */
  security?: import("@/lib/security/security-fields").SecurityFields;
}

export interface UseFileUploadReturn {
  /** Files currently being uploaded with progress */
  uploadingFiles: UploadingFile[];
  /** Whether any upload is in progress */
  isUploading: boolean;
  /** Upload files handler */
  handleFilesUpload: (
    files: File[],
    uploadOptions?: {
      pdfParserBackend?: "vision" | "edgeparse" | "auto";
      visionReasoningEffort?: string;
    },
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
 * - Bounded concurrent file upload with progress tracking
 * - PDF vs text file routing
 * - Optimistic cache updates
 * - Duplicate detection
 * - Success/error toast notifications
 */
export function useFileUpload(
  options: UseFileUploadOptions = {},
): UseFileUploadReturn {
  const { tenantId, workspaceId, onUploadStart } = options;
  const {
    pdfParserBackend,
    visionReasoningEffort,
    visionExtractImages,
    visionExtractCharts,
    visionExtractFigures,
    visionPageSystemPrompt,
    visionImageSystemPrompt,
    visionChartSystemPrompt,
    visionFigureSystemPrompt,
    demoteLoadingToast = true,
    security,
  } =
    options;

  const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
  const [activeBatchCount, setActiveBatchCount] = useState(0);
  const inFlightFingerprints = useRef(new Set<string>());
  const uploadExecutor = useRef(
    createBoundedExecutor(MAX_CONCURRENT_FILE_UPLOADS),
  );
  const isUploading = activeBatchCount > 0;
  // WHY: Duplicates are collected during the upload loop and shown to the
  // user in a single DuplicateUploadDialog after all files are processed.
  const [pendingDuplicates, setPendingDuplicates] = useState<
    PendingDuplicate[]
  >([]);

  const queryClient = useQueryClient();
  const { t } = useTranslation();

  const updateUploadingFile = useCallback(
    (
      uploadId: string,
      update:
        | Partial<UploadingFile>
        | ((current: UploadingFile) => Partial<UploadingFile>),
    ) => {
      setUploadingFiles((current) =>
        updateByUploadId(current, uploadId, update),
      );
    },
    [],
  );

  /**
   * Main upload handler with progress tracking
   * WHY: Bound transfer/admission concurrency independently from task fairness.
   */
  const handleFilesUpload = useCallback(
    async (
      files: File[],
      uploadOptions?: {
        pdfParserBackend?: "vision" | "edgeparse" | "auto";
        visionReasoningEffort?: string;
      },
    ) => {
      if (files.length === 0) return;

      // Suppress only exact files already in flight. A global isUploading guard
      // contradicted "Add more files anytime" and silently discarded later batches.
      const batchFingerprints = new Set<string>();
      const acceptedFiles = files.filter((file) => {
        const fingerprint = fileUploadFingerprint(file);
        if (
          batchFingerprints.has(fingerprint) ||
          inFlightFingerprints.current.has(fingerprint)
        ) {
          return false;
        }
        batchFingerprints.add(fingerprint);
        inFlightFingerprints.current.add(fingerprint);
        return true;
      });
      if (acceptedFiles.length === 0) {
        console.warn(
          "[useFileUpload] All selected files are already uploading",
        );
        return;
      }

      // Notify parent (e.g., to switch status filter)
      onUploadStart?.();

      setActiveBatchCount((count) => count + 1);

      // Client batch correlation id (shared across files). Progress keys are per-task.
      const batchTrackId = `upload_${Date.now()}_${Math.random()
        .toString(36)
        .slice(2, 10)}`;

      // Append new intent rows; never replace a batch already in progress.
      const initialFiles: UploadingFile[] = acceptedFiles.map((file) => ({
        uploadId: createUploadId(),
        file,
        progress: 0,
        status: "pending" as const,
        phase: t("common.waiting", "Waiting..."),
      }));
      const selectionUploadIds = new Set(
        initialFiles.map((entry) => entry.uploadId),
      );
      setUploadingFiles((current) => [...current, ...initialFiles]);

      // SPEC-099 LAW-099-6: toast XOR feedback-zone upload list.
      // When demoteLoadingToast, skip persistent loading toast; completion
      // toasts still fire with a stable id.
      const toastId = `upload-batch-${Date.now()}`;
      if (!demoteLoadingToast) {
        toast.loading(
          t("documents.upload.inProgress", { count: acceptedFiles.length }) ||
            `Uploading ${acceptedFiles.length} file(s)...`,
          { id: toastId, duration: Infinity },
        );
      }

      let successCount = 0;
      let errorCount = 0;

      // Each worker catches its own errors, so one failure cannot stop siblings.
      await Promise.all(
        initialFiles.map(({ file, uploadId }) =>
          uploadExecutor.current.run(async () => {
            // Phase 1: Reading file
            updateUploadingFile(uploadId, {
              status: "reading" as const,
              progress: 10,
              phase: t("documents.upload.reading", "Reading file..."),
            });

            try {
              const applyUploadProgress = (
                progress: MultipartUploadProgress,
              ) => {
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
                    : t(
                        "documents.upload.sending",
                        "Sending {{sent}} / {{total}} MB",
                        {
                          sent: formatUploadMegabytes(bytesSent),
                          total: formatUploadMegabytes(bytesTotal),
                        },
                      );

                updateUploadingFile(uploadId, {
                  status: "uploading" as const,
                  progress: progressPercent,
                  phase: phaseLabel,
                  bytesSent,
                  bytesTotal,
                  uploadPhase: phase,
                });
              };

              updateUploadingFile(uploadId, {
                status: "uploading" as const,
                progress: 5,
                phase: t(
                  "documents.upload.sending",
                  "Sending {{sent}} / {{total}} MB",
                  {
                    sent: "0.0",
                    total: formatUploadMegabytes(file.size),
                  },
                ),
                bytesSent: 0,
                bytesTotal: file.size,
                uploadPhase: "transfer" as const,
              });

              let response: {
                document_id?: string;
                pdf_id?: string;
                duplicate_of?: string;
                task_id?: string;
                track_id?: string;
                isPdf?: boolean;
                source_type?: "pdf" | "image" | "text" | "markdown";
              };

              const uploadResult = await performFileUpload(file, {
                expectedBatchCount: acceptedFiles.length,
                batchTrackId,
                pdfParserBackend:
                  uploadOptions?.pdfParserBackend ?? pdfParserBackend,
                visionReasoningEffort:
                  uploadOptions?.visionReasoningEffort ?? visionReasoningEffort,
                visionExtractImages,
                visionExtractCharts,
                visionExtractFigures,
                visionPageSystemPrompt,
                visionImageSystemPrompt,
                visionChartSystemPrompt,
                visionFigureSystemPrompt,
                security,
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

              if (
                uploadResult.isPdf &&
                uploadResult.pdf_id &&
                !isPdfDuplicate
              ) {
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
                    if (!old || !old.items || !Array.isArray(old.items)) {
                      return old;
                    }
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
                  // SPEC-086: align with admit (uploading/queued) — do not hard-code chunking.
                  status: "pending",
                  current_stage: "uploading",
                  // No {{taskId}} — documents.upload.queued requires interpolation.
                  stage_message: t(
                    "documents.upload.queuedPending",
                    "Queued for processing…",
                  ),
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
                    if (!old || !old.items || !Array.isArray(old.items)) {
                      return old;
                    }
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
                updateUploadingFile(uploadId, { isPdf: true });
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
                updateUploadingFile(uploadId, {
                  status: "success" as const,
                  progress: 100,
                  phase: t(
                    "documents.upload.duplicateSkipped",
                    "Duplicate (skipped)",
                  ),
                });
                successCount++;
                return;
              }

              // Pipeline tracking: PDF + text/markdown/image share track_id progress (FEAT0602 parity)
              if (uploadResult.track_id) {
                const documentId =
                  uploadResult.document_id ?? uploadResult.pdf_id ?? "";
                useIngestionStore
                  .getState()
                  .startTracking(uploadResult.track_id, documentId, file.name);

                updateUploadingFile(uploadId, (current) => ({
                  trackId: uploadResult.track_id,
                  status: "extracting" as const,
                  progress: uploadResult.isPdf ? current.progress : 85,
                  phase: response.task_id
                    ? t(
                        "documents.upload.queued",
                        "Queued for extraction (Task: {{taskId}})",
                        {
                          taskId: response.task_id.slice(0, 8),
                        },
                      )
                    : t("documents.upload.extracting", "Processing..."),
                }));

                successCount++;
                return;
              }

              // No track_id — mark complete immediately (sync path)
              updateUploadingFile(uploadId, {
                status: "extracting" as const,
                progress: 80,
                phase: t("documents.upload.extracting", "Processing..."),
              });

              await new Promise((resolve) => setTimeout(resolve, 300));

              updateUploadingFile(uploadId, {
                status: "success" as const,
                progress: 100,
                phase: t("documents.upload.complete", "Complete!"),
              });

              successCount++;
            } catch (error) {
              // SPEC-132 LAW-132-3: per-file terminal error; executor finally frees the slot.
              const errorMessage = perFileUploadErrorMessage(
                error,
                t("documents.upload.uploadFailed", "Upload failed"),
              );
              updateUploadingFile(uploadId, {
                status: "error" as const,
                progress: 100,
                error: errorMessage,
                phase: t("common.failed", "Failed"),
              });

              errorCount++;
            } finally {
              inFlightFingerprints.current.delete(fileUploadFingerprint(file));
            }
          }),
        ),
      );

      // SPEC-122 LAW-122-1: admit ≠ searchable — no Graph/query CTA on admit.
      if (errorCount === 0) {
        toast.success(
          t("documents.upload.admitted", { count: successCount }) ||
            admitSuccessMessage(successCount),
          {
            id: toastId,
            duration: 5000,
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
                setUploadingFiles((current) =>
                  current.filter(
                    (entry) => !selectionUploadIds.has(entry.uploadId),
                  ),
                );
              },
            },
          },
        );
      } else {
        toast.warning(
          t("documents.upload.partial", {
            success: successCount,
            failed: errorCount,
          }) || admitPartialMessage(successCount, errorCount),
          {
            id: toastId,
            duration: 5000,
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

      setActiveBatchCount((count) => Math.max(0, count - 1));

      // Drop finished HTTP uploads; keep pipeline-tracked rows until onComplete
      setTimeout(() => {
        setUploadingFiles((prev) =>
          prev.filter(
            (entry) =>
              !selectionUploadIds.has(entry.uploadId) ||
              (entry.trackId && entry.status === "extracting"),
          ),
        );
      }, 3000);
    },
    [
      onUploadStart,
      pdfParserBackend,
      visionReasoningEffort,
      visionExtractImages,
      visionExtractCharts,
      visionExtractFigures,
      visionPageSystemPrompt,
      visionImageSystemPrompt,
      visionChartSystemPrompt,
      visionFigureSystemPrompt,
      security,
      queryClient,
      t,
      tenantId,
      updateUploadingFile,
      workspaceId,
    ],
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
              err instanceof Error
                ? err.message
                : t("common.unknownError", "Unknown error");
            toast.error(
              t(
                "documents.upload.replaceFailed",
                "Failed to replace {{name}}",
                {
                  name: entry.fileName,
                },
              ),
              { description: message },
            );
          };

          if (isPdf) {
            try {
              const batchTrackId = `upload_${Date.now()}_${Math.random()
                .toString(36)
                .slice(2, 10)}`;
              await uploadPdfDocument(entry.file, {
                title: entry.file.name,
                enable_vision: true,
                track_id: batchTrackId,
                force_reindex: true,
                // SPEC-123 V4: preserve upload-level parser override on Replace.
                pdf_parser_backend: pdfParserBackend,
                security,
              });
              queryClient.invalidateQueries({ queryKey: ["documents"] });
            } catch (err) {
              failReplace(err);
            }
          } else if (isImage) {
            try {
              await performFileUpload(entry.file, {
                batchTrackId: `upload_${Date.now()}_${Math.random()
                  .toString(36)
                  .slice(2, 10)}`,
                security,
              });
              queryClient.invalidateQueries({ queryKey: ["documents"] });
            } catch (err) {
              failReplace(err);
            }
          } else {
            // Text/MD Replace: wait until old row is gone before re-admit
            // (202 async delete race created duplicate completed rows).
            try {
              const del = await deleteDocument(entry.existingDocId);
              await queryClient.invalidateQueries({ queryKey: ["documents"] });
              if (!del.deleted) {
                await waitForDocumentAbsent(queryClient, entry.existingDocId);
              }
            } catch (err) {
              failReplace(err);
              continue;
            }
            try {
              await handleFilesUpload([entry.file]);
              await queryClient.invalidateQueries({ queryKey: ["documents"] });
            } catch (err) {
              failReplace(err);
            }
          }
        }
        if (replaceErrors === 0 && replaceEntries.length > 0) {
          toast.success(
            t(
              "documents.upload.replaceStarted",
              "Re-upload started for {{count}} file(s)",
              {
                count: replaceEntries.length,
              },
            ),
          );
        }
      };

      doReplaceAll();
    },
    [pendingDuplicates, handleFilesUpload, queryClient, pdfParserBackend, security],
  );

  /**
   * Mark PDF upload as successful (called by PdfUploadProgress)
   */
  const handleUploadComplete = useCallback(
    (index: number) => {
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
    },
    [t],
  );

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
