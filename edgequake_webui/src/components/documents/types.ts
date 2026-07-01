/**
 * @module DocumentUploadTypes
 * @description Type definitions for document upload functionality
 */

/** Track upload progress and errors for files */
export interface UploadingFile {
  file: File;
  progress: number;
  status:
    | "pending"
    | "reading"
    | "uploading"
    | "extracting"
    | "success"
    | "error";
  error?: string;
  phase?: string; // Human-readable phase description
  /** Bytes sent during HTTP transfer (SPEC-038 honest upload progress). */
  bytesSent?: number;
  bytesTotal?: number;
  uploadPhase?: "transfer" | "admit";
  /** OODA-22: Track ID for PDF progress monitoring */
  trackId?: string;
  /** OODA-22: Whether this is a PDF file (for enhanced progress) */
  isPdf?: boolean;
}
