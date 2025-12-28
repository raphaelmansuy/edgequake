/**
 * Test file for normalizeMarkdownForStreaming
 * 
 * Run with: node test-normalize-markdown.mjs
 */

import { marked } from 'marked';

// Copy of the normalization function with all fixes
function normalizeMarkdownForStreaming(content) {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let normalized = content;

  // ═══════════════════════════════════════════════════════════════════
  // BOLD (**text**)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 1: **text ** (trailing space before closing) → **text**
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  
  // Pattern 2: ** text** (leading space after opening) → **text**
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)\*\* +([^*]+?)\*\*/g, '**$1**');
  
  // Pattern 3: Re-run trailing pattern for "** text **" case
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');

  // ═══════════════════════════════════════════════════════════════════
  // ITALIC (*text*)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 1: *text * (trailing space before closing) → *text*
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');
  
  // Pattern 2: * text* (leading space after opening) → *text*
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)(?<!\*)\* +([^*]+?)\*(?!\*)/g, '*$1*');
  
  // Pattern 3: Re-run trailing for "* text *" case
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');

  // ═══════════════════════════════════════════════════════════════════
  // STRIKETHROUGH (~~text~~)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 1: ~~text ~~ (trailing space) → ~~text~~
  normalized = normalized.replace(/~~([^\s~][^~]*?) +~~/g, '~~$1~~');
  
  // Pattern 2: ~~ text~~ (leading space) → ~~text~~
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)~~ +([^~]+?)~~/g, '~~$1~~');
  
  // Pattern 3: Re-run trailing
  normalized = normalized.replace(/~~([^\s~][^~]*?) +~~/g, '~~$1~~');

  // ═══════════════════════════════════════════════════════════════════
  // INLINE CODE (`text`)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 1: `text ` (trailing space) → `text`
  normalized = normalized.replace(/`([^\s`][^`]*?) +`/g, '`$1`');
  
  // Pattern 2: ` text` (leading space) → `text`
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)` +([^`]+?)`/g, '`$1`');
  
  // Pattern 3: Re-run trailing
  normalized = normalized.replace(/`([^\s`][^`]*?) +`/g, '`$1`');

  return normalized;
}

// Test cases
const tests = [
  // Bold tests
  { input: '**Products **:', expected: '**Products**:', name: 'Bold: trailing space before closing' },
  { input: '** Products**:', expected: '**Products**:', name: 'Bold: leading space after opening' },
  { input: '** Products **:', expected: '**Products**:', name: 'Bold: spaces on both sides' },
  { input: '**Products**:', expected: '**Products**:', name: 'Bold: no spaces - unchanged' },
  { input: 'text **Bold ** and **more **', expected: 'text **Bold** and **more**', name: 'Bold: multiple in sentence' },
  
  // Italic tests
  { input: '*italic *:', expected: '*italic*:', name: 'Italic: trailing space before closing' },
  { input: '* italic*:', expected: '*italic*:', name: 'Italic: leading space after opening' },
  { input: '* italic *:', expected: '*italic*:', name: 'Italic: spaces on both sides' },
  { input: '*italic*:', expected: '*italic*:', name: 'Italic: no spaces - unchanged' },
  
  // Strikethrough tests
  { input: '~~strike ~~', expected: '~~strike~~', name: 'Strikethrough: trailing space' },
  { input: '~~ strike~~', expected: '~~strike~~', name: 'Strikethrough: leading space' },
  { input: '~~ strike ~~', expected: '~~strike~~', name: 'Strikethrough: both sides' },
  
  // Inline code tests
  { input: '`code `', expected: '`code`', name: 'Code: trailing space' },
  { input: '` code`', expected: '`code`', name: 'Code: leading space' },
  { input: '` code `', expected: '`code`', name: 'Code: both sides' },
  
  // Real LLM output examples
  { input: '- **Products **:\n  - Product A\n  - Product B', 
    expected: '- **Products**:\n  - Product A\n  - Product B', 
    name: 'Real: List with bold header' },
];

console.log('Testing normalizeMarkdownForStreaming:');
console.log('='.repeat(70));

let allPassed = true;
let passCount = 0;
let failCount = 0;

tests.forEach(({ input, expected, name }) => {
  const result = normalizeMarkdownForStreaming(input);
  const passed = result === expected;
  if (passed) {
    passCount++;
  } else {
    failCount++;
    allPassed = false;
  }
  console.log(`\n${passed ? '✅' : '❌'} ${name}`);
  if (!passed) {
    console.log(`   Input:    ${JSON.stringify(input)}`);
    console.log(`   Expected: ${JSON.stringify(expected)}`);
    console.log(`   Got:      ${JSON.stringify(result)}`);
  }
});

// Now test that marked parses the normalized content correctly
console.log('\n' + '='.repeat(70));
console.log('Testing marked.lexer() detects bold after normalization:');
console.log('='.repeat(70));

const boldTests = [
  { input: '**Products **:', name: 'Bold: trailing space' },
  { input: '** Products**:', name: 'Bold: leading space' },
  { input: '** Products **:', name: 'Bold: both sides' },
  { input: '**Products**:', name: 'Bold: correct syntax' },
];

boldTests.forEach(({ input, name }) => {
  const normalized = normalizeMarkdownForStreaming(input);
  const tokens = marked.lexer(normalized);
  const hasBold = JSON.stringify(tokens).includes('"type":"strong"');
  console.log(`\n${hasBold ? '✅' : '❌'} ${name}`);
  console.log(`   Original:   ${JSON.stringify(input)}`);
  console.log(`   Normalized: ${JSON.stringify(normalized)}`);
  console.log(`   Bold found: ${hasBold}`);
});

console.log('\n' + '='.repeat(70));
console.log(`Results: ${passCount} passed, ${failCount} failed`);
console.log(allPassed ? '✅ ALL NORMALIZATION TESTS PASSED!' : '❌ SOME TESTS FAILED');
console.log('='.repeat(70));

// Exit with error code if tests failed
process.exit(allPassed ? 0 : 1);
