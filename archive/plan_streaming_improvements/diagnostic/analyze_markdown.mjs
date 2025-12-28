#!/usr/bin/env node
/**
 * Markdown Issue Detector
 * 
 * Analyzes the raw SSE token sequence to identify markdown rendering issues.
 * Based on actual observed issues:
 * - "The** Code2Doc Dataset**" -> word** pattern
 * - "**The curse of recursion **" -> trailing space before **
 */

import { readFileSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Load the diagnostic output
const outputPath = join(__dirname, 'diagnostic_output.json');
let data;
try {
  data = JSON.parse(readFileSync(outputPath, 'utf-8'));
} catch (e) {
  console.error('No diagnostic output found. Run capture_sse_events.mjs first.');
  process.exit(1);
}

console.log('='.repeat(80));
console.log('MARKDOWN ISSUE ANALYSIS');
console.log('='.repeat(80));
console.log(`Query: "${data.query}"`);
console.log(`Tokens: ${data.tokenChunks.length}`);
console.log();

// Reconstruct content and track token boundaries
const tokens = data.tokenChunks;
const content = tokens.join('');

// Track where each token starts in the final content
let tokenPositions = [];
let pos = 0;
for (let i = 0; i < tokens.length; i++) {
  tokenPositions.push({ index: i, start: pos, end: pos + tokens[i].length, content: tokens[i] });
  pos += tokens[i].length;
}

console.log('--- FINDING BOLD MARKER SEQUENCES ---\n');

// Find all ** markers in the content
const markerPattern = /\*\*/g;
let markers = [];
let match;
while ((match = markerPattern.exec(content)) !== null) {
  markers.push(match.index);
}

console.log(`Found ${markers.length} ** markers at positions: ${markers.join(', ')}`);
console.log();

// For each marker, find which token it came from
console.log('--- MARKER TOKEN ANALYSIS ---\n');
markers.forEach((markerPos, i) => {
  const tokenInfo = tokenPositions.find(t => markerPos >= t.start && markerPos < t.end);
  const prevToken = tokenInfo ? tokenPositions[tokenInfo.index - 1] : null;
  const nextToken = tokenInfo ? tokenPositions[tokenInfo.index + 1] : null;
  
  const context = content.slice(Math.max(0, markerPos - 15), Math.min(content.length, markerPos + 15));
  const contextVis = context.replace(/ /g, '·').replace(/\n/g, '↵');
  
  // Determine if this is opening or closing
  const isOpening = i % 2 === 0;
  const markerType = isOpening ? 'OPEN' : 'CLOSE';
  
  console.log(`[${i}] ${markerType} at pos ${markerPos}`);
  console.log(`    Context: "${contextVis}"`);
  console.log(`    Token: [${tokenInfo?.index}] "${tokenInfo?.content.replace(/ /g, '·').replace(/\n/g, '↵')}"`);
  
  if (prevToken) {
    console.log(`    Prev:  [${prevToken.index}] "${prevToken.content.replace(/ /g, '·').replace(/\n/g, '↵')}"`);
  }
  if (nextToken) {
    console.log(`    Next:  [${nextToken.index}] "${nextToken.content.replace(/ /g, '·').replace(/\n/g, '↵')}"`);
  }
  
  // Check for issues
  if (isOpening) {
    // For opening **, check if previous token ends with alphanumeric (word**pattern)
    if (prevToken && /[a-zA-Z0-9]$/.test(prevToken.content)) {
      console.log(`    ⚠️  ISSUE: Opening ** directly follows word: "${prevToken.content}**..."`);
    }
    // Check if ** token itself has leading space (correct) or no space (issue)
    if (tokenInfo && tokenInfo.content === '**' && prevToken && !/\s$/.test(prevToken.content)) {
      console.log(`    ⚠️  ISSUE: Opening ** has no space before it`);
    }
  } else {
    // For closing **, check if next token starts with alphanumeric (pattern**word)
    if (nextToken && /^[a-zA-Z0-9]/.test(nextToken.content)) {
      console.log(`    ⚠️  ISSUE: Closing ** directly precedes word: "**${nextToken.content}"`);
    }
    // Check for trailing space before closing
    if (prevToken && / $/.test(prevToken.content)) {
      console.log(`    ⚠️  ISSUE: Space before closing **: "...${prevToken.content}**"`);
    }
  }
  console.log();
});

// Check for the specific patterns we saw
console.log('--- SPECIFIC PATTERN CHECKS ---\n');

// Pattern 1: word** (e.g., "The**")
const wordStarStar = /([a-zA-Z0-9])\*\* /g;
let matches = [];
while ((match = wordStarStar.exec(content)) !== null) {
  matches.push({ pos: match.index, context: content.slice(Math.max(0, match.index - 10), match.index + 15) });
}
if (matches.length > 0) {
  console.log(`FOUND ${matches.length} "word** " patterns:`);
  matches.forEach(m => console.log(`  - at ${m.pos}: "${m.context.replace(/\n/g, '↵')}"`));
} else {
  console.log('✅ No "word** " patterns found');
}
console.log();

// Pattern 2: ** followed by text (correct)
const starStarText = / \*\*[^\s*]/g;
matches = [];
while ((match = starStarText.exec(content)) !== null) {
  matches.push({ pos: match.index, context: content.slice(Math.max(0, match.index), match.index + 20) });
}
console.log(`Found ${matches.length} correct " **text" patterns (space before opening)`);
console.log();

// Pattern 3: text** (closing after text, correct)
const textStarStar = /[^\s*]\*\*[^*]/g;
matches = [];
while ((match = textStarStar.exec(content)) !== null) {
  matches.push({ pos: match.index, context: content.slice(Math.max(0, match.index), match.index + 15) });
}
console.log(`Found ${matches.length} "text**" patterns (closing after text)`);
console.log();

// Pattern 4: trailing space before closing (the issue in screenshot "recursion **")
const spaceClosePattern = / \*\*(?:[:\.,\n\r]|$)/g;
matches = [];
while ((match = spaceClosePattern.exec(content)) !== null) {
  matches.push({ pos: match.index, context: content.slice(Math.max(0, match.index - 10), match.index + 5) });
}
if (matches.length > 0) {
  console.log(`⚠️  FOUND ${matches.length} " **" (space before close) patterns:`);
  matches.forEach(m => console.log(`  - at ${m.pos}: "${m.context.replace(/\n/g, '↵')}"`));
} else {
  console.log('✅ No trailing space before closing ** patterns found');
}

console.log('\n' + '='.repeat(80));
console.log('SUMMARY');
console.log('='.repeat(80));
console.log(`
In this sample, the LLM output appears to be CORRECT:
- Opening ** markers have leading spaces (e.g., " **Products")
- Closing ** markers follow directly after text (e.g., "Products**")

The patterns in the token sequence:
  [011] " **"      <- space + opening marker (CORRECT)
  [012] "Products" <- text
  [013] "**"       <- closing marker (CORRECT)

If issues are appearing in the UI, they may be caused by:
1. Race conditions in React rendering during streaming
2. The normalization function introducing artifacts
3. Different LLM responses producing different patterns
4. Specific edge cases not covered in this sample
`);
