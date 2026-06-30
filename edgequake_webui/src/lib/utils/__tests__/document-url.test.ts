/**
 * @module document-url.test
 * @description Unit tests for buildDocumentPageUrl canonical deeplink helper.
 * @implements SPEC-033 Phase 2 acceptance criteria
 */

import { describe, expect, it } from 'bun:test';
import { buildDocumentCitationUrl, buildDocumentPageUrl } from '../document-url';

describe('buildDocumentPageUrl', () => {
  it('returns path-only when no params provided', () => {
    expect(buildDocumentPageUrl('d1')).toBe('/documents/d1');
  });

  it('includes chunk param when chunkId is provided', () => {
    expect(buildDocumentPageUrl('d1', 'c1')).toBe('/documents/d1?chunk=c1');
  });

  it('includes page param when page ≥ 1', () => {
    expect(buildDocumentPageUrl('d1', undefined, 3)).toBe('/documents/d1?page=3');
  });

  it('includes both chunk and page params', () => {
    expect(buildDocumentPageUrl('d1', 'c1', 3)).toBe('/documents/d1?chunk=c1&page=3');
  });

  it('omits page=0 (treated as no page)', () => {
    expect(buildDocumentPageUrl('d1', 'c1', 0)).toBe('/documents/d1?chunk=c1');
  });

  it('omits negative page values', () => {
    expect(buildDocumentPageUrl('d1', 'c1', -1)).toBe('/documents/d1?chunk=c1');
  });

  it('omits empty chunkId', () => {
    expect(buildDocumentPageUrl('d1', '', 3)).toBe('/documents/d1?page=3');
  });

  it('handles page 1 correctly', () => {
    expect(buildDocumentPageUrl('doc-abc', 'chunk-xyz', 1)).toBe(
      '/documents/doc-abc?chunk=chunk-xyz&page=1',
    );
  });

  it('handles large page numbers', () => {
    expect(buildDocumentPageUrl('d1', undefined, 999)).toBe('/documents/d1?page=999');
  });

  it('preserves document IDs with hyphens (UUID format)', () => {
    const docId = '5d52f10f-1d42-40b5-a41e-d75b3a44f1ae';
    const chunkId = 'chunk-abc123';
    expect(buildDocumentPageUrl(docId, chunkId, 5)).toBe(
      `/documents/${docId}?chunk=${chunkId}&page=5`,
    );
  });
});

describe('buildDocumentCitationUrl', () => {
  it('includes page + chunk + highlight when no line range is provided', () => {
    expect(
      buildDocumentCitationUrl({
        documentId: 'd1',
        chunkId: 'c1',
        page: 7,
        chunkContent: 'Example highlighted content',
      }),
    ).toBe('/documents/d1?chunk=c1&page=7&highlight=Example+highlighted+content');
  });

  it('includes page + line range and omits highlight when line range is present', () => {
    expect(
      buildDocumentCitationUrl({
        documentId: 'd1',
        chunkId: 'c1',
        page: 7,
        chunkContent: 'Example highlighted content',
        startLine: 42,
        endLine: 49,
      }),
    ).toBe('/documents/d1?chunk=c1&page=7&start_line=42&end_line=49');
  });

  it('supports deeplink with page only (no chunk)', () => {
    expect(
      buildDocumentCitationUrl({
        documentId: 'd1',
        page: 18,
      }),
    ).toBe('/documents/d1?page=18');
  });
});
