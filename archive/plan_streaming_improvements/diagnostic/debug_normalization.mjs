#!/usr/bin/env node
/**
 * Detailed Normalization Debug
 * 
 * Tests each regex pattern step by step to identify the bug
 */

// Test input that's being corrupted
const input = "The main **entities** include:";

console.log('='.repeat(80));
console.log('DETAILED NORMALIZATION DEBUG');
console.log('='.repeat(80));
console.log();
console.log('Input:', input);
console.log('Input (visible):', input.replace(/ /g, '·'));
console.log();

let content = input;

// Step 1: normalizeMarkdownForStreaming patterns
console.log('--- NORMALIZE MARKDOWN FOR STREAMING ---');
console.log();

// Pattern 0: word** text → word **text
const p0 = /([a-zA-Z0-9])\*\* (\w)/g;
const after_p0 = content.replace(p0, '$1 **$2');
console.log('Pattern 0 (word** text):', content !== after_p0 ? '🔄 CHANGED' : 'no change');
if (content !== after_p0) {
  console.log('  Before:', content.replace(/ /g, '·'));
  console.log('  After: ', after_p0.replace(/ /g, '·'));
}
content = after_p0;

// Pattern 1: **text ** → **text**
const p1 = /\*\*([^\s*][^*]*?) +\*\*/g;
const after_p1 = content.replace(p1, '**$1**');
console.log('Pattern 1 (**text **):', content !== after_p1 ? '🔄 CHANGED' : 'no change');
if (content !== after_p1) {
  console.log('  Before:', content.replace(/ /g, '·'));
  console.log('  After: ', after_p1.replace(/ /g, '·'));
}
content = after_p1;

// Pattern 2: ** text** → **text**
const p2 = /(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)\*\* +([^*]+?)\*\*/g;
const after_p2 = content.replace(p2, '**$1**');
console.log('Pattern 2 (** text**):', content !== after_p2 ? '🔄 CHANGED' : 'no change');
if (content !== after_p2) {
  console.log('  Before:', content.replace(/ /g, '·'));
  console.log('  After: ', after_p2.replace(/ /g, '·'));
}
content = after_p2;

// Pattern 3: Re-run trailing
const p3 = /\*\*([^\s*][^*]*?) +\*\*/g;
const after_p3 = content.replace(p3, '**$1**');
console.log('Pattern 3 (trailing re-run):', content !== after_p3 ? '🔄 CHANGED' : 'no change');
if (content !== after_p3) {
  console.log('  Before:', content.replace(/ /g, '·'));
  console.log('  After: ', after_p3.replace(/ /g, '·'));
}
content = after_p3;

console.log();
console.log('After normalizeMarkdownForStreaming:', content.replace(/ /g, '·'));
console.log();

// Step 2: addSpacesAroundMarkdown patterns
console.log('--- ADD SPACES AROUND MARKDOWN ---');
console.log();

// Pattern A: **boldtext**nextword → **boldtext** nextword
const pA = /(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g;
const after_pA = content.replace(pA, '$1 $3');
console.log('Pattern A (after **):');
console.log('  Regex:', pA.toString());
console.log('  Changed:', content !== after_pA ? '🔄 CHANGED' : 'no change');
if (content !== after_pA) {
  console.log('  Before:', content.replace(/ /g, '·'));
  console.log('  After: ', after_pA.replace(/ /g, '·'));
  
  // Debug: show matches
  const matches = content.matchAll(/(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g);
  for (const m of matches) {
    console.log('  Match:', m[0], 'at index', m.index);
  }
}
content = after_pA;

// Pattern B: word**boldtext** → word **boldtext**
const pB = /([a-zA-Z0-9])(\*\*([^\s*][^*]*?)\*\*)/g;
const after_pB = content.replace(pB, '$1 $2');
console.log('Pattern B (before **):');
console.log('  Regex:', pB.toString());
console.log('  Changed:', content !== after_pB ? '🔄 CHANGED' : 'no change');
if (content !== after_pB) {
  console.log('  Before:', content.replace(/ /g, '·'));
  console.log('  After: ', after_pB.replace(/ /g, '·'));
  
  // Debug: show matches
  const matches2 = content.matchAll(/([a-zA-Z0-9])(\*\*([^\s*][^*]*?)\*\*)/g);
  for (const m of matches2) {
    console.log('  Match:', JSON.stringify(m[0]), 'at index', m.index);
    console.log('    Group 1 (char before):', JSON.stringify(m[1]));
    console.log('    Group 2 (bold block):', JSON.stringify(m[2]));
  }
}
content = after_pB;

console.log();
console.log('FINAL OUTPUT:', content);
console.log('FINAL (visible):', content.replace(/ /g, '·'));
console.log();

// Test a known-problematic case
console.log('='.repeat(80));
console.log('TESTING: "The **quick** brown **fox** jumps"');
console.log('='.repeat(80));

content = "The **quick** brown **fox** jumps";
console.log('Input:', content.replace(/ /g, '·'));

// Apply addSpacesAroundMarkdown patterns
const pA2 = /(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g;

console.log();
console.log('Pattern A matches:');
const matches3 = content.matchAll(/(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g);
for (const m of matches3) {
  console.log('  Match:', JSON.stringify(m[0]), 'at index', m.index);
  console.log('    Group 1:', JSON.stringify(m[1]));
  console.log('    Group 3:', JSON.stringify(m[3]));
}

content = content.replace(pA2, '$1 $3');
console.log();
console.log('After Pattern A:', content.replace(/ /g, '·'));

// The issue is clear: the pattern is matching "**quick** b" and "**fox** j"
// and replacing them with "**quick** b" and "**fox** j" - BUT WAIT
// Let me re-check...

console.log();
console.log('='.repeat(80));
console.log('THE ISSUE: Pattern A is NOT adding space, it seems correct...');
console.log('Let me test the FULL function...');
console.log('='.repeat(80));

function addSpacesAroundMarkdown(c) {
  let processed = c;
  
  // Pattern A
  processed = processed.replace(/(\*\*([^\s*][^*]*?)\*\*)([a-zA-Z0-9])/g, '$1 $3');
  // Pattern B
  processed = processed.replace(/([a-zA-Z0-9])(\*\*([^\s*][^*]*?)\*\*)/g, '$1 $2');
  
  return processed;
}

function normalizeMarkdownForStreaming(c) {
  let normalized = c;
  
  normalized = normalized.replace(/([a-zA-Z0-9])\*\* (\w)/g, '$1 **$2');
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  normalized = normalized.replace(/(?<=^|[\r\n\.,;:!?'"()\[\]{}]|^[ \t]+)\*\* +([^*]+?)\*\*/g, '**$1**');
  normalized = normalized.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
  
  return normalized;
}

const testInput = "The **quick** brown **fox** jumps";
console.log();
console.log('Input:', testInput.replace(/ /g, '·'));

const afterNorm = normalizeMarkdownForStreaming(testInput);
console.log('After normalize:', afterNorm.replace(/ /g, '·'));

const afterSpace = addSpacesAroundMarkdown(afterNorm);
console.log('After addSpaces:', afterSpace.replace(/ /g, '·'));

// Hmm, let me check the SPECIFIC pattern that's eating spaces
console.log();
console.log('='.repeat(80));
console.log('AH-HA! Found it - the regex is consuming the space in the match!');
console.log('='.repeat(80));
console.log();

// Let's trace exactly what Pattern B is doing
const text = "The **quick** brown";
console.log('Text:', text.replace(/ /g, '·'));

const patternB = /([a-zA-Z0-9])(\*\*([^\s*][^*]*?)\*\*)/g;
console.log('Pattern B regex:', patternB.toString());

const matches4 = [...text.matchAll(patternB)];
console.log('Matches found:', matches4.length);
for (const m of matches4) {
  console.log('  Full match:', JSON.stringify(m[0]));
  console.log('  Index:', m.index);
  console.log('  Group 1 (alphanum):', JSON.stringify(m[1]));
  console.log('  Group 2 (bold):', JSON.stringify(m[2]));
}

// The ISSUE: "n **quick**" matches because:
// - "n" is alphanumeric (end of "brown")
// - "**quick**" is the bold block
// But that's WRONG because there's already a space!
// Wait... let me check again

const text2 = "n **quick**";
const m = text2.match(patternB);
console.log();
console.log('Testing:', JSON.stringify(text2));
console.log('Match:', m);
// This should NOT match because **quick** has a space before it

// Let me test without the space
const text3 = "n**quick**";
const m2 = text3.match(patternB);
console.log('Testing:', JSON.stringify(text3));
console.log('Match:', m2);
// This SHOULD match

// So the pattern is CORRECT. The issue must be in normalizeMarkdownForStreaming!
console.log();
console.log('='.repeat(80));
console.log('FOUND IT! The issue is in normalizeMarkdownForStreaming');
console.log('='.repeat(80));

// Let's trace each pattern
const t = "The **quick** brown **fox** jumps";
console.log('Input:', t.replace(/ /g, '·'));

// Pattern 1: **text ** → **text**
// This matches "**quick** " and replaces with "**quick**" - EATING THE SPACE!
const pattern1_result = t.replace(/\*\*([^\s*][^*]*?) +\*\*/g, '**$1**');
console.log();
console.log('After Pattern 1 (**text ** → **text**):');
console.log('  Result:', pattern1_result.replace(/ /g, '·'));

// Yes! The pattern /\*\*([^\s*][^*]*?) +\*\*/ matches:
// - "**quick** " (with trailing space!) because " +" matches one or more spaces
// Wait, that's not quite right either...

// Let me test more carefully
const testStr = "**quick** brown";
const pattern1 = /\*\*([^\s*][^*]*?) +\*\*/g;
const matches5 = [...testStr.matchAll(pattern1)];
console.log();
console.log('Testing Pattern 1 on:', JSON.stringify(testStr));
console.log('Matches:', matches5.length);
for (const m of matches5) {
  console.log('  Match:', JSON.stringify(m[0]));
  console.log('  Group 1:', JSON.stringify(m[1]));
}

// Hmm, this should NOT match because there's no " **"
// The pattern is looking for space BEFORE the closing **

const testStr2 = "**quick ** brown";
const matches6 = [...testStr2.matchAll(pattern1)];
console.log();
console.log('Testing Pattern 1 on:', JSON.stringify(testStr2));
console.log('Matches:', matches6.length);
for (const m of matches6) {
  console.log('  Match:', JSON.stringify(m[0]));
}

// OK so Pattern 1 should be fine. Let me trace the FULL normalize function step by step
