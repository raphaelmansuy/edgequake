#!/usr/bin/env node
/**
 * Test the FIXED normalization function
 */

// FIXED normalizeMarkdownForStreaming function
function normalizeMarkdownForStreaming(content) {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let normalized = content;

  // BOLD (**text**)
  
  // Pattern 0 (FIXED): word** text → word **text
  // IMPORTANT: Use negative lookbehind to ensure we're not inside a bold span
  // (?<!\*\*[^*]*) ensures there's no preceding **text before our match
  normalized = normalized.replace(/(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g, '$1 **$2');
  
  // Pattern 1: **text ** (trailing space before closing) → **text**
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  
  // Pattern 2: ** text** (leading space after opening) → **text**
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)\*\* +([^*]+?)\*\*/g, '**$1**');
  
  // Pattern 3: Re-run trailing pattern
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');

  // ITALIC (*text*) 
  normalized = normalized.replace(/(?<!\*[^*]*)([a-zA-Z0-9])(?<!\*)\* (\w)/g, '$1 *$2');
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)(?<!\*)\* +([^*]+?)\*(?!\*)/g, '*$1*');
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');

  // UNDERSCORE BOLD (__text__)
  normalized = normalized.replace(/(?<!__[^_]*)([a-zA-Z0-9])__ (\w)/g, '$1 __$2');
  normalized = normalized.replace(/__([^\s_][^_]*?) +__/g, '__$1__');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)__ +([^_]+?)__/g, '__$1__');
  normalized = normalized.replace(/__([^\s_][^_]*?) +__/g, '__$1__');

  // UNDERSCORE ITALIC (_text_)
  normalized = normalized.replace(/(?<!_[^_]*)([a-zA-Z0-9])(?<!_)_ (\w)/g, '$1 _$2');
  normalized = normalized.replace(/(?<!_)_([^\s_][^_]*?) +_(?!_)/g, '_$1_');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)(?<!_)_ +([^_]+?)_(?!_)/g, '_$1_');
  normalized = normalized.replace(/(?<!_)_([^\s_][^_]*?) +_(?!_)/g, '_$1_');

  // STRIKETHROUGH (~~text~~)
  normalized = normalized.replace(/(?<!~~[^~]*)([a-zA-Z0-9])~~ (\w)/g, '$1 ~~$2');
  normalized = normalized.replace(/~~([^\s~][^~]*?) +~~/g, '~~$1~~');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)~~ +([^~]+?)~~/g, '~~$1~~');
  normalized = normalized.replace(/~~([^\s~][^~]*?) +~~/g, '~~$1~~');

  // INLINE CODE (`text`)
  normalized = normalized.replace(/`([^\s`][^`]*?) +`/g, '`$1`');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)` +([^`]+?)`/g, '`$1`');
  normalized = normalized.replace(/`([^\s`][^`]*?) +`/g, '`$1`');

  return normalized;
}

// Test cases
const testCases = [
  // ===== SHOULD FIX =====
  {
    name: 'word** text pattern (the original issue)',
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
    name: 'multiple bold in sentence (THE BUG WE FIXED)',
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
  {
    name: 'LLM with partial bold formation (THE BUG WE FIXED)',
    input: 'The main **entities** include:',
    expected: 'The main **entities** include:',
    shouldFix: false
  },
  {
    name: 'code in bold',
    input: '**`code`**',
    expected: '**`code`**',
    shouldFix: false
  },
  {
    name: 'LLM list with bold headers',
    input: '1. **Products**:\n   - Code2Doc\n   - Dataset',
    expected: '1. **Products**:\n   - Code2Doc\n   - Dataset',
    shouldFix: false
  }
];

console.log('='.repeat(80));
console.log('FIXED NORMALIZATION FUNCTION TEST');
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
    console.log(`   Input:    "${test.input.replace(/ /g, '·').replace(/\n/g, '↵')}"`);
    console.log(`   Expected: "${test.expected.replace(/ /g, '·').replace(/\n/g, '↵')}"`);
    console.log(`   Got:      "${result.replace(/ /g, '·').replace(/\n/g, '↵')}"`);
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
  for (const f of failures) {
    console.log();
    console.log(`${f.name}:`);
    console.log(`  Should ${f.shouldFix ? 'FIX' : 'NOT CHANGE'}`);
  }
}

process.exit(failed > 0 ? 1 : 0);
