import { describe, expect, it } from 'bun:test';
import { balancedColumns, imageMatchCount, imageMatches, isImageSource } from '../image-matches';

const chunk = (document_id: string, file_path: string, score: number) => ({
  content: '',
  document_id,
  file_path,
  score,
});

describe('image matches', () => {
  it('recognizes image files by extension, case-insensitively', () => {
    expect(isImageSource({ file_path: 'a/photo.PNG' })).toBe(true);
    expect(isImageSource({ file_path: 'scan.jpeg' })).toBe(true);
    expect(isImageSource({ file_path: 'handbook.pdf' })).toBe(false);
    expect(isImageSource({})).toBe(false);
  });

  it('keeps images only, strongest first, with the answer citation number', () => {
    const matches = imageMatches([
      chunk('d1', 'handbook.pdf', 0.9),
      chunk('d2', 'photo.png', 0.7),
      chunk('d3', 'wall.jpg', 0.8),
    ]);
    expect(matches.map((m) => [m.title, m.index])).toEqual([
      ['wall.jpg', 3],
      ['photo.png', 2],
    ]);
  });

  it('collapses a document to its best chunk', () => {
    const matches = imageMatches([chunk('d1', 'a.png', 0.4), chunk('d1', 'a.png', 0.9)]);
    expect(matches).toHaveLength(1);
    expect(matches[0]!.chunk.score).toBe(0.9);
    expect(matches[0]!.index).toBe(2);
  });

  it('applies the limit and counts all matches', () => {
    const chunks = ['a', 'b', 'c'].map((d, i) => chunk(d, `${d}.png`, 0.5 + i / 10));
    expect(imageMatches(chunks, 2)).toHaveLength(2);
    expect(imageMatchCount(chunks)).toBe(3);
    expect(imageMatchCount(undefined)).toBe(0);
  });
});

describe('balancedColumns', () => {
  it('keeps one row when everything fits', () => {
    expect(balancedColumns(1, 4)).toBe(1);
    expect(balancedColumns(3, 4)).toBe(3);
    expect(balancedColumns(4, 4)).toBe(4);
  });

  it('evens out the rows instead of leaving a single orphan', () => {
    expect(balancedColumns(5, 4)).toBe(3);
    expect(balancedColumns(6, 4)).toBe(3);
    expect(balancedColumns(7, 4)).toBe(4);
    expect(balancedColumns(5, 3)).toBe(3);
  });

  it('is safe on empty or degenerate input', () => {
    expect(balancedColumns(0, 4)).toBe(1);
    expect(balancedColumns(3, 0)).toBe(1);
  });
});
