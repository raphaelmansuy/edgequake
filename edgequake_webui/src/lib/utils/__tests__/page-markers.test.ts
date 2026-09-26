/**
 * SPEC-143 — Unit tests for page marker parse / inject (U-143-01 / U-143-03).
 */

import { describe, expect, it } from 'bun:test';
import { marked, type Token, type Tokens } from 'marked';
import {
  hasPageMarkers,
  injectPageAnchors,
  listPageMarkers,
  pageAnchorHtml,
  parsePageMarker,
} from '../page-markers';

/** Walk lexer output for image tokens (they live inside paragraphs). */
function imageHrefs(tokens: Token[]): string[] {
  const hrefs: string[] = [];
  const walk = (list: Token[] | undefined) => {
    if (!list) return;
    for (const token of list) {
      if (token.type === 'image') {
        hrefs.push((token as Tokens.Image).href);
      }
      if ('tokens' in token && Array.isArray(token.tokens)) {
        walk(token.tokens);
      }
    }
  };
  walk(tokens);
  return hrefs;
}

describe('parsePageMarker', () => {
  it('parses canonical marker', () => {
    expect(parsePageMarker('<!-- edgequake-page:4 -->')).toBe(4);
  });

  it('allows surrounding whitespace', () => {
    expect(parsePageMarker('  <!-- edgequake-page:1 -->  ')).toBe(1);
  });

  it('rejects non-markers', () => {
    expect(parsePageMarker('<!-- multimodal-chunks -->')).toBeUndefined();
    expect(parsePageMarker('# Heading')).toBeUndefined();
  });
});

describe('listPageMarkers', () => {
  it('lists unique pages in order', () => {
    const md = [
      '# Fixture',
      '<!-- edgequake-page:1 -->',
      'Intro',
      '<!-- edgequake-page:4 -->',
      'Later',
      '<!-- edgequake-page:4 -->',
    ].join('\n');
    expect(listPageMarkers(md)).toEqual([1, 4]);
  });

  it('ignores MM fence comments (U-143-03)', () => {
    const md = [
      '<!-- edgequake-page:1 -->',
      '<!-- multimodal-chunks -->',
      '<!-- edgequake-page:2 -->',
    ].join('\n');
    expect(listPageMarkers(md)).toEqual([1, 2]);
  });
});

describe('injectPageAnchors', () => {
  it('replaces markers with data-eq-page anchors (U-143-01)', () => {
    const md = '<!-- edgequake-page:1 -->\nHello\n<!-- edgequake-page:2 -->\nWorld';
    const out = injectPageAnchors(md);
    expect(out).toContain('data-eq-page="1"');
    expect(out).toContain('id="eq-md-page-1"');
    expect(out).toContain('data-eq-page="2"');
    expect(out).not.toContain('edgequake-page:');
  });

  it('dedupes id on duplicate page markers', () => {
    const out = injectPageAnchors(
      '<!-- edgequake-page:3 -->\na\n<!-- edgequake-page:3 -->\nb',
    );
    const ids = out.match(/id="eq-md-page-3"/g) ?? [];
    expect(ids.length).toBe(1);
    expect((out.match(/data-eq-page="3"/g) ?? []).length).toBe(2);
  });

  it('pageAnchorHtml clamps to >= 1', () => {
    expect(pageAnchorHtml(0)).toContain('data-eq-page="1"');
  });

  it('keeps a figure on the next line as an image token', () => {
    const src = '<!-- edgequake-page:3 -->\n![Page 3](assets/page-0003-fig-02.png)\n';
    const out = injectPageAnchors(src);
    expect(out).toContain('id="eq-md-page-3"');
    const tokens = marked.lexer(out, { gfm: true, breaks: true });
    expect(imageHrefs(tokens)).toEqual(['assets/page-0003-fig-02.png']);
    const html = tokens.find((token) => token.type === 'html');
    expect(html?.raw ?? '').not.toContain('![');
  });
});

describe('hasPageMarkers', () => {
  it('returns false for empty / missing', () => {
    expect(hasPageMarkers(null)).toBe(false);
    expect(hasPageMarkers('')).toBe(false);
    expect(hasPageMarkers('# no markers')).toBe(false);
  });

  it('returns true when markers present', () => {
    expect(hasPageMarkers('<!-- edgequake-page:1 -->\nx')).toBe(true);
  });
});
