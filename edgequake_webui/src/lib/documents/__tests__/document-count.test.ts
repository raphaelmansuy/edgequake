import { describe, expect, it } from 'vitest';
import type { Document } from '@/types';
import { countColumn, countKindFor, documentCount, sortFieldFor } from '../document-count';
import { compareDocumentsBySort } from '../document-sort';

const doc = (over: Partial<Document>) => ({ id: 'd', ...over }) as Document;

describe('document count column', () => {
  it('counts entities with a graph and chunks without one', () => {
    expect(countKindFor(true)).toBe('entities');
    expect(countKindFor(false)).toBe('chunks');
    expect(countColumn('chunks').labelKey).toBe('documents.table.chunks');
  });

  it('never reports entities for a chunk workspace', () => {
    expect(documentCount(doc({ entity_count: 0, chunk_count: 12 }), 'chunks')).toBe(12);
    expect(documentCount(doc({}), 'chunks')).toBeNull();
  });

  it('keeps zero as real data', () => {
    expect(documentCount(doc({ entity_count: 0 }), 'entities')).toBe(0);
    expect(documentCount(doc({ chunk_count: 0 }), 'chunks')).toBe(0);
  });

  it('maps a saved count sort onto the column that is shown', () => {
    expect(sortFieldFor('entity_count', 'chunks')).toBe('chunk_count');
    expect(sortFieldFor('chunk_count', 'entities')).toBe('entity_count');
    expect(sortFieldFor('title', 'chunks')).toBe('title');
  });

  it('sorts by chunk count', () => {
    const a = doc({ id: 'a', title: 'a', chunk_count: 2 });
    const b = doc({ id: 'b', title: 'b', chunk_count: 9 });
    expect(compareDocumentsBySort(a, b, 'chunk_count', 'desc')).toBeGreaterThan(0);
  });
});
