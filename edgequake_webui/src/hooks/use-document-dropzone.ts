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
 * @implements BR0301 - File size limit enforcement (mirrors backend MAX_UPLOAD_BYTES)
 */
"use client";

import type { TFunction } from "i18next";
import { useCallback } from "react";
import { useDropzone, type Accept } from "react-dropzone";
import { toast } from "sonner";
import { MAX_UPLOAD_BYTES, MAX_UPLOAD_LABEL } from "@/lib/api/upload-limits";
import { splitByImageCapability } from "@/lib/upload/file-kind";
import type { ImageCapability } from "@/lib/documents/image-upload-guidance";

/**
 * Maximum file size for uploads. Sourced from `upload-limits.ts` (the
 * client-side SSOT that mirrors the backend `MAX_UPLOAD_BYTES`). Override
 * via `NEXT_PUBLIC_MAX_UPLOAD_BYTES` for deployments that raise the backend
 * limit via `EDGEQUAKE_MAX_UPLOAD_BYTES`.
 */
const MAX_FILE_SIZE = MAX_UPLOAD_BYTES;

/**
 * Accepted file types for document upload.
 * @implements FEAT0203 - Image document upload (PNG, JPG, GIF, WEBP)
 */
const ACCEPTED_FILE_TYPES: Accept = {
  "text/plain": [".txt"],
  "text/markdown": [".md"],
  "application/json": [".json"],
  "application/pdf": [".pdf"],
  "image/png": [".png"],
  "image/jpeg": [".jpg", ".jpeg"],
  "image/gif": [".gif"],
  "image/webp": [".webp"],
};

/**
 * Options for useDocumentDropzone hook.
 */
export interface UseDocumentDropzoneOptions {
  /** Handler for accepted files */
  onFilesAccepted: (files: File[]) => Promise<void>;
  /** i18n translation function */
  t: TFunction;
  /**
   * SPEC-183 — the workspace's image capability, as the server stated it. `embeds_images: false`
   * refuses pictures here with a sentence that names the fix, instead of travelling to the
   * server to be told `embedding_model_not_multimodal` with nowhere to go. Absent means the
   * server has not said, and the upload proceeds: the server still answers correctly.
   */
  acceptsImages?: ImageCapability;
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
  const { onFilesAccepted, t, acceptsImages } = options;

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
                'File "{{name}}" is too large ({{size}}MB). Maximum size is {{limit}}.',
                {
                  name: rejection.file.name,
                  size: sizeMB,
                  limit: MAX_UPLOAD_LABEL,
                },
              );
            }
            if (e.code === "file-invalid-type") {
              const lower = rejection.file.name.toLowerCase();
              const isOffice =
                /\.(docx?|docm|xlsx?|xlsm)$/i.test(lower) ||
                /officedocument|ms-excel|msword/i.test(rejection.file.type);
              if (isOffice) {
                return t(
                  "documents.upload.officeNotSupported",
                  'File "{{name}}" is a Word/Excel file. Those formats are not supported. Use PDF, Markdown, text, JSON, or an image (PNG/JPG/GIF/WEBP).',
                  { name: rejection.file.name },
                );
              }
              return t(
                "documents.upload.invalidType",
                'File "{{name}}" has an unsupported format. Supported: TXT, MD, JSON, PDF, PNG, JPG, GIF, WEBP. DOCX/Excel are not supported.',
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

      if (acceptedFiles.length === 0) return;

      // Fail open on an unknown capability; refuse only what the server said it cannot store.
      if (acceptsImages?.embeds_images !== false) {
        await onFilesAccepted(acceptedFiles);
        return;
      }
      // SPEC-183: the evidence is immutable, so an image cannot be stored here at all. Name the
      // workspace shape that can, rather than repeating the server's code.
      const { text, images } = splitByImageCapability(acceptedFiles);
      if (images.length > 0) {
        const names = images.map((file) => file.name).join(", ");
        toast.error(
          t(
            "documents.upload.imagesNeedImageWorkspace",
            '{{count}} picture(s) were not uploaded: this workspace stores text only. Pictures need a workspace created with evidence “Text and images”.',
            { count: images.length, names },
          ),
          { description: names },
        );
      }
      if (text.length > 0) {
        await onFilesAccepted(text);
      }
    },
    [onFilesAccepted, t, acceptsImages?.embeds_images],
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
    // WHY: We take explicit control of click behavior for cross-browser reliability.
    // react-dropzone's internal click handler can silently fail with the File System
    // Access API (showOpenFilePicker) in non-secure contexts, certain Chrome versions,
    // and cross-origin iframes. See: https://github.com/react-dropzone/react-dropzone/issues/1127
    noClick: true,
    // WHY: Force traditional <input> click instead of showOpenFilePicker() API.
    // The FS Access API fails silently in HTTP contexts, specific Chrome versions,
    // and when MIME types don't match showOpenFilePicker's strict format requirements.
    // See: https://github.com/react-dropzone/react-dropzone/issues/1349
    useFsAccessApi: false,
    onError: (err) => {
      console.error("[Dropzone] Error:", err);
      toast.error(
        t(
          "documents.upload.dropzoneError",
          "File upload error. Please try again or use the upload button.",
        ),
      );
    },
  });

  return {
    getRootProps,
    getInputProps,
    isDragActive,
    openFileDialog,
  };
}

export default useDocumentDropzone;
