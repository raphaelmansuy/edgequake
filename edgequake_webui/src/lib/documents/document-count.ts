/**
 * SPEC-180 — which count the documents table shows.
 *
 * A workspace with a graph counts entities. One without counts chunks, and never prints
 * "0 entities". One descriptor feeds the header, the cell and the sort, so they agree.
 */
import type { Document } from '@/types';
import type { SortField } from './document-sort';

export type DocumentCountKind = 'entities' | 'chunks';

interface CountColumn {
  field: SortField;
  labelKey: string;
  fallback: string;
}

const COLUMNS: Record<DocumentCountKind, CountColumn> = {
  entities: { field: 'entity_count', labelKey: 'documents.table.entities', fallback: 'Entities' },
  chunks: { field: 'chunk_count', labelKey: 'documents.table.chunks', fallback: 'Chunks' },
};

export function countKindFor(extractsGraph: boolean): DocumentCountKind {
  return extractsGraph ? 'entities' : 'chunks';
}

export function countColumn(kind: DocumentCountKind): CountColumn {
  return COLUMNS[kind];
}

/** `null` means unknown (rendered as a dash); `0` is real data. */
export function documentCount(doc: Document, kind: DocumentCountKind): number | null {
  if (kind === 'chunks') return doc.chunk_count ?? null;
  return doc.entity_count ?? doc.chunk_count ?? null;
}

/** A sort field that only exists for the other kind falls back to the shown count. */
export function sortFieldFor(field: SortField, kind: DocumentCountKind): SortField {
  if (field === 'entity_count' || field === 'chunk_count') return COLUMNS[kind].field;
  return field;
}
