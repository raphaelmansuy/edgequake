/**
 * SPEC-143 — Page marker SSOT for the WebUI.
 *
 * Grammar must match Rust `PageMarkerWriter` (SPEC-083 X-13):
 *   `<!-- edgequake-page:N -->` (1-indexed).
 *
 * Pure helpers: parse / list / inject DOM anchors for PDF↔Markdown sync.
 */

export const PAGE_MARKER_PREFIX = '<!-- edgequake-page:';
export const PAGE_MARKER_SUFFIX = ' -->';

/** Regex matching a whole-line (or inline) page marker. */
export const PAGE_MARKER_RE = /<!--\s*edgequake-page:(\d+)\s*-->/g;

/**
 * Build a sanitizer-safe DOM anchor for page N.
 * Height comes from `.eq-page-anchor` in globals.css (style attr is stripped).
 */
export function pageAnchorHtml(page: number): string {
  const n = Math.max(1, Math.floor(page));
  // Zero-width space keeps the node non-empty for DOMPurify / IntersectionObserver.
  return `<div data-eq-page="${n}" id="eq-md-page-${n}" class="eq-page-anchor" aria-hidden="true">&#8203;</div>`;
}

/** Parse a single line as a page marker; `undefined` if not a marker. */
export function parsePageMarker(line: string): number | undefined {
  const m = line.trim().match(/<!--\s*edgequake-page:(\d+)\s*-->/);
  if (!m) return undefined;
  const n = parseInt(m[1], 10);
  return Number.isFinite(n) && n >= 1 ? n : undefined;
}

/**
 * List unique page numbers in document order (first occurrence wins).
 */
export function listPageMarkers(markdown: string): number[] {
  const seen = new Set<number>();
  const out: number[] = [];
  const re = new RegExp(PAGE_MARKER_RE.source, 'g');
  let m: RegExpExecArray | null;
  while ((m = re.exec(markdown)) !== null) {
    const n = parseInt(m[1], 10);
    if (!Number.isFinite(n) || n < 1 || seen.has(n)) continue;
    seen.add(n);
    out.push(n);
  }
  return out;
}

export function hasPageMarkers(markdown: string | null | undefined): boolean {
  if (!markdown) return false;
  return listPageMarkers(markdown).length > 0;
}

/**
 * Replace each `<!-- edgequake-page:N -->` with a DOM anchor.
 * Duplicate page numbers keep a single `id` (first wins); later duplicates
 * get `data-eq-page` without id to avoid invalid HTML.
 *
 * The extra newline terminates the CommonMark HTML block. The marker regex
 * does not consume the newline after the comment, so the anchor plus that
 * leftover newline is a blank line. Without it, the next source line (a
 * figure image, heading, or list) is swallowed into the anchor's html token
 * and never rendered.
 */
export function injectPageAnchors(markdown: string): string {
  const seenIds = new Set<number>();
  return markdown.replace(PAGE_MARKER_RE, (_full, pageStr: string) => {
    const n = parseInt(pageStr, 10);
    if (!Number.isFinite(n) || n < 1) return '';
    const anchor = seenIds.has(n)
      ? `<div data-eq-page="${n}" class="eq-page-anchor" aria-hidden="true">&#8203;</div>`
      : pageAnchorHtml(n);
    seenIds.add(n);
    return `${anchor}\n`;
  });
}
