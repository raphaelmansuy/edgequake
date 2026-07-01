/**
 * Large PDF admission helpers (SPEC-038) — mirrors backend LargeDocumentProfile thresholds.
 */

import { extractPdfPageCount } from "./extract-page-count";

export const LARGE_PDF_PAGE_THRESHOLD = Number.parseInt(
  process.env.NEXT_PUBLIC_LARGE_PDF_PAGE_THRESHOLD ?? "100",
  10,
);

export type PdfParserChoice = "default" | "edgeparse" | "vision";

export interface LargePdfAdmissionPreview {
  file: File;
  pageCount: number;
  fileSizeBytes: number;
  recommendedBackend: "edgeparse" | "vision";
  estimatedMinutes: number;
}

/** Rough ETA: EdgeParse ~0.5s/page + extract waves; Vision ~8s/page cloud. */
export function estimateIngestMinutes(pageCount: number, backend: "edgeparse" | "vision"): number {
  const convertSecs =
    backend === "edgeparse"
      ? 60 + pageCount / 2
      : 120 + pageCount * 8;
  const extractSecs = Math.ceil(pageCount / 16) * 25;
  return Math.ceil((convertSecs + extractSecs + 600) / 60);
}

export async function buildLargePdfAdmissionPreview(
  file: File,
): Promise<LargePdfAdmissionPreview | null> {
  const buffer = await file.arrayBuffer();
  const pageCount = extractPdfPageCount(buffer);
  if (pageCount === null || pageCount < LARGE_PDF_PAGE_THRESHOLD) {
    return null;
  }
  const recommendedBackend: "edgeparse" | "vision" = "edgeparse";
  return {
    file,
    pageCount,
    fileSizeBytes: file.size,
    recommendedBackend,
    estimatedMinutes: estimateIngestMinutes(pageCount, recommendedBackend),
  };
}

export async function filterLargePdfFiles(files: File[]): Promise<LargePdfAdmissionPreview[]> {
  const previews: LargePdfAdmissionPreview[] = [];
  for (const file of files) {
    if (!file.name.toLowerCase().endsWith(".pdf")) continue;
    const preview = await buildLargePdfAdmissionPreview(file);
    if (preview) previews.push(preview);
  }
  return previews;
}
