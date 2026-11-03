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

/**
 * Minimum absolute similarity for a picture to be shown as a match.
 *
 * WHY a floor exists at all: retrieval always returns `top_k` rows, so an image grid built from
 * "what came back" shows pictures even when the question had nothing to do with any of them —
 * twenty photographs appeared under "what does the handbook say about refunds?". Fusion made
 * this unfixable downstream, because the fused `score` is a reciprocal rank: the best and worst
 * rows both score ~0.0164, so nothing in the payload distinguished them. The dense cosine is now
 * carried through as `similarity`, and this floor is what it is for.
 *
 * Measured on a 20-picture corpus embedded with `gemini-embedding-2`, a text query against
 * pictures — i.e. the hardest case, since the query and the vectors are different modalities:
 *
 *   "a golden retriever puppy"   0.715 0.610 0.569 0.530 0.522 … (noise floor ≈ 0.52)
 *   "coffee with latte art"      0.696 0.625 0.557 0.544 0.539
 *   "what does the handbook say" 0.473 0.467 0.466 …, nothing above 0.48
 *
 * Two facts follow. Unrelated pairs never exceed ~0.48, so a floor near that is safe. But the
 * second and third results of a *good* query (0.61, 0.57) are no better than the second and
 * third results of a query with no answer at all — so an absolute floor alone cannot separate
 * "the runner-up" from "noise", and this constant is only half the rule. See
 * {@link isRelevantImageMatch} for the relative half.
 */
export const MIN_IMAGE_SIMILARITY = 0.48;

/**
 * How far below the best match another picture may fall and still be shown.
 *
 * WHY relative and not just absolute: cosine similarity of a text query against images has a
 * high, corpus-dependent floor (~0.52 here), so "0.55" is meaningless without knowing what the
 * best match scored. A query that matched one picture at 0.72 and another at 0.61 has a genuine
 * second answer; a query whose best is 0.47 has none, however many rows came back.
 *
 * 0.10 keeps the runner-ups a person would call the same kind of thing (a beagle beside a
 * golden retriever, an espresso beside a latte) and drops the ones they would not (a bicycle,
 * an ocean wave).
 *
 * 0.15 is the value the measurements choose: with a best of 0.715 it admits 0.610 and 0.569 and
 * stops at 0.530, which is exactly the split a reader makes. 0.10 was too tight (it dropped the
 * beagle, a legitimate second answer) and 0.20 was too loose (it readmitted the bicycle).
 * Being slightly generous costs a thumbnail; being strict hides a real answer, which is worse.
 */
export const IMAGE_SIMILARITY_MARGIN = 0.15;

/**
 * True when a picture is relevant enough to display, judged against the best match.
 *
 * `best` is the highest similarity in the candidate set. An absent `similarity` is treated as
 * **relevant**, not irrelevant: older stored conversations and lexical-only hits do not carry
 * the field, and hiding every picture on those payloads would be a regression dressed up as
 * strictness. Only a measured, low similarity hides a tile.
 */
export function isRelevantImageMatch(chunk: Source, best?: number): boolean {
  const similarity = chunk.similarity;
  if (typeof similarity !== 'number' || !Number.isFinite(similarity)) return true;
  if (similarity < MIN_IMAGE_SIMILARITY) return false;
  if (typeof best !== 'number' || !Number.isFinite(best)) return true;
  return similarity >= best - IMAGE_SIMILARITY_MARGIN;
}

/** Image sources that are both pictures and relevant enough to show. */
export function relevantImageMatches(
  chunks: Chunks | undefined,
  limit = Infinity,
): ImageMatch[] {
  const pictures = imageMatches(chunks);
  // The best match sets the scale for the rest; when nothing carries a similarity, every tile
  // stays, which is the pre-existing behaviour.
  const best = pictures.reduce<number | undefined>((acc, match) => {
    const s = match.chunk.similarity;
    if (typeof s !== 'number' || !Number.isFinite(s)) return acc;
    return acc === undefined ? s : Math.max(acc, s);
  }, undefined);
  return pictures
    .filter((match) => isRelevantImageMatch(match.chunk, best))
    .slice(0, limit);
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
