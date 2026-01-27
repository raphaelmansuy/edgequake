/**
 * @module DocumentUploadTypes
 * @description Type definitions for document upload functionality
 */

/** Track upload progress and errors for files */
export interface UploadingFile {
  file: File;
  progress: number;
  status: 'pending' | 'reading' | 'uploading' | 'extracting' | 'success' | 'error';
  error?: string;
  phase?: string; // Human-readable phase description
}
