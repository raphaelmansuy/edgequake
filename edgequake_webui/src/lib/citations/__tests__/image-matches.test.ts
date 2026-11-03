import { describe, expect, it } from 'bun:test';
import {
  MIN_IMAGE_SIMILARITY,
  balancedColumns,
  imageMatchCount,
  imageMatches,
  isImageSource,
  isRelevantImageMatch,
  relevantImageMatches,
} from '../image-matches';

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

  it('recognizes a picture from the facts the server states', () => {
    // WHY: `file_path` holds the document title, and an image document's title is its
    // caption — no extension. Every real image was therefore reported as "not an image",
    // and the matching-images grid never rendered outside mocked routes.
    const caption = 'Two cappuccinos with latte art on a wooden table';
    expect(isImageSource({ file_path: caption, source_modality: 'image' })).toBe(true);
    expect(isImageSource({ file_path: caption, mime_type: 'image/jpeg' })).toBe(true);
    expect(isImageSource({ file_path: caption, doc_kind: 'image' })).toBe(true);
    // The same caption with no facts and no extension: unknown, and the old rule says no.
    expect(isImageSource({ file_path: caption })).toBe(false);
    // A text document must not be mistaken for a picture by its title.
    expect(
      isImageSource({
        file_path: 'A photo of a cat, described',
        doc_kind: 'markdown',
        mime_type: 'text/markdown',
        source_modality: 'text',
      }),
    ).toBe(false);
  });

  it('finds a captioned picture among text sources', () => {
    const matches = imageMatches([
      { ...chunk('d1', 'Orchard notes', 0.4), doc_kind: 'text', source_modality: 'text' },
      {
        ...chunk('d2', 'A beagle with its tongue out', 0.9),
        doc_kind: 'image',
        source_modality: 'image',
      },
    ]);
    expect(matches.map((m) => m.title)).toEqual(['A beagle with its tongue out']);
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

describe('SPEC-183 relevance gate', () => {
  /** A picture row as the API now states it: fused rank in `score`, cosine in `similarity`. */
  const picture = (document_id: string, similarity?: number) => ({
    content: '',
    document_id,
    file_path: document_id,
    source_modality: 'image',
    // Fusion always puts the rank near 1/61, so `score` cannot separate a good match from a bad.
    score: 0.016_393_442,
    ...(similarity === undefined ? {} : { similarity }),
  });

  it('drops the noise but keeps the runner-up a person would accept', () => {
    // Real scores measured on "a golden retriever puppy" over 20 photographs.
    const chunks = [
      picture('golden-retriever', 0.715),
      picture('beagle', 0.61),
      picture('orange-poppies', 0.569),
      picture('ocean-wave', 0.53),
      picture('vintage-bicycle', 0.522),
    ];
    // With a best of 0.715 the 0.15 margin admits 0.610 and 0.569 and stops at 0.530, which is
    // the split a reader makes: a beagle and flowers beside a retriever are plausible answers,
    // an ocean wave and a bicycle are not.
    const shown = relevantImageMatches(chunks).map((m) => m.chunk.document_id);
    expect(shown).toEqual(['golden-retriever', 'beagle', 'orange-poppies']);
    expect(shown).not.toContain('ocean-wave');
    expect(shown).not.toContain('vintage-bicycle');
  });

  it('shows nothing when no picture is a real match', () => {
    // Real scores measured for "what does the handbook say about refunds?" — a question this
    // picture corpus cannot answer. Nothing exceeded 0.48, so nothing may appear.
    const chunks = [
      picture('orange-poppies', 0.473),
      picture('golden-retriever', 0.467),
      picture('man-portrait', 0.466),
      picture('beagle', 0.459),
    ];
    expect(relevantImageMatches(chunks)).toEqual([]);
    // The unfiltered view still knows they came back, so the count can stay honest.
    expect(imageMatches(chunks)).toHaveLength(4);
  });

  it('uses the best match as the scale, not an absolute number alone', () => {
    const best = picture('best', 0.72);
    // 0.63 is poor in absolute terms but is a real second answer beside a 0.72 top hit.
    expect(isRelevantImageMatch(picture('second', 0.63), 0.72)).toBe(true);
    // The same 0.63 is noise when the best match was 0.55.
    expect(isRelevantImageMatch(picture('noise', 0.63), 0.55)).toBe(true);
    // And a weak set is rejected outright by the absolute floor.
    expect(isRelevantImageMatch(picture('weak', 0.47), 0.47)).toBe(false);
    expect(best.similarity).toBe(0.72);
  });

  it('keeps a picture at the absolute floor', () => {
    expect(isRelevantImageMatch(picture('a', MIN_IMAGE_SIMILARITY))).toBe(true);
  });

  it('fails open when no similarity was measured', () => {
    // Older stored conversations and lexical-only hits carry no `similarity`. Hiding those
    // pictures would be a regression disguised as strictness.
    expect(isRelevantImageMatch(picture('legacy'))).toBe(true);
    expect(relevantImageMatches([picture('legacy')])).toHaveLength(1);
    // A mix of measured and unmeasured keeps the unmeasured ones.
    expect(relevantImageMatches([picture('a', 0.8), picture('legacy')])).toHaveLength(2);
  });

  it('never filters a text chunk, only pictures', () => {
    const text = {
      content: 'Refunds are accepted within 30 days.',
      document_id: 'handbook',
      file_path: 'handbook.pdf',
      score: 0.016,
      similarity: 0.05,
    };
    expect(relevantImageMatches([text])).toHaveLength(0);
  });
});
