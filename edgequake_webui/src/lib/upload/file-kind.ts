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

/** Routing kind (API path). Source taxonomy may be finer (e.g. markdown). */
export type UploadFileKind = "pdf" | "image" | "text";

/** True when the file should use the PDF vision upload endpoint. */
export function isPdfUploadFile(file: File): boolean {
  if (file.type === "application/pdf") return true;
  return file.name.toLowerCase().endsWith(".pdf");
}

/** SPEC-086: `.md` / markdown MIME → source_type markdown (still routes as text JSON). */
export function isMarkdownUploadFile(file: File): boolean {
  if (file.type === "text/markdown" || file.type === "text/x-markdown") {
    return true;
  }
  return file.name.toLowerCase().endsWith(".md");
}

/** Taxonomy for progress UI (finer than UploadFileKind routing). */
export type UploadSourceType = "pdf" | "markdown" | "text" | "image" | "unknown";

/**
 * Resolve source_type for presenters from a filename (+ optional MIME).
 * Prefer this over `isPdf ? "pdf" : "markdown"` so `.txt` stays text.
 */
export function sourceTypeFromFileName(
  fileName: string,
  mimeType?: string,
): UploadSourceType {
  const lower = fileName.toLowerCase();
  const mime = (mimeType || "").toLowerCase();
  if (mime === "application/pdf" || lower.endsWith(".pdf")) return "pdf";
  if (
    mime === "text/markdown" ||
    mime === "text/x-markdown" ||
    lower.endsWith(".md")
  ) {
    return "markdown";
  }
  if (
    mime.startsWith("image/") ||
    IMAGE_UPLOAD_EXTENSIONS.some((ext) => lower.endsWith(ext))
  ) {
    return "image";
  }
  if (lower.endsWith(".txt") || mime.startsWith("text/")) return "text";
  return "unknown";
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
