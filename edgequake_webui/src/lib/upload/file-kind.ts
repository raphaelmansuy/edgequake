/**
 * Upload file classification SSOT (FEAT0203 / SPEC-026 Phase 4).
 *
 * WHY: Binary images must never be read via `File.text()` — they require
 * multipart POST to `/documents/upload` for server-side VLM extraction.
 */

/** MIME prefixes and extensions accepted for image document upload. */
export const IMAGE_UPLOAD_MIME_TYPES = [
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
] as const;

export const IMAGE_UPLOAD_EXTENSIONS = [
  ".png",
  ".jpg",
  ".jpeg",
  ".gif",
  ".webp",
] as const;

export type UploadFileKind = "pdf" | "image" | "text";

/** True when the file should use the PDF vision upload endpoint. */
export function isPdfUploadFile(file: File): boolean {
  if (file.type === "application/pdf") return true;
  return file.name.toLowerCase().endsWith(".pdf");
}

/** True when the file is a raster image for VLM describe-to-text ingest. */
export function isImageUploadFile(file: File): boolean {
  if (file.type.startsWith("image/")) {
    return (IMAGE_UPLOAD_MIME_TYPES as readonly string[]).includes(file.type);
  }
  const lower = file.name.toLowerCase();
  return IMAGE_UPLOAD_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

/** Classify upload routing: PDF → /documents/pdf, image → /documents/upload, else JSON /documents. */
export function classifyUploadFile(file: File): UploadFileKind {
  if (isPdfUploadFile(file)) return "pdf";
  if (isImageUploadFile(file)) return "image";
  return "text";
}
