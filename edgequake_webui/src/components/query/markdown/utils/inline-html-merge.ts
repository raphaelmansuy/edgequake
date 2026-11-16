/**
 * Inline HTML token merging, extracted from `MarkdownInlineTokens.tsx`.
 *
 * WHY this exists
 * ---------------
 * marked.js splits an inline HTML element across three tokens:
 *
 *   marked.lexer('a <sup>1</sup> b')  →  [text, html('<sup>'), text('1'), html('</sup>'), text]
 *
 * Rendering those individually loses the element: DOMPurify drops the empty
 * `<sup>`, the inner text renders unstyled, and `</sup>` shows up as literal
 * characters. So matching open/close pairs are reassembled into one html token
 * that DOMPurify can sanitise and the DOM can render as a real element.
 *
 * FIRST PRINCIPLES — the cost model this file is built around
 * -----------------------------------------------------------
 * An opening tag with no closing partner is NORMAL, not exceptional: a
 * truncating model stream, a malformed answer, or literal `<` in prose all
 * produce them. The merge therefore must not pay for absence.
 *
 * The previous implementation (GH-556) scanned forward from every opening tag
 * until it ran out of tokens, then advanced one token and scanned again — O(N²)
 * when N opening tags are unclosed (4 000 → 88 ms, 32 000 → 5.5 s, and the cost
 * grows with every streamed render in the chat UI).
 *
 * This implementation answers "where is the closing tag for `T` after index i?"
 * from an index built in one pass, with a per-tag cursor that only moves
 * forward. So:
 *
 *   - no closing tags at all  → one O(N) classification pass, then an early return
 *   - unmatched opening tag   → O(1) rejection, the rest of the list is untouched
 *   - matched pair            → merged once; merged ranges never overlap
 *
 * Total cost is O(total characters), i.e. linear in the answer size.
 *
 * MATCHING SEMANTICS (unchanged from the previous implementation, deliberately)
 * ----------------------------------------------------------------------------
 * For an opening tag `T` at index i, the partner is the *first* later html token
 * whose whole raw text is `</T>` (any case, optional whitespace before `>`).
 * Intermediate tokens are absorbed into the merged token, so this is
 * "same tag name, no nesting analysis" — `<b><b>x</b>` still merges as
 * `<b><b>x</b>` and lets the sanitiser repair it. Preserving that behaviour keeps
 * this a complexity fix, not a rendering change; `inline-html-merge.test.ts`
 * pins it.
 */
import type { Token } from 'marked';

/** `<name ...>` — leading `<`, a tag name, then a non-name boundary. */
const OPENING_TAG = /^<([a-z][a-z0-9]*)\b/i;

/** A html token that is *only* a closing tag: `</name>` with optional spaces. */
const CLOSING_TAG = /^<\/([a-z][a-z0-9]*)\s*>$/i;

/**
 * Tag name of an opening HTML element, lowercased — or `null`.
 *
 * Self-closing (`<br/>`) and closing (`</p>`) tags are rejected: they cannot
 * open a span that needs merging.
 */
export function openingTagName(raw: string): string | null {
  if (raw.startsWith('</') || raw.endsWith('/>')) return null;
  const match = OPENING_TAG.exec(raw);
  return match ? match[1].toLowerCase() : null;
}

/**
 * Tag name of a token that is exactly a closing tag, lowercased — or `null`.
 *
 * Trims first because marked can emit a closing tag with surrounding newline
 * whitespace, which must still count as the pair of a preceding opening tag.
 */
export function closingTagName(raw: string): string | null {
  const match = CLOSING_TAG.exec(raw.trim());
  return match ? match[1].toLowerCase() : null;
}

/**
 * One pass over the tokens, collecting the indices at which each tag name is
 * closed. Lets a lookup for "next `</T>` after i" skip absent names entirely.
 */
function indexClosingTags(tokens: readonly Token[]): Map<string, number[]> {
  const byTag = new Map<string, number[]>();
  for (let i = 0; i < tokens.length; i += 1) {
    const token = tokens[i];
    if (token.type !== 'html') continue;
    const tag = closingTagName(token.raw);
    if (tag === null) continue;
    const list = byTag.get(tag);
    if (list) list.push(i);
    else byTag.set(tag, [i]);
  }
  return byTag;
}

/**
 * Index of the first closing token for `tag` at or after `from`, or -1.
 *
 * `cursors` makes this amortised O(1): the outer loop only ever moves forward,
 * so a cursor never needs to look back, and across the whole merge each entry of
 * each list is visited once.
 */
function nextClosingIndex(
  byTag: Map<string, number[]>,
  cursors: Map<string, number>,
  tag: string,
  from: number,
): number {
  const list = byTag.get(tag);
  if (!list) return -1;

  let cursor = cursors.get(tag) ?? 0;
  while (cursor < list.length && list[cursor] <= from) cursor += 1;
  cursors.set(tag, cursor);

  return cursor < list.length ? list[cursor] : -1;
}

/** Raw text of `tokens[from..to]` as one string, without an intermediate array. */
function concatRaw(tokens: readonly Token[], from: number, to: number): string {
  let raw = '';
  for (let i = from; i <= to; i += 1) raw += tokens[i].raw;
  return raw;
}

/** A single html token standing in for a merged element. */
function mergedHtmlToken(raw: string): Token {
  return {
    type: 'html',
    raw,
    text: raw,
    inLink: false,
    inRawBlock: false,
    block: false,
  } as Token;
}

/**
 * Reassemble inline HTML elements that marked split across tokens.
 *
 * Tokens that are not part of a matched pair are returned untouched (same
 * objects), so an answer full of unclosed tags costs one linear pass and
 * allocates nothing.
 */
export function mergeInlineHtmlTokens(tokens: Token[]): Token[] {
  if (tokens.length < 2) return tokens;

  const byTag = indexClosingTags(tokens);
  // No closing tag anywhere ⇒ nothing can pair, so nothing can merge.
  if (byTag.size === 0) return tokens;

  const cursors = new Map<string, number>();
  const merged: Token[] = [];

  let i = 0;
  while (i < tokens.length) {
    const token = tokens[i];
    const tag = token.type === 'html' ? openingTagName(token.raw) : null;
    if (tag !== null) {
      const closeIndex = nextClosingIndex(byTag, cursors, tag, i);
      if (closeIndex > i) {
        merged.push(mergedHtmlToken(concatRaw(tokens, i, closeIndex)));
        i = closeIndex + 1;
        continue;
      }
    }
    merged.push(token);
    i += 1;
  }

  return merged;
}
