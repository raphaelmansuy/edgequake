#!/usr/bin/env node
/**
 * Normalization Function Test
 * 
 * Tests the normalizeMarkdownForStreaming patterns against various inputs
 * to identify if the normalization is correctly fixing issues or introducing new ones.
 */

// Copy of the normalizeMarkdownForStreaming function
function normalizeMarkdownForStreaming(content) {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let normalized = content;

  // Pattern 0: word** text → word **text (marker attached to previous word)
  normalized = normalized.replace(/([a-zA-Z0-9])\*\* (\w)/g, '$1 **$2');
  
  // Pattern 1: **text ** (trailing space before closing) → **text**
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  
  // Pattern 2: ** text** (leading space after opening) → **text**
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)\*\* +([^*]+?)\*\*/g, '**$1**');
  
  // Pattern 3: Re-run trailing pattern for "** text **" case
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');

  // ITALIC patterns
  normalized = normalized.replace(/([a-zA-Z0-9])(?<!\*)\* (\w)/g, '$1 *$2');
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)(?<!\*)\* +([^*]+?)\*(?!\*)/g, '*$1*');
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');

  // UNDERSCORE BOLD patterns
  normalized = normalized.replace(/([a-zA-Z0-9])__ (\w)/g, '$1 __$2');
  normalized = normalized.replace(/__([^\s_][^_]*?) +__/g, '__$1__');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)__ +([^_]+?)__/g, '__$1__');
  normalized = normalized.replace(/__([^\s_][^_]*?) +__/g, '__$1__');

  // UNDERSCORE ITALIC patterns
  normalized = normalized.replace(/([a-zA-Z0-9])(?<!_)_ (\w)/g, '$1 _$2');
  normalized = normalized.replace(/(?<!_)_([^\s_][^_]*?) +_(?!_)/g, '_$1_');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)(?<!_)_ +([^_]+?)_(?!_)/g, '_$1_');
  normalized = normalized.replace(/(?<!_)_([^\s_][^_]*?) +_(?!_)/g, '_$1_');

  // STRIKETHROUGH patterns
  normalized = normalized.replace(/([a-zA-Z0-9])~~ (\w)/g, '$1 ~~$2');
  normalized = normalized.replace(/~~([^\s~][^~]*?) +~~/g, '~~$1~~');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)~~ +([^~]+?)~~/g, '~~$1~~');
  normalized = normalized.replace(/~~([^\s~][^~]*?) +~~/g, '~~$1~~');

  // INLINE CODE patterns
  normalized = normalized.replace(/`([^\s`][^`]*?) +`/g, '`$1`');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)` +([^`]+?)`/g, '`$1`');
  normalized = normalized.replace(/`([^\s`][^`]*?) +`/g, '`$1`');

  return normalized;
}

// Test cases
const testCases = [
  // ===== SHOULD FIX =====
  {
    name: 'word** text pattern',
    input: 'The** Code2Doc Dataset**',
    expected: 'The **Code2Doc Dataset**',
    shouldFix: true
  },
  {
    name: 'trailing space before close',
    input: '**The curse of recursion **',
    expected: '**The curse of recursion**',
    shouldFix: true
  },
  {
    name: 'both issues combined',
    input: 'This** bold **word',
    expected: 'This **bold** word',
    shouldFix: true
  },
  {
    name: 'leading space after open',
    input: '** text**',
    expected: '**text**',
    shouldFix: true
  },
  {
    name: 'both leading and trailing space',
    input: '** text **',
    expected: '**text**',
    shouldFix: true
  },
  
  // ===== SHOULD NOT CHANGE (correct markdown) =====
  {
    name: 'correct bold',
    input: '**Products**:',
    expected: '**Products**:',
    shouldFix: false
  },
  {
    name: 'correct bold with space before',
    input: '1. **Products**:',
    expected: '1. **Products**:',
    shouldFix: false
  },
  {
    name: 'multiple bold in sentence',
    input: 'The **quick** brown **fox** jumps',
    expected: 'The **quick** brown **fox** jumps',
    shouldFix: false
  },
  {
    name: 'bold at line start',
    input: '**Products**:\n  - Code2Doc',
    expected: '**Products**:\n  - Code2Doc',
    shouldFix: false
  },
  {
    name: 'nested bold italic (should not break)',
    input: '***bold italic***',
    expected: '***bold italic***',
    shouldFix: false
  },
  
  // ===== EDGE CASES =====
  {
    name: 'code in bold',
    input: '**`code`**',
    expected: '**`code`**',
    shouldFix: false
  },
  {
    name: 'asterisks in code (should not touch)',
    input: '`word** text**`',
    expected: '`word** text**`',
    shouldFix: false
  },
  {
    name: 'URL with ** (should not break)',
    input: 'https://example.com/**path**/',
    expected: 'https://example.com/**path**/',
    shouldFix: false
  },
  {
    name: 'math expression (should not touch)',
    input: 'The equation is $a**2 + b**2$',
    expected: 'The equation is $a**2 + b**2$',
    shouldFix: false  // Math expressions might be mishandled
  },
  
  // ===== REAL LLM OUTPUT PATTERNS =====
  {
    name: 'LLM list with bold headers',
    input: '1. **Products**:\n   - Code2Doc\n   - Dataset',
    expected: '1. **Products**:\n   - Code2Doc\n   - Dataset',
    shouldFix: false
  },
  {
    name: 'LLM with partial bold formation',
    input: 'The main **entities** include:',
    expected: 'The main **entities** include:',
    shouldFix: false
  }
];

console.log('='.repeat(80));
console.log('NORMALIZATION FUNCTION TEST');
console.log('='.repeat(80));
console.log();

let passed = 0;
let failed = 0;
const failures = [];

for (const test of testCases) {
  const result = normalizeMarkdownForStreaming(test.input);
  const success = result === test.expected;
  
  if (success) {
    passed++;
    console.log(`✅ PASS: ${test.name}`);
  } else {
    failed++;
    console.log(`❌ FAIL: ${test.name}`);
    console.log(`   Input:    "${test.input}"`);
    console.log(`   Expected: "${test.expected}"`);
    console.log(`   Got:      "${result}"`);
    failures.push({
      name: test.name,
      input: test.input,
      expected: test.expected,
      actual: result,
      shouldFix: test.shouldFix
    });
  }
}

console.log();
console.log('='.repeat(80));
console.log(`RESULTS: ${passed} passed, ${failed} failed`);
console.log('='.repeat(80));

if (failures.length > 0) {
  console.log();
  console.log('FAILURE DETAILS:');
  console.log();
  
  for (const f of failures) {
    console.log(`${f.name}:`);
    console.log(`  Should ${f.shouldFix ? 'FIX' : 'NOT CHANGE'} but didn't match expected`);
    
    // Highlight the difference
    const inputVis = f.input.replace(/ /g, '·').replace(/\n/g, '↵');
    const expectedVis = f.expected.replace(/ /g, '·').replace(/\n/g, '↵');
    const actualVis = f.actual.replace(/ /g, '·').replace(/\n/g, '↵');
    
    console.log(`  Input:    "${inputVis}"`);
    console.log(`  Expected: "${expectedVis}"`);
    console.log(`  Actual:   "${actualVis}"`);
    console.log();
  }
}

// Test progressive streaming simulation
console.log();
console.log('='.repeat(80));
console.log('STREAMING SIMULATION TEST');
console.log('='.repeat(80));
console.log();

// Simulate tokens arriving progressively
const streamTokens = [
  "The",
  " main",
  " entities",
  " in",
  " your",
  " knowledge",
  " graph",
  " include",
  ":\n\n",
  "1",
  ".",
  " **",
  "Products",
  "**",
  ":\n"
];

console.log('Simulating token stream and normalization at each step:');
console.log();

let accumulated = '';
for (let i = 0; i < streamTokens.length; i++) {
  accumulated += streamTokens[i];
  const normalized = normalizeMarkdownForStreaming(accumulated);
  
  const tokenVis = streamTokens[i].replace(/ /g, '·').replace(/\n/g, '↵');
  const accVis = accumulated.replace(/ /g, '·').replace(/\n/g, '↵');
  const normVis = normalized.replace(/ /g, '·').replace(/\n/g, '↵');
  
  const changed = accumulated !== normalized;
  const marker = changed ? '🔄' : '  ';
  
  console.log(`[${i.toString().padStart(2, '0')}] Token: "${tokenVis}"`);
  console.log(`     ${marker} Accumulated: "${accVis}"`);
  if (changed) {
    console.log(`     ${marker} Normalized:  "${normVis}"`);
  }
  console.log();
}
