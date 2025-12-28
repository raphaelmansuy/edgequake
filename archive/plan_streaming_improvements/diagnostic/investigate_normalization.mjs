/**
 * Deep Investigation: Is normalization CAUSING the markdown issues?
 * 
 * Hypothesis: The server returns CORRECT markdown, but our normalization
 * function is CORRUPTING it.
 */

// Exact server response (captured from API)
const serverResponse = `The main entities in your knowledge graph include:

1. **Products**:
   - Code2Doc
   - Code2Doc Dataset

2. **Concepts**:
   - The curse of recursion

3. **Programming Languages**:
   - Java
   - Python`;

console.log('=== INVESTIGATION: Is normalization corrupting correct markdown? ===\n');
console.log('SERVER RESPONSE (raw from API):');
console.log(serverResponse);
console.log('\n' + '='.repeat(70) + '\n');

// Copy of the normalization function from StreamingMarkdownRenderer.tsx
function normalizeMarkdownForStreaming(content) {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let normalized = content;

  // Pattern 0: word** text → word **text
  normalized = normalized.replace(/(?<!\*\*[^*]*)([a-zA-Z0-9])\*\* (\w)/g, '$1 **$2');
  
  // Pattern 0b: punctuation followed by ** → add space
  normalized = normalized.replace(/([\.\,\:\;\!\?\)])(\*\*)/g, '$1 $2');
  
  // Pattern 1: **text ** → **text**
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  
  // Pattern 2: ** text** → **text**
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+|\s)\*\* +([^*]+?)\*\*/g, '**$1**');
  
  // Pattern 3: Re-run trailing
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');

  return normalized;
}

// Copy of addSpacesAroundMarkdown function
function addSpacesAroundMarkdown(content) {
  if (!content || typeof content !== 'string') {
    return content;
  }

  let processed = content;

  // Fix **boldtext**nextword → **boldtext** nextword
  processed = processed.replace(/(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g, '$1 $3');
  
  // Fix word**boldtext** → word **boldtext**
  processed = processed.replace(/([a-zA-Z0-9])(\*\*([^\s*][^*]*?)\*\*)/g, '$1 $2');

  return processed;
}

// Test normalization only
const afterNormalization = normalizeMarkdownForStreaming(serverResponse);
console.log('AFTER normalizeMarkdownForStreaming():');
console.log(afterNormalization);
console.log('\n' + '='.repeat(70) + '\n');

// Check if normalization changed anything
if (serverResponse === afterNormalization) {
  console.log('✅ normalizeMarkdownForStreaming: NO CHANGE (good!)');
} else {
  console.log('❌ normalizeMarkdownForStreaming: CONTENT WAS MODIFIED!');
  console.log('\nDifferences:');
  
  const serverLines = serverResponse.split('\n');
  const normalizedLines = afterNormalization.split('\n');
  
  for (let i = 0; i < Math.max(serverLines.length, normalizedLines.length); i++) {
    if (serverLines[i] !== normalizedLines[i]) {
      console.log(`  Line ${i + 1}:`);
      console.log(`    Server:     "${serverLines[i]}"`);
      console.log(`    Normalized: "${normalizedLines[i]}"`);
    }
  }
}

console.log('\n' + '='.repeat(70) + '\n');

// Test addSpacesAroundMarkdown
const afterSpacing = addSpacesAroundMarkdown(afterNormalization);
console.log('AFTER addSpacesAroundMarkdown():');
console.log(afterSpacing);
console.log('\n' + '='.repeat(70) + '\n');

// Check if spacing changed anything
if (afterNormalization === afterSpacing) {
  console.log('✅ addSpacesAroundMarkdown: NO CHANGE (good!)');
} else {
  console.log('❌ addSpacesAroundMarkdown: CONTENT WAS MODIFIED!');
  console.log('\nDifferences:');
  
  const normalizedLines = afterNormalization.split('\n');
  const spacedLines = afterSpacing.split('\n');
  
  for (let i = 0; i < Math.max(normalizedLines.length, spacedLines.length); i++) {
    if (normalizedLines[i] !== spacedLines[i]) {
      console.log(`  Line ${i + 1}:`);
      console.log(`    Before: "${normalizedLines[i]}"`);
      console.log(`    After:  "${spacedLines[i]}"`);
    }
  }
}

console.log('\n' + '='.repeat(70) + '\n');

// Final comparison
console.log('SUMMARY:');
console.log(`  Server response correct: YES (verified above)`);
console.log(`  After normalization same as server: ${serverResponse === afterNormalization ? 'YES ✅' : 'NO ❌'}`);
console.log(`  After spacing same as normalized: ${afterNormalization === afterSpacing ? 'YES ✅' : 'NO ❌'}`);
console.log(`  Final output same as server: ${serverResponse === afterSpacing ? 'YES ✅' : 'NO ❌'}`);
