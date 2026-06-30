/**
 * @module document-url
 * @description Canonical deeplink URL builder for document + page citations.
 *
 * @implements SPEC-033 — Single definition of deeplink URL schema.
 * All citation surfaces (hierarchy tree, query citations) MUST use this helper
 * so the URL schema is defined in exactly one place (DRY principle).
 *
 * URL schema: /documents/{docId}?chunk={chunkId}&page={pageN}
 *
 * @example
 * buildDocumentPageUrl('doc-1', 'chunk-abc', 3)
 * // → '/documents/doc-1?chunk=chunk-abc&page=3'
 *
 * buildDocumentPageUrl('doc-1', undefined, 3)
 * // → '/documents/doc-1?page=3'
 *
 * buildDocumentPageUrl('doc-1')
 * // → '/documents/doc-1'
 */

/**
 * Build a canonical document viewer URL with optional chunk + page params.
 *
 * Rules:
 * - `page` values < 1 are omitted (treated as "no page").
 * - `chunkId` is omitted when undefined or empty.
 * - Parameter order is always chunk first, then page.
 *
 * @param docId     - Document UUID
 * @param chunkId   - Optional chunk UUID for chunk highlight
 * @param page      - Optional 1-indexed PDF page number for viewer navigation
 */
export function buildDocumentPageUrl(
  docId: string,
  chunkId?: string,
  page?: number,
): string {
  const params = new URLSearchParams();
  if (chunkId) params.set('chunk', chunkId);
  if (page !== undefined && page >= 1) params.set('page', String(page));
  const qs = params.toString();
  return `/documents/${docId}${qs ? `?${qs}` : ''}`;
}

/**
 * Build a citation deeplink preserving line-range, chunk selection and optional page navigation.
 *
 * Behavior mirrors citation click UX:
 * - line range is added only when both start/end are present
 * - highlight is used only when no explicit line range is available
 */
export function buildDocumentCitationUrl({
  documentId,
  chunkId,
  page,
  chunkContent,
  startLine,
  endLine,
}: {
  documentId: string;
  chunkId?: string;
  page?: number;
  chunkContent?: string;
  startLine?: number;
  endLine?: number;
}): string {
  const baseUrl = buildDocumentPageUrl(documentId, chunkId, page);
  const [path, existingQuery = ''] = baseUrl.split('?');
  const params = new URLSearchParams(existingQuery);

  if (startLine !== undefined && endLine !== undefined) {
    params.set('start_line', startLine.toString());
    params.set('end_line', endLine.toString());
  }

  if (chunkContent && startLine === undefined) {
    params.set('highlight', chunkContent.slice(0, 100));
  }

  const queryString = params.toString();
  return `${path}${queryString ? `?${queryString}` : ''}`;
}
