/**
 * @module label-utils
 * @description Human-readable formatting for entity labels stored in normalized
 * UPPERCASE_UNDERSCORE format in the knowledge graph backend.
 *
 * @implements UX-AUDIT-030 F-GR-01 — Entity labels must be human-readable
 */

/**
 * Convert a normalized entity name to a human-readable label.
 *
 * Examples:
 *   MARKET_SURVEILLANCE_AUTH → "Market Surveillance Auth"
 *   AB_CARVAL_AVIATION_LEASING_FU → "Ab Carval Aviation Leasing Fu"
 *   TECHNOLOGY → "Technology"
 *   A → "A"
 *
 * @param raw - The raw entity name as stored in the database
 * @param maxLen - Maximum character length before truncation (default 35)
 * @returns Human-readable label, truncated with ellipsis if needed
 */
export function formatEntityLabel(raw: string, maxLen = 35): string {
  if (!raw) return '';

  const formatted = raw
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (c) => c.toUpperCase());

  if (formatted.length <= maxLen) return formatted;
  return formatted.slice(0, maxLen - 1) + '…';
}

/**
 * Format an entity type name for display in the UI.
 * Entity types are stored as ALL_CAPS; this converts to Title Case.
 *
 * Examples:
 *   TECHNOLOGY → "Technology"
 *   ORGANIZATION → "Organization"
 *   CONCEPT → "Concept"
 */
export function formatEntityType(raw: string): string {
  if (!raw) return '';
  return raw
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (c) => c.toUpperCase());
}
