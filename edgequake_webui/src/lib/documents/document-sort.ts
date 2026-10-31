/**
 * Document list sort SSOT (first principles).
 *
 * Identity: one `(field, direction)` controls row order everywhere —
 * toolbar shortcuts and table column headers share this module (DRY).
 */

import type { Document } from "@/types";

export type SortField =
  | "created_at"
  | "updated_at"
  | "title"
  | "status"
  | "entity_count"
  | "chunk_count"
  | "cost_usd";

export type SortDirection = "asc" | "desc";

export const DOCUMENT_SORT_FIELDS: readonly SortField[] = [
  "created_at",
  "updated_at",
  "title",
  "status",
  "entity_count",
  "chunk_count",
  "cost_usd",
] as const;

export function isSortField(value: unknown): value is SortField {
  return (
    typeof value === "string" &&
    (DOCUMENT_SORT_FIELDS as readonly string[]).includes(value)
  );
}

export function isSortDirection(value: unknown): value is SortDirection {
  return value === "asc" || value === "desc";
}

/** Next sort state when a column/control is activated (WAI-ARIA toggle pattern). */
export function nextDocumentSortState(
  currentField: SortField,
  currentDirection: SortDirection,
  clickedField: SortField,
): { field: SortField; direction: SortDirection } {
  if (currentField === clickedField) {
    return {
      field: currentField,
      direction: currentDirection === "asc" ? "desc" : "asc",
    };
  }
  // New column: default descending for metrics/dates, ascending for title/status.
  const direction: SortDirection =
    clickedField === "title" || clickedField === "status" ? "asc" : "desc";
  return { field: clickedField, direction };
}

function documentTitleKey(doc: Document): string {
  return (doc.title || doc.file_name || doc.id || "").toLowerCase();
}

function documentTimeMs(value: string | undefined): number {
  if (!value) return 0;
  const ms = Date.parse(value);
  return Number.isFinite(ms) ? ms : 0;
}

/** Comparable sort key for a document + field (SRP: one place for field→value). */
export function documentSortValue(
  doc: Document,
  field: SortField,
): string | number {
  switch (field) {
    case "title":
      return documentTitleKey(doc);
    case "created_at":
      return documentTimeMs(doc.created_at);
    case "updated_at":
      return documentTimeMs(doc.updated_at || doc.created_at);
    case "status":
      return (doc.status || "").toLowerCase();
    case "entity_count":
      return doc.entity_count ?? doc.chunk_count ?? 0;
    case "chunk_count":
      return doc.chunk_count ?? 0;
    case "cost_usd":
      return doc.cost_usd ?? 0;
  }
}

export function compareDocumentsBySort(
  a: Document,
  b: Document,
  field: SortField,
  direction: SortDirection,
): number {
  const aVal = documentSortValue(a, field);
  const bVal = documentSortValue(b, field);
  if (aVal < bVal) return direction === "asc" ? -1 : 1;
  if (aVal > bVal) return direction === "asc" ? 1 : -1;
  // Stable secondary key
  return documentTitleKey(a).localeCompare(documentTitleKey(b));
}

export function sortDocuments(
  docs: Document[],
  field: SortField,
  direction: SortDirection,
): Document[] {
  return [...docs].sort((a, b) => compareDocumentsBySort(a, b, field, direction));
}

/** `aria-sort` value for a column header (only active column gets ascending/descending). */
export function ariaSortForColumn(
  column: SortField,
  activeField: SortField,
  direction: SortDirection,
): "ascending" | "descending" | "none" {
  if (column !== activeField) return "none";
  return direction === "asc" ? "ascending" : "descending";
}
