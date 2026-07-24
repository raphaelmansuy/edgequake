/**
 * Single-file upload router — DRY SSOT for PDF / image / text paths.
 */

import {
  uploadDocument,
  uploadFile,
  uploadPdfDocument,
} from "@/lib/api/edgequake";
import type { PdfUploadOptions } from "@/types";
import type { MultipartUploadProgress } from "@/lib/upload/multipart-upload-client";

import { classifyUploadFile } from "./file-kind";
import { resolveProgressTrackId } from "./progress-track-id";

export interface PerformFileUploadOptions {
  /** Client batch correlation id (multipart); not the progress-store key. */
  batchTrackId: string;
  /** SPEC-084 / GH-318: total files in this client batch (track completeness). */
  expectedBatchCount?: number;
  pdfParserBackend?: PdfUploadOptions["pdf_parser_backend"];
  /** Enable inline image VLM analysis on PDF markdown (LightRAG `process_options=i`). */
  analyzeInlineImages?: boolean;
  onUploadProgress?: (progress: MultipartUploadProgress) => void;
}

/** Normalized shape consumed by useFileUpload optimistic updates. */
export interface NormalizedUploadResult {
  document_id?: string;
  pdf_id?: string;
  duplicate_of?: string;
  task_id?: string;
  track_id?: string;
  status?: string;
  isPdf: boolean;
  source_type: "pdf" | "image" | "text";
}

function duplicateFromFileUpload(response: {
  document_id: string;
  is_duplicate?: boolean;
  duplicate_of?: string;
  status?: string;
}): string | undefined {
  if (response.duplicate_of) return response.duplicate_of;
  if (response.is_duplicate || response.status === "duplicate_processing") {
    return response.document_id;
  }
  return undefined;
}

/**
 * Upload one file via the correct API (never `file.text()` for images).
 */
export async function performFileUpload(
  file: File,
  options: PerformFileUploadOptions,
): Promise<NormalizedUploadResult> {
  const kind = classifyUploadFile(file);

  if (kind === "pdf") {
    const pdfResponse = await uploadPdfDocument(file, {
      title: file.name,
      enable_vision: true,
      track_id: options.batchTrackId,
      pdf_parser_backend: options.pdfParserBackend,
      analyze_inline_images: options.analyzeInlineImages ?? true,
      onUploadProgress: options.onUploadProgress,
      metadata: options.expectedBatchCount
        ? { expected_batch_count: options.expectedBatchCount }
        : undefined,
    });
    return {
      document_id: pdfResponse.document_id,
      pdf_id: pdfResponse.pdf_id,
      duplicate_of:
        pdfResponse.duplicate_of ??
        (pdfResponse.status === "duplicate" ? pdfResponse.pdf_id : undefined),
      task_id: pdfResponse.task_id,
      // SPEC-054 / #300: subscribe to server task_id, not client batch id.
      track_id: resolveProgressTrackId(pdfResponse),
      status: pdfResponse.status,
      isPdf: true,
      source_type: "pdf",
    };
  }

  if (kind === "image") {
    const fileResponse = await uploadFile(file, {
      onUploadProgress: options.onUploadProgress,
    });
    return {
      document_id: fileResponse.document_id,
      duplicate_of: duplicateFromFileUpload(
        fileResponse as {
          document_id: string;
          is_duplicate?: boolean;
          duplicate_of?: string;
          status?: string;
        },
      ),
      task_id: fileResponse.task_id,
      // Same SSOT as PDF: prefer task_id for progress subscription.
      track_id: resolveProgressTrackId(fileResponse),
      status: fileResponse.status,
      isPdf: false,
      source_type: "image",
    };
  }

  const text = await file.text();
  const textResponse = await uploadDocument({
    content: text,
    source_type: "text",
    title: file.name,
    async_processing: true,
    track_id: options.batchTrackId,
    metadata: options.expectedBatchCount
      ? { expected_batch_count: options.expectedBatchCount }
      : undefined,
  });

  return {
    document_id: textResponse.document_id,
    duplicate_of: textResponse.duplicate_of,
    task_id: textResponse.task_id,
    track_id: resolveProgressTrackId(textResponse),
    status: textResponse.status,
    isPdf: false,
    source_type: "text",
  };
}
