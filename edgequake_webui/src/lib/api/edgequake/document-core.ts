import { getRuntimeServerBaseUrl } from "@/lib/runtime-config";
import {
  postMultipart,
  type MultipartUploadProgress,
} from "@/lib/upload/multipart-upload-client";
import { appendSecurityFields } from "@/lib/upload/append-security-fields";
import { buildPdfUploadFormData } from "@/lib/upload/pdf-upload-form-data";
import type { SecurityFields } from "@/lib/security/security-fields";
import type {
  Document,
  DocumentStatusCounts,
  ListDocumentsResponse,
  PaginatedResponse,
  PaginationParams,
  PdfUploadOptions,
  PdfUploadResponse,
  UploadDocumentRequest,
  UploadDocumentResponse,
} from "@/types";
import { api, DOCUMENTS_API_TIMEOUT_MS } from "../client";
import { buildQueryString, withQuery } from "../query-params";

export interface DocumentsListResult extends PaginatedResponse<Document> {
  status_counts: DocumentStatusCounts;
}

export async function getDocuments(
  params?: PaginationParams & {
    status?: string;
    date_from?: string;
    date_to?: string;
    document_pattern?: string;
  },
): Promise<DocumentsListResult> {
  const query = buildQueryString({
    page: params?.page,
    page_size: params?.page_size,
    sort_by: params?.sort_by,
    sort_order: params?.sort_order,
    status: params?.status,
    date_from: params?.date_from,
    date_to: params?.date_to,
    document_pattern: params?.document_pattern,
  });
  const response = await api.get<ListDocumentsResponse>(
    withQuery("/documents", query),
    { timeoutMs: DOCUMENTS_API_TIMEOUT_MS },
  );
  return {
    items: response.documents || [],
    total: response.total || 0,
    page: response.page || 1,
    page_size: response.page_size || 20,
    total_pages:
      response.total_pages ||
      Math.ceil((response.total || 0) / (response.page_size || 20)),
    has_more:
      response.has_more ?? response.page * response.page_size < response.total,
    status_counts: response.status_counts || {
      pending: 0,
      processing: 0,
      completed: 0,
      failed: 0,
    },
  };
}

export async function getDocument(documentId: string): Promise<Document> {
  return api.get<Document>(`/documents/${documentId}`, {
    timeoutMs: DOCUMENTS_API_TIMEOUT_MS,
  });
}

export async function uploadDocument(
  data: UploadDocumentRequest,
): Promise<UploadDocumentResponse> {
  return api.post<UploadDocumentResponse>("/documents", data);
}

export async function uploadFile(
  file: File,
  options?: {
    onUploadProgress?: (progress: MultipartUploadProgress) => void;
    security?: SecurityFields;
  },
): Promise<UploadDocumentResponse> {
  const formData = new FormData();
  formData.append("file", file);
  appendSecurityFields(formData, options?.security);
  return postMultipart<UploadDocumentResponse>("/documents/upload", formData, {
    fileSizeBytes: file.size,
    onProgress: options?.onUploadProgress,
  });
}

export type PdfUploadRequestOptions = PdfUploadOptions & {
  onUploadProgress?: (progress: MultipartUploadProgress) => void;
  security?: SecurityFields;
};

export async function uploadPdfDocument(
  file: File,
  options?: PdfUploadRequestOptions,
): Promise<PdfUploadResponse> {
  const formData = buildPdfUploadFormData(file, options);
  return postMultipart<PdfUploadResponse>("/documents/pdf", formData, {
    fileSizeBytes: file.size,
    onProgress: options?.onUploadProgress,
  });
}

export interface PdfProgressResponse {
  track_id: string;
  pdf_id: string;
  document_id?: string | null;
  filename: string;
  status?: "pending" | "processing" | "completed" | "failed";
  phases: PhaseProgressData[];
  overall_percentage: number;
  is_complete: boolean;
  is_failed: boolean;
  started_at: string;
  updated_at: string;
  completed_at?: string | null;
  eta_seconds?: number | null;
  error?: string;
}

export interface PhaseProgressData {
  phase: string;
  status: "pending" | "active" | "complete" | "failed" | "skipped";
  current: number;
  total: number;
  percentage: number;
  message: string;
  eta_seconds?: number | null;
  error?: PhaseErrorData | null;
  started_at?: string | null;
  completed_at?: string | null;
}

export interface PhaseErrorData {
  message: string;
  code: string;
  retryable: boolean;
  suggestion: string;
  affected_item?: string | null;
}

export type PhaseStatus =
  | { type: "pending" }
  | { type: "active"; current: number; total: number; percent: number }
  | { type: "completed" }
  | { type: "failed"; error: string };

export interface PdfOperationResponse {
  success: boolean;
  pdf_id: string;
  message: string;
  task_id?: string;
}

export async function getPdfProgress(
  trackId: string,
): Promise<PdfProgressResponse> {
  return api.get<PdfProgressResponse>(`/documents/pdf/progress/${trackId}`);
}

export function createPdfProgressEventSource(trackId: string): EventSource {
  return new EventSource(
    `${getRuntimeServerBaseUrl()}/api/v1/documents/pdf/progress/stream/${trackId}`,
  );
}

export async function retryPdfProcessing(
  pdfId: string,
): Promise<PdfOperationResponse> {
  return api.post<PdfOperationResponse>(`/documents/pdf/${pdfId}/retry`);
}

export async function cancelPdfProcessing(
  pdfId: string,
): Promise<PdfOperationResponse> {
  return api.delete<PdfOperationResponse>(`/documents/pdf/${pdfId}/cancel`);
}
