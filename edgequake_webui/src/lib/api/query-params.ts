/**
 * @module query-params
 * @description Shared helper for building URL query strings from optional
 * param objects. Eliminates the repeated `new URLSearchParams()` +
 * `if (params?.x) searchParams.set(...)` boilerplate across domain API
 * modules (SPEC-017 UI-DRY-001).
 */

/**
 * Build a query string from a record of optional values.
 *
 * - `undefined` and `null` values are skipped.
 * - `false` and `0` are kept (they are valid query values).
 * - Arrays are serialized as repeated keys (`?key=a&key=b`).
 * - Returns `"?key=value&..."` (with leading `?`) or `""` when empty.
 *
 * @example
 * buildQueryString({ page: 1, status: undefined, tags: ["a", "b"] })
 * // → "?page=1&tags=a&tags=b"
 */
export function buildQueryString(params: Record<string, unknown> | undefined): string {
  if (!params) return "";
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null) continue;
    if (Array.isArray(value)) {
      for (const item of value) {
        if (item !== undefined && item !== null) {
          searchParams.append(key, String(item));
        }
      }
    } else {
      searchParams.set(key, String(value));
    }
  }
  const query = searchParams.toString();
  return query ? `?${query}` : "";
}

/**
 * Append a query string to a path, handling the `?` separator.
 *
 * @example
 * withQuery("/documents", "?page=1")   // → "/documents?page=1"
 * withQuery("/documents", "")          // → "/documents"
 */
export function withQuery(path: string, query: string): string {
  if (!query) return path;
  return `${path}${query}`;
}

export default buildQueryString;
