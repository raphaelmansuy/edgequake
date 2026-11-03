/**
 * SPEC-180 — the image sources of an answer, ready for a thumbnail grid.
 *
 * Pure. An image document is one chunk, so a match is a document: duplicates collapse to the
 * best score. `index` is the chunk's position in the answer's sources, so the `[n]` on a tile
 * is the same `[n]` the answer cites.
 */
import type { Chunks } from './group-passages';

/**
 * Extensions, used only for sources that predate `source_modality` (older stored
 * conversations) or that come from a client that did not receive it.
 */
const IMAGE_EXTENSION = /\.(png|jpe?g|webp|gif|bmp|tiff?)$/i;

type Source = Chunks[number];

export interface ImageMatch {
  chunk: Source;
  /** 1-based citation number. */
  index: number;
  title: string;
}

/**
 * Whether a citation is a picture.
 *
 * WHY the server fields come first: `file_path` carries the document *title*, and an image
 * document's title is its caption ("Two cappuccinos with latte art on a wooden table"), with
 * no extension. So an extension test alone answered `false` for every real image, and the
 * "Matching images" grid never rendered outside mocked routes. The API now states
 * `source_modality` (what the chunk was embedded from), `mime_type` and `doc_kind`; those are
 * authoritative. The filename check stays last as a fallback for old payloads.
 */
export function isImageSource(
  chunk: Pick<Source, 'file_path'> & Partial<Pick<Source, 'source_modality' | 'mime_type' | 'doc_kind'>>,
): boolean {
  if (chunk.source_modality === 'image') return true;
  if (chunk.mime_type?.startsWith('image/')) return true;
  if (chunk.doc_kind === 'image') return true;
  return IMAGE_EXTENSION.test(chunk.file_path ?? '');
}

function titleOf(chunk: Source): string {
  const name = chunk.file_path?.split('/').pop();
  return name || chunk.document_id.slice(0, 8);
}

/** Best-scoring image sources, one per document, strongest first. */
export function imageMatches(chunks: Chunks | undefined, limit = Infinity): ImageMatch[] {
  const best = new Map<string, ImageMatch>();
  (chunks ?? []).forEach((chunk, i) => {
    if (!isImageSource(chunk)) return;
    const seen = best.get(chunk.document_id);
    if (seen && seen.chunk.score >= chunk.score) return;
    best.set(chunk.document_id, { chunk, index: i + 1, title: titleOf(chunk) });
  });
  return [...best.values()].sort((a, b) => b.chunk.score - a.chunk.score).slice(0, limit);
}

/** How many image matches the answer has in all. */
export function imageMatchCount(chunks: Chunks | undefined): number {
  return imageMatches(chunks).length;
}

/**
 * Columns for `count` tiles when at most `max` fit in a row, so the rows come out even:
 * 5 tiles in 4 columns become 3 + 2, never 4 + 1.
 */
export function balancedColumns(count: number, max: number): number {
  if (count <= 0) return 1;
  const rows = Math.ceil(count / Math.max(1, max));
  return Math.ceil(count / rows);
}
