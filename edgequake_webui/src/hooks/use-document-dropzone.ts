/**
 * @module useDocumentDropzone
 * @description Document dropzone configuration with file validation.
 * Extracted from DocumentManager for SRP compliance (OODA-21).
 *
 * WHY: Dropzone setup and validation were inline in DocumentManager.
 * This hook:
 * - Configures react-dropzone with file type and size limits
 * - Shows toast errors for rejected files
 * - Delegates accepted files to upload handler
 *
 * @implements FEAT0001 - Document upload via drag-and-drop
 * @implements BR0301 - File size limit enforcement (10MB)
 */
"use client";

import type { TFunction } from "i18next";
import { useCallback } from "react";
import { useDropzone, type Accept } from "react-dropzone";
import { toast } from "sonner";

/**
 * Maximum file size: 10MB
 */
const MAX_FILE_SIZE = 10 * 1024 * 1024;

/**
 * Accepted file types for document upload.
 */
const ACCEPTED_FILE_TYPES: Accept = {
  "text/plain": [".txt"],
  "text/markdown": [".md"],
  "application/json": [".json"],
  "application/pdf": [".pdf"],
};

/**
 * Options for useDocumentDropzone hook.
 */
export interface UseDocumentDropzoneOptions {
  /** Handler for accepted files */
  onFilesAccepted: (files: File[]) => Promise<void>;
  /** i18n translation function */
  t: TFunction;
}

/**
 * Return type for useDocumentDropzone hook.
 */
export interface UseDocumentDropzoneReturn {
  /** Props to spread on dropzone root element */
  getRootProps: ReturnType<typeof useDropzone>["getRootProps"];
  /** Props to spread on hidden file input */
  getInputProps: ReturnType<typeof useDropzone>["getInputProps"];
  /** Whether a drag is currently active over the dropzone */
  isDragActive: boolean;
  /** Function to programmatically open file dialog */
  openFileDialog: () => void;
}

/**
 * Hook for document upload dropzone with validation.
 *
 * @example
 * ```tsx
 * const { getRootProps, getInputProps, isDragActive, openFileDialog } = useDocumentDropzone({
 *   onFilesAccepted: handleFilesUpload,
 *   t,
 * });
 *
 * return (
 *   <div {...getRootProps()}>
 *     <input {...getInputProps()} />
 *     {isDragActive ? 'Drop files here' : 'Click or drag files'}
 *   </div>
 * );
 * ```
 */
export function useDocumentDropzone(
  options: UseDocumentDropzoneOptions,
): UseDocumentDropzoneReturn {
  const { onFilesAccepted, t } = options;

  const onDrop = useCallback(
    async (
      acceptedFiles: File[],
      fileRejections: readonly {
        file: File;
        errors: readonly { code: string; message: string }[];
      }[],
    ) => {
      // Handle rejected files (too large or wrong type)
      for (const rejection of fileRejections) {
        const errorMessages = rejection.errors
          .map((e) => {
            if (e.code === "file-too-large") {
              const sizeMB = (rejection.file.size / (1024 * 1024)).toFixed(2);
              return t(
                "documents.upload.fileTooLarge",
                'File "{{name}}" is too large ({{size}}MB). Maximum size is 10MB.',
                {
                  name: rejection.file.name,
                  size: sizeMB,
                },
              );
            }
            if (e.code === "file-invalid-type") {
              return t(
                "documents.upload.invalidType",
                'File "{{name}}" has an unsupported format. Supported: TXT, MD, JSON, PDF.',
                {
                  name: rejection.file.name,
                },
              );
            }
            return e.message;
          })
          .join(", ");

        toast.error(errorMessages);
      }

      // Process accepted files
      if (acceptedFiles.length > 0) {
        await onFilesAccepted(acceptedFiles);
      }
    },
    [onFilesAccepted, t],
  );

  const {
    getRootProps,
    getInputProps,
    isDragActive,
    open: openFileDialog,
  } = useDropzone({
    onDrop,
    accept: ACCEPTED_FILE_TYPES,
    maxSize: MAX_FILE_SIZE,
    noClick: false,
  });

  return {
    getRootProps,
    getInputProps,
    isDragActive,
    openFileDialog,
  };
}

export default useDocumentDropzone;
