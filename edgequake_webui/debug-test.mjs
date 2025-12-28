// Full function trace - with re-run for both-sides cases
function normalizeMarkdownForStreaming(content) {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let normalized = content;

  console.log('Step 0 (Input):', JSON.stringify(normalized));

  // ═══════════════════════════════════════════════════════════════════
  // BOLD (**text**)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 1: **text ** (trailing space before closing) → **text**
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  console.log('Step 1 (bold trailing space):', JSON.stringify(normalized));
  
  // Pattern 2: ** text** (leading space after opening) → **text**
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)\*\* +([^*]+?)\*\*/g, '**$1**');
  console.log('Step 2 (bold leading space):', JSON.stringify(normalized));
  
  // Pattern 3: Re-run trailing pattern for "** text **" case
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  console.log('Step 3 (bold trailing - second pass):', JSON.stringify(normalized));

  // ═══════════════════════════════════════════════════════════════════
  // ITALIC (*text*)
  // ═══════════════════════════════════════════════════════════════════
  
  // Pattern 1: *text * (trailing space before closing) → *text*
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');
  console.log('Step 4 (italic trailing space):', JSON.stringify(normalized));
  
  // Pattern 2: * text* (leading space after opening) → *text*
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)(?<!\*)\* +([^*]+?)\*(?!\*)/g, '*$1*');
  console.log('Step 5 (italic leading space):', JSON.stringify(normalized));
  
  // Pattern 3: Re-run trailing for "* text *" case
  normalized = normalized.replace(/(?<!\*)\*([^\s*][^*]*?) +\*(?!\*)/g, '*$1*');
  console.log('Step 6 (italic trailing - second pass):', JSON.stringify(normalized));

  return normalized;
}

// Test cases
const tests = [
  { input: 'text **Bold ** and **more **', expected: 'text **Bold** and **more**', name: 'Bold: multiple in sentence' },
  { input: '**Products **:', expected: '**Products**:', name: 'Bold: trailing space' },
  { input: '** Products**:', expected: '**Products**:', name: 'Bold: leading space' },
  { input: '** Products **:', expected: '**Products**:', name: 'Bold: both sides' },
  { input: '*italic *', expected: '*italic*', name: 'Italic: trailing space' },
];

console.log('=== Full normalization trace ===\n');
tests.forEach(({ input, expected, name }) => {
  console.log(`\n--- ${name} ---`);
  const result = normalizeMarkdownForStreaming(input);
  const passed = result === expected;
  console.log(`\n${passed ? '✅' : '❌'} ${name}`);
  console.log(`Expected: ${JSON.stringify(expected)}`);
  console.log(`Got:      ${JSON.stringify(result)}`);
});
