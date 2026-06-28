/**
 * @module label-utils
 * @description Human-readable formatting for entity labels stored in normalized
 * UPPERCASE_UNDERSCORE format in the knowledge graph backend.
 *
 * @implements UX-AUDIT-030 F-GR-01 — Entity labels must be human-readable
 */

// ─── Canonical entity type color palette ────────────────────────────────────
// Single source of truth used by graph-renderer, graph-legend, and entity-browser.
// WHY: Multiple components had diverged copies causing visual inconsistency.
export const ENTITY_TYPE_COLORS: Record<string, string> = {
  PERSON: '#3b82f6',          // blue-500
  ORGANIZATION: '#10b981',    // emerald-500
  TECHNOLOGY: '#06b6d4',      // cyan-500
  LOCATION: '#f59e0b',        // amber-500
  EVENT: '#ef4444',           // red-500
  CONCEPT: '#8b5cf6',         // violet-500
  DOCUMENT: '#6366f1',        // indigo-500
  PRODUCT: '#f97316',         // orange-500
  LAW: '#64748b',             // slate-500
  REGULATION: '#64748b',      // slate-500
  DEFAULT: '#94a3b8',         // slate-400
};

/**
 * Get the display color for an entity type.
 * Falls back to DEFAULT for unknown types.
 */
export function getEntityTypeColor(entityType: string | undefined): string {
  if (!entityType) return ENTITY_TYPE_COLORS.DEFAULT;
  return ENTITY_TYPE_COLORS[entityType.toUpperCase()] ?? ENTITY_TYPE_COLORS.DEFAULT;
}

// ─── Label formatting ────────────────────────────────────────────────────────

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
    .replace(/_/g, ' ')    .toLowerCase()    .replace(/\b\w/g, (c) => c.toUpperCase());

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
    .toLowerCase()
    .replace(/\b\w/g, (c) => c.toUpperCase());
}
