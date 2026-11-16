/**
 * Benchmark for GH-556 — quadratic inline-HTML merge.
 *
 * WHY this file exists:
 *   `mergeInlineHtmlTokens()` used to scan forward from every opening HTML tag to
 *   find its closing tag. When a tag has no closing partner (truncated stream,
 *   malformed model output, a literal `<` in prose) the inner scan still walked
 *   to the end of the token list and the outer loop then restarted the full scan
 *   from the next token — O(N²) in the number of unmatched opening tags.
 *
 * This script runs the *pre-fix* algorithm (kept below, verbatim) and the shipped
 * implementation against identical inputs, so the claim is reproducible and a
 * future regression is visible as a number, not an opinion.
 *
 * Usage: node scripts/tests/bench-556-inline-html-merge.mjs [--sizes=4000,8000]
 */

/** Verbatim copy of the pre-fix implementation, for comparison only. */
function mergeBaseline(tokens) {
  const merged = [];
  let i = 0;
  while (i < tokens.length) {
    const tok = tokens[i];
    const openTagMatch =
      tok.type === 'html' &&
      !tok.raw.startsWith('</') &&
      !tok.raw.endsWith('/>') &&
      /^<([a-z][a-z0-9]*)\b/i.exec(tok.raw);
    if (openTagMatch) {
      const tag = openTagMatch[1].toLowerCase();
      const closingRE = new RegExp(`^</${tag}\\s*>$`, 'i');
      let found = false;
      for (let j = i + 1; j < tokens.length; j++) {
        if (tokens[j].type === 'html' && closingRE.test(tokens[j].raw.trim())) {
          const inner = tokens.slice(i + 1, j).map((t) => t.raw).join('');
          const fullHtml = tok.raw + inner + tokens[j].raw;
          merged.push({
            type: 'html',
            raw: fullHtml,
            text: fullHtml,
            inLink: false,
            inRawBlock: false,
            block: false,
          });
          i = j + 1;
          found = true;
          break;
        }
      }
      if (!found) {
        merged.push(tok);
        i++;
      }
      continue;
    }
    merged.push(tok);
    i++;
  }
  return merged;
}

const { mergeInlineHtmlTokens } = await import(
  '../../src/components/query/markdown/utils/inline-html-merge.ts'
);

const html = (raw) => ({ type: 'html', raw, text: raw, inRawBlock: false, block: false });

const unmatchedOpeningTags = (n) =>
  Array.from({ length: n }, (_, i) => html(`<span class="k${i}">`));

const matchedPairs = (pairs) => {
  const out = [];
  for (let i = 0; i < pairs; i += 1) out.push(html('<sup>'), html(String(i)), html('</sup>'));
  return out;
};

const sizeArg = process.argv.find((a) => a.startsWith('--sizes='));
const sizes = sizeArg
  ? sizeArg.split('=')[1].split(',').map(Number)
  : [4000, 8000, 16000, 32000];

/** Fastest of 3 runs in ms; the minimum is the honest cost of the work. */
function fastestMs(fn, tokens) {
  fn(tokens);
  let best = Number.POSITIVE_INFINITY;
  for (let run = 0; run < 3; run += 1) {
    const start = performance.now();
    fn(tokens);
    best = Math.min(best, performance.now() - start);
  }
  return best;
}

const fmt = (ms) => (ms < 1 ? ms.toFixed(2) : ms.toFixed(1));

console.log('# GH-556 — mergeInlineHtmlTokens: unpaired opening tags');
console.log('');
console.log('| tokens | before (ms) | after (ms) | speedup |');
console.log('|---:|---:|---:|---:|');

let previousBefore = null;
for (const n of sizes) {
  const tokens = unmatchedOpeningTags(n);
  const before = fastestMs(mergeBaseline, tokens);
  const after = fastestMs(mergeInlineHtmlTokens, tokens);
  console.log(
    `| ${n.toLocaleString('en-US')} | ${fmt(before)} | ${fmt(after)} | ${(before / Math.max(after, 0.001)).toFixed(0)}x |`,
  );
  previousBefore = before;
}
void previousBefore;

console.log('');
console.log('# Matched pairs (unchanged work — both implementations merge)');
console.log('');
console.log('| pairs | before (ms) | after (ms) |');
console.log('|---:|---:|---:|');
for (const n of sizes.map((s) => s / 2)) {
  const tokens = matchedPairs(n);
  console.log(`| ${n.toLocaleString('en-US')} | ${fmt(fastestMs(mergeBaseline, tokens))} | ${fmt(fastestMs(mergeInlineHtmlTokens, tokens))} |`);
}

// Correctness cross-check: both implementations must agree on every size.
for (const n of sizes) {
  const tokens = unmatchedOpeningTags(n);
  const before = JSON.stringify(mergeBaseline(tokens).map((t) => t.raw));
  const after = JSON.stringify(mergeInlineHtmlTokens(tokens).map((t) => t.raw));
  if (before !== after) {
    console.error(`PARITY FAILURE at ${n} unpaired tags`);
    process.exit(1);
  }
}
for (const n of sizes.map((s) => s / 2)) {
  const tokens = matchedPairs(n);
  const before = JSON.stringify(mergeBaseline(tokens).map((t) => t.raw));
  const after = JSON.stringify(mergeInlineHtmlTokens(tokens).map((t) => t.raw));
  if (before !== after) {
    console.error(`PARITY FAILURE at ${n} pairs`);
    process.exit(1);
  }
}
console.log('');
console.log('parity: OK (identical output for every size above)');
