/**
 * Test FIXED non-streaming markdown normalization
 */

// Fixed normalization function
function normalizeMarkdownForStreaming(content) {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let normalized = content;

  // ═══════════════════════════════════════════════════════════════════
  // BOLD (**text**)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 0 (FIXED): word** text → word **text
  normalized = normalized.replace(/(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g, '$1 **$2');
  
  // Pattern 0b (NEW): punctuation followed by ** with no space → add space
  // Fixes: "2.**" → "2. **" and "1.**" → "1. **" (numbered lists)
  // Also handles: "word.**" → "word. **", "end:**" → "end: **"
  normalized = normalized.replace(/([\.\,\:\;\!\?\)])(\*\*)/g, '$1 $2');
  
  // Pattern 1: **text ** → **text**
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  
  // Pattern 2: ** text** → **text** (leading space after opening)
  // This now works better after 0b adds space before **
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+|\s)\*\* +([^*]+?)\*\*/g, '**$1**');
  
  // Pattern 3: Re-run trailing for "** text **" case
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');

  return normalized;
}

// Test cases from screenshot
const testCases = [
  { 
    input: '1. **Code2Doc and Code2Doc Dataset **:', 
    expected: '1. **Code2Doc and Code2Doc Dataset**:',
    description: 'Trailing space before closing **'
  },
  { 
    input: '2.** Programming Languages and Their Representations **:', 
    expected: '2. **Programming Languages and Their Representations**:',
    description: 'No space after dot, leading space after **, trailing space'
  },
  { 
    input: '3.** Technology and Tools **:', 
    expected: '3. **Technology and Tools**:',
    description: 'Same pattern as #2'
  },
  {
    input: 'The main **entities** include:',
    expected: 'The main **entities** include:',
    description: 'Already correct - should not change'
  },
  {
    input: 'This is **bold text** and more.',
    expected: 'This is **bold text** and more.',
    description: 'Already correct bold'
  },
  {
    input: 'word:**bold** works',
    expected: 'word: **bold** works',
    description: 'Colon before bold needs space'
  },
];

console.log('=== Testing FIXED normalization ===\n');
let passed = 0;
let failed = 0;

for (const tc of testCases) {
  const result = normalizeMarkdownForStreaming(tc.input);
  const pass = result === tc.expected;
  if (pass) passed++;
  else failed++;
  console.log(`${pass ? '✅ PASS' : '❌ FAIL'}: ${tc.description}`);
  console.log(`  Input:    "${tc.input}"`);
  console.log(`  Output:   "${result}"`);
  if (!pass) {
    console.log(`  Expected: "${tc.expected}"`);
  }
  console.log();
}

console.log(`Result: ${passed}/${testCases.length} passed`);
