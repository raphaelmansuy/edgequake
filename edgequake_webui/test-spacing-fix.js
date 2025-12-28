// Test the spacing fix regex patterns

const testCases = [
  // Should add spaces
  { input: "word**bold**word", expected: "word **bold** word" },
  { input: "**bold**word", expected: "**bold** word" },
  { input: "word**bold**", expected: "word **bold**" },
  
  // Should NOT add spaces (already has internal spaces)
  { input: "** bold** word", expected: "** bold** word" },
  { input: "word **bold **", expected: "word **bold **" },
  { input: "** bold **", expected: "** bold **" },
  
  // Mixed scenarios
  { input: "**Textbooks, Reports, and Slides** are", expected: "**Textbooks, Reports, and Slides** are" },
  { input: "**MegaRAG**is developed", expected: "**MegaRAG** is developed" },
];

function addSpacesAroundMarkdown(content) {
  let processed = content;
  
  // Fix **boldtext**nextword → **boldtext** nextword
  processed = processed.replace(/(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g, '$1 $3');
  
  // Fix word**boldtext** → word **boldtext**
  processed = processed.replace(/([a-zA-Z0-9])(\*\*([^\s*][^*]*?)\*\*)/g, '$1 $2');
  
  return processed;
}

console.log("Testing spacing fix:\n");
testCases.forEach(({ input, expected }, i) => {
  const result = addSpacesAroundMarkdown(input);
  const status = result === expected ? "✅" : "❌";
  console.log(`Test ${i + 1} ${status}`);
  console.log(`  Input:    "${input}"`);
  console.log(`  Expected: "${expected}"`);
  console.log(`  Got:      "${result}"`);
  if (result !== expected) {
    console.log(`  ERROR: Mismatch!`);
  }
  console.log();
});
