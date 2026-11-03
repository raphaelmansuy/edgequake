import type { CitationChunk } from "@/components/query/markdown/citation-resolver";
import { buildDocumentCitationUrl } from "@/lib/utils/document-url";

/** Deep link to the cited passage in the document viewer (page + chunk + highlight). */
export function buildCitationHref(chunk: CitationChunk): string {
  return buildDocumentCitationUrl({
    documentId: encodeURIComponent(chunk.document_id),
    chunkId: chunk.chunk_id,
    page: chunk.page_start,
    chunkContent: chunk.content,
    startLine: chunk.start_line,
    endLine: chunk.end_line,
  });
}

export type ClickIntent = "same-tab" | "new-tab" | "preview";

/**
 * Decide what a click on a citation chip should do.
 * - modifier / middle click → new tab (standard link behaviour)
 * - touch / no-hover devices → toggle the preview (there is no hover to reveal it)
 * - otherwise → navigate to the source
 */
export function resolveClickIntent(opts: {
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  button: number;
  canHover: boolean;
}): ClickIntent {
  if (opts.metaKey || opts.ctrlKey || opts.shiftKey || opts.button === 1) {
    return "new-tab";
  }
  return opts.canHover ? "same-tab" : "preview";
}

/** Percent label clamped to 0–100 (raw retrieval scores may exceed 1). */
/**
 * Match strength for a tile, or `null` when the number would be misleading.
 *
 * WHY null: on a normal workspace `score` is an RRF fusion score, not a cosine. With
 * twenty candidates its only possible values are 0.01639…0.01538, so `score * 100` renders
 * "2%" beside a perfect match and "1%" beside an unrelated picture. Both numbers are real
 * and both are meaningless: RRF encodes *rank*, and comparing it to a threshold is exactly
 * the mistake the API docs warn about.
 *
 * A percentage is only shown when it can be read as relatedness — a cosine in [0, 1], which
 * is what a reranker or a dense score produces. Below 0.1 the value is a fusion score, so
 * the tile shows the citation number alone rather than a number that invites the wrong
 * conclusion.
 */
export function formatMatchPercent(score: number): number | null {
  if (!Number.isFinite(score)) return null;
  if (score < 0.1) return null;
  return Math.round(Math.min(1, Math.max(0, score)) * 100);
}

/** True for a plain left click (no modifier) — i.e. not "open in new tab". */
export function isPlainActivation(e: {
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey?: boolean;
  button: number;
}): boolean {
  return !e.metaKey && !e.ctrlKey && !e.shiftKey && !e.altKey && e.button === 0;
}
