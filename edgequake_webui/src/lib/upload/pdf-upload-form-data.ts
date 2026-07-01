/**
 * Build PDF upload FormData — DRY SSOT for field names.
 */

import type { PdfUploadOptions } from "@/types";

export function buildPdfUploadFormData(
  file: File,
  options?: PdfUploadOptions,
): FormData {
  const formData = new FormData();
  formData.append("file", file);

  if (options?.enable_vision !== undefined) {
    formData.append("enable_vision", String(options.enable_vision));
  }
  if (options?.vision_provider) {
    formData.append("vision_provider", options.vision_provider);
  }
  if (options?.vision_model) {
    formData.append("vision_model", options.vision_model);
  }
  if (options?.title) {
    formData.append("title", options.title);
  }
  if (options?.metadata) {
    formData.append("metadata", JSON.stringify(options.metadata));
  }
  if (options?.track_id) {
    formData.append("track_id", options.track_id);
  }
  if (options?.force_reindex !== undefined) {
    formData.append("force_reindex", String(options.force_reindex));
  }
  if (options?.pdf_parser_backend) {
    formData.append("pdf_parser_backend", options.pdf_parser_backend);
  }
  if (options?.process_options) {
    formData.append("process_options", options.process_options);
  } else if (options?.analyze_inline_images) {
    formData.append("process_options", "i");
  }

  return formData;
}
