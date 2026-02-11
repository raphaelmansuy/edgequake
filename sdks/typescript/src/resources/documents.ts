/**
 * Documents resource — upload, list, manage documents and PDFs.
 *
 * @module resources/documents
 * @see edgequake/crates/edgequake-api/src/handlers/documents.rs
 */

import { Resource } from "./base.js";
import { Paginator } from "../pagination.js";
import type { Page } from "../types/common.js";
import type {
  UploadDocumentRequest,
  UploadDocumentResponse,
  DocumentInfo,
  DocumentDetail,
  ListDocumentsQuery,
  TrackStatusResponse,
  DeletionImpactResponse,
  UploadFileResponse,
  BatchUploadResponse,
  ScanDirectoryRequest,
  ScanDirectoryResponse,
  ReprocessResponse,
  RecoverStuckResponse,
  FailedChunkInfo,
  RetryChunksResponse,
  PdfUploadResponse,
  PdfInfo,
  PdfStatusResponse,
  PdfContentResponse,
  PdfProgressResponse,
} from "../types/documents.js";
import type { HttpTransport } from "../transport/types.js";

/** PDF sub-resource accessed via `client.documents.pdf`. */
export class PdfResource extends Resource {
  /** Upload a PDF for extraction. */
  async upload(
    file: File | Blob,
    metadata?: Record<string, string>,
  ): Promise<PdfUploadResponse> {
    return this.transport.upload("/api/v1/documents/pdf", file, metadata);
  }

  /** List uploaded PDFs. */
  async list(): Promise<PdfInfo[]> {
    return this._get("/api/v1/documents/pdf");
  }

  /** Get PDF processing status. */
  async getStatus(pdfId: string): Promise<PdfStatusResponse> {
    return this._get(`/api/v1/documents/pdf/${pdfId}`);
  }

  /** Get extracted PDF content (markdown). */
  async getContent(pdfId: string): Promise<PdfContentResponse> {
    return this._get(`/api/v1/documents/pdf/${pdfId}/content`);
  }

  /** Download original PDF as a Blob. */
  async download(pdfId: string): Promise<Blob> {
    return this.transport.requestBlob({
      method: "GET",
      path: `/api/v1/documents/pdf/${pdfId}/download`,
    });
  }

  /** Get PDF processing progress. */
  async getProgress(trackId: string): Promise<PdfProgressResponse> {
    return this._get(`/api/v1/documents/pdf/progress/${trackId}`);
  }

  /** Retry failed PDF processing. */
  async retry(pdfId: string): Promise<void> {
    await this._post(`/api/v1/documents/pdf/${pdfId}/retry`);
  }

  /** Cancel ongoing PDF processing. */
  async cancel(pdfId: string): Promise<void> {
    await this._del(`/api/v1/documents/pdf/${pdfId}/cancel`);
  }

  /** Delete a PDF. */
  async delete(pdfId: string): Promise<void> {
    await this._del(`/api/v1/documents/pdf/${pdfId}`);
  }
}

/** Documents resource with PDF sub-namespace. */
export class DocumentsResource extends Resource {
  /** PDF sub-resource for PDF-specific operations. */
  readonly pdf: PdfResource;

  constructor(transport: HttpTransport) {
    super(transport);
    this.pdf = new PdfResource(transport);
  }

  /** Upload a document (text/JSON body). */
  async upload(request: UploadDocumentRequest): Promise<UploadDocumentResponse> {
    return this._post("/api/v1/documents", request);
  }

  /**
   * Upload a file (multipart form-data).
   * Accepts File, Blob, or Buffer.
   */
  async uploadFile(file: File | Blob): Promise<UploadFileResponse> {
    return this.transport.upload("/api/v1/documents/upload", file);
  }

  /**
   * Batch upload multiple files.
   * Returns individual status for each file.
   */
  async uploadBatch(files: (File | Blob)[]): Promise<BatchUploadResponse> {
    return this.transport.uploadBatch(
      "/api/v1/documents/upload/batch",
      files,
    );
  }

  /** List documents with optional filters + pagination. */
  list(query?: ListDocumentsQuery): Paginator<DocumentInfo> {
    return new Paginator(
      async (page, perPage) => {
        const params = new URLSearchParams();
        params.set("page", String(page));
        params.set("per_page", String(perPage));
        if (query?.status) params.set("status", query.status);
        if (query?.search) params.set("search", query.search);
        const path = `/api/v1/documents?${params}`;
        return this._get<Page<DocumentInfo>>(path);
      },
      query?.limit ?? 20,
    );
  }

  /** Get document details by ID. */
  async get(documentId: string): Promise<DocumentDetail> {
    return this._get(`/api/v1/documents/${documentId}`);
  }

  /** Delete a specific document. */
  async delete(documentId: string): Promise<void> {
    await this._del(`/api/v1/documents/${documentId}`);
  }

  /** Delete all documents in the workspace. */
  async deleteAll(): Promise<void> {
    await this._del("/api/v1/documents");
  }

  /** Get track status for an async operation. */
  async getTrackStatus(trackId: string): Promise<TrackStatusResponse> {
    return this._get(`/api/v1/documents/track/${trackId}`);
  }

  /** Analyze deletion impact before deleting a document. */
  async analyzeDeletionImpact(
    documentId: string,
  ): Promise<DeletionImpactResponse> {
    return this._get(`/api/v1/documents/${documentId}/deletion-impact`);
  }

  /** Scan a directory for documents to ingest. */
  async scan(request: ScanDirectoryRequest): Promise<ScanDirectoryResponse> {
    return this._post("/api/v1/documents/scan", request);
  }

  /** Reprocess all failed documents. */
  async reprocessFailed(): Promise<ReprocessResponse> {
    return this._post("/api/v1/documents/reprocess");
  }

  /** Recover stuck processing documents. */
  async recoverStuck(): Promise<RecoverStuckResponse> {
    return this._post("/api/v1/documents/recover-stuck");
  }

  /** Retry failed chunks for a specific document. */
  async retryFailedChunks(documentId: string): Promise<RetryChunksResponse> {
    return this._post(`/api/v1/documents/${documentId}/retry-chunks`);
  }

  /** List failed chunks for a specific document. */
  async listFailedChunks(documentId: string): Promise<FailedChunkInfo[]> {
    return this._get(`/api/v1/documents/${documentId}/failed-chunks`);
  }
}
