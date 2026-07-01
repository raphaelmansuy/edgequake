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

export interface PerformFileUploadOptions {
  trackId: string;
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
      track_id: options.trackId,
      pdf_parser_backend: options.pdfParserBackend,
      analyze_inline_images: options.analyzeInlineImages ?? false,
      onUploadProgress: options.onUploadProgress,
    });
    return {
      document_id: pdfResponse.document_id,
      pdf_id: pdfResponse.pdf_id,
      duplicate_of:
        pdfResponse.duplicate_of ??
        (pdfResponse.status === "duplicate" ? pdfResponse.pdf_id : undefined),
      task_id: pdfResponse.task_id,
      track_id: pdfResponse.track_id,
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
      track_id: fileResponse.track_id,
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
    track_id: options.trackId,
  });

  return {
    document_id: textResponse.document_id,
    duplicate_of: textResponse.duplicate_of,
    task_id: textResponse.task_id,
    track_id: textResponse.track_id,
    status: textResponse.status,
    isPdf: false,
    source_type: "text",
  };
}
