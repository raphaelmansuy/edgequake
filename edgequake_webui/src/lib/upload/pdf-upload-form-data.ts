/**
 * Build PDF upload FormData — DRY SSOT for field names.
 */

import type { PdfUploadOptions } from "@/types";
import { appendSecurityFields } from "@/lib/upload/append-security-fields";
import type { SecurityFields } from "@/lib/security/security-fields";

export function buildPdfUploadFormData(
  file: File,
  options?: PdfUploadOptions & { security?: SecurityFields },
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
  } else if (options?.analyze_inline_images ?? true) {
    formData.append("process_options", "i");
  }
  if (options?.vision_reasoning_effort?.trim()) {
    formData.append(
      "vision_reasoning_effort",
      options.vision_reasoning_effort.trim(),
    );
  }
  if (options?.vision_extract_images !== undefined) {
    formData.append("vision_extract_images", String(options.vision_extract_images));
  }
  if (options?.vision_extract_charts !== undefined) {
    formData.append("vision_extract_charts", String(options.vision_extract_charts));
  }
  if (options?.vision_extract_figures !== undefined) {
    formData.append("vision_extract_figures", String(options.vision_extract_figures));
  }
  if (options?.vision_page_system_prompt !== undefined) {
    formData.append("vision_page_system_prompt", options.vision_page_system_prompt);
  }
  if (options?.vision_image_system_prompt !== undefined) {
    formData.append("vision_image_system_prompt", options.vision_image_system_prompt);
  }
  if (options?.vision_chart_system_prompt !== undefined) {
    formData.append("vision_chart_system_prompt", options.vision_chart_system_prompt);
  }
  if (options?.vision_figure_system_prompt !== undefined) {
    formData.append("vision_figure_system_prompt", options.vision_figure_system_prompt);
  }

  appendSecurityFields(formData, options?.security);

  return formData;
}
