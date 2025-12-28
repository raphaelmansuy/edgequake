/**
 * Test non-streaming markdown normalization issues
 */

// Full normalization function - current implementation
function normalizeMarkdownForStreaming(content) {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let normalized = content;

  // Pattern 0 (FIXED): word** text → word **text
  normalized = normalized.replace(/(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g, '$1 **$2');
  
  // Pattern 1: **text ** → **text**
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  
  // Pattern 2: ** text** → **text**
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)\*\* +([^*]+?)\*\*/g, '**$1**');
  
  // Pattern 3: Re-run trailing
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');

  return normalized;
}

// Test cases from screenshot
const testCases = [
  { 
    input: '1. **Code2Doc and Code2Doc Dataset **:', 
    expected: '1. **Code2Doc and Code2Doc Dataset**:' 
  },
  { 
    input: '2.** Programming Languages and Their Representations **:', 
    expected: '2. **Programming Languages and Their Representations**:' 
  },
  { 
    input: '3.** Technology and Tools **:', 
    expected: '3. **Technology and Tools**:' 
  },
];

console.log('=== Testing current normalization ===\n');
let passed = 0;
let failed = 0;

for (const tc of testCases) {
  const result = normalizeMarkdownForStreaming(tc.input);
  const pass = result === tc.expected;
  if (pass) passed++;
  else failed++;
  console.log(`${pass ? '✅ PASS' : '❌ FAIL'}: "${tc.input}"`);
  console.log(`  -> "${result}"`);
  if (!pass) {
    console.log(`  Expected: "${tc.expected}"`);
  }
  console.log();
}

console.log(`Result: ${passed}/${testCases.length} passed\n`);

if (failed > 0) {
  console.log('=== Analysis ===');
  console.log('Issue: Pattern 2 handles "** text**" but doesn\'t add space before "**"');
  console.log('After patterns: "2.** Programming **" → "2.**Programming**"');
  console.log('Missing: Need to add space after "." before "**" → "2. **Programming**"');
  console.log();
  console.log('Proposed fix: Add pattern to handle numbered list markers');
  console.log('Pattern: /(\\d+\\.)\\*\\*/ → "$1 **"');
}
