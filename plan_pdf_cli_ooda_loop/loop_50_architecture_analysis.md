# OODA Loop 50: Code Analysis - Block-Level vs Line-Level Processing
## SpaceTimePilot Paper (01_2512.25075v1.pdf)

**Date**: 2026-01-04  
**Focus**: Examine HyphenContinuationProcessor implementation  
**Discovery**: Processor only handles block-to-block, NOT line-to-line within blocks

---

## 🔍 OBSERVE: Current Implementation

### HyphenContinuationProcessor Code
**Location**: `edgequake/crates/edgequake-pdf/src/processors/text_cleanup.rs` (lines 503-624)

**Algorithm**:
```rust
impl Processor for HyphenContinuationProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            let mut i = 0;
            while i < page.blocks.len().saturating_sub(1) {
                // Check if CURRENT BLOCK ends with hyphen
                if Self::ends_with_explicit_hyphen(&current_text)
                    && Self::is_valid_continuation(&next_text)
                    && Self::get_hyphen_fragment(&current_text).is_some()
                {
                    // Join THIS BLOCK with NEXT BLOCK
                    // Remove hyphen, combine words, merge blocks
                }
                i += 1;
            }
        }
        Ok(document)
    }
}
```

### What This Handles ✅
**Inter-block hyphenation**:
```
Block 1: "This is a gener-"
Block 2: "ative rendering model"
Result: "This is a generative rendering model" (blocks merged)
```

### What This DOESN'T Handle ❌
**Intra-block hyphenation** (lines within a block):
```
Block 1:
  Line 1: "This is a gener-"
  Line 2: "ative rendering model"
Result: "This is a gener- ative rendering model" (hyphen + line break remain!)
```

---

## 🧭 ORIENT: The Real Problem

### Two-Level Hyphenation Issue

**Level 1: Block-to-Block** (HANDLED ✅)
- When PDF has each line as separate block
- Example: Simple PDFs, basic layouts
- HyphenContinuationProcessor works correctly

**Level 2: Line-to-Line Within Blocks** (NOT HANDLED ❌)
- When PDF merges multiple lines into one block/paragraph
- Example: Academic papers with flowing text
- Hyphenated line breaks PRESERVED in block text
- This is our SpaceTimePilot case!

### Why SpaceTimePilot Paper Fails

**Evidence**:
```
Current extraction (truncated):
"disentangles space and time for controllable generative ren- independently alter..."
                                                        ^^^--- hyphen + line break + lost word
```

**Root Cause**:
1. PDF extracts text as paragraphs (multi-line blocks)
2. Block text includes embedded line breaks: `"...ren-\ndering..."`
3. HyphenContinuationProcessor only looks at block boundaries
4. Line breaks WITHIN block text are never processed
5. Result: "ren-\ndering" stays broken, appears as "ren- " (hyphen + space)

### Architectural Flow

```
PDF → SotaBackend → Document(pages, blocks)
                      ↓
                   blocks[i].text = "line1\nline2\nline3"
                      ↓
          HyphenContinuationProcessor (BLOCK LEVEL only)
                      ↓
                  ❌ Misses intra-block line breaks
```

---

## 🎯 DECIDE: Fix Strategy

### Option 1: Add Line-Level Processing to HyphenContinuationProcessor ✅ PREFERRED

**Approach**: Process text WITHIN each block, not just between blocks.

**Pseudocode**:
```rust
fn process_block_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::new();
    
    for i in 0..lines.len() {
        let line = lines[i].trim_end();
        
        if line.ends_with('-') && i + 1 < lines.len() {
            let next_line = lines[i + 1].trim_start();
            
            if is_valid_continuation(next_line) {
                // Remove hyphen
                let without_hyphen = &line[..line.len() - 1];
                // Get first word from next line
                let first_word = next_line.split_whitespace().next().unwrap();
                // Join
                result.push_str(without_hyphen);
                result.push_str(first_word);
                
                // Add rest of next line
                let rest = next_line.strip_prefix(first_word).unwrap().trim_start();
                if !rest.is_empty() {
                    result.push(' ');
                    result.push_str(rest);
                }
                // SKIP next line (already processed)
                i += 1;
            } else {
                result.push_str(line);
            }
        } else {
            result.push_str(line);
        }
        
        if i + 1 < lines.len() {
            result.push('\n');
        }
    }
    
    result
}
```

**Pros**:
- Fixes root cause directly
- Handles BOTH block-to-block AND line-to-line
- Clean, maintainable solution

**Cons**:
- Need to be careful with line iteration (skip processed lines)
- Must preserve line breaks that are intentional

### Option 2: Pre-process in SotaBackend ⚠️ NOT RECOMMENDED

**Approach**: Fix hyphenation during PDF extraction, before creating blocks.

**Cons**:
- Violates separation of concerns
- SotaBackend should be raw extraction only
- Makes debugging harder
- Would need to duplicate logic

### Option 3: Add Separate LineProcessor ⚠️ OVER-ENGINEERED

**Approach**: Create new processor that splits blocks into lines, processes, rejoins.

**Cons**:
- Unnecessary complexity
- Hyphen continuation is ONE feature, not worth separate processor
- Better to extend existing HyphenContinuationProcessor

---

## ⚡ ACT: Implementation Plan for Loop 51

### Step 1: Write Failing Test Case
```rust
#[test]
fn test_hyphen_continuation_within_block() {
    let mut doc = Document::new();
    doc.add_page(Page::new(0));
    
    // Single block with embedded line break and hyphen
    doc.pages[0].add_block(Block {
        text: "This is a gener-\native rendering model".to_string(),
        block_type: BlockType::Paragraph,
        bbox: BBox::new(0.0, 0.0, 100.0, 20.0),
        page_num: 0,
    });
    
    let processor = HyphenContinuationProcessor::new();
    let result = processor.process(doc).unwrap();
    
    assert_eq!(
        result.pages[0].blocks[0].text,
        "This is a generative rendering model"
    );
}
```

### Step 2: Implement Line-Level Processing
1. Add `process_lines_in_block()` method
2. Update `process()` to call it for each block FIRST
3. Then apply existing block-to-block logic

### Step 3: Handle Edge Cases
- Em-dash (—) vs hyphen (-)
- Soft hyphen (U+00AD)
- Mid-sentence hyphens that are intentional (e.g., "state-of-the-art")
- Multiple consecutive hyphens

### Step 4: Run Tests
- New test: MUST pass
- Existing 133 tests: MUST all still pass
- SpaceTimePilot extraction: Check improvement

---

## 📊 RESULT: Loop 50 Insights

### Critical Understanding 🔥
**The bug is architectural**: HyphenContinuationProcessor was designed for line-per-block PDFs, but academic papers use paragraph-per-block structure.

### Why This Wasn't Caught Earlier
- Synthetic test PDFs likely use simple layouts (line-per-block)
- Real academic papers use complex layouts (paragraph-per-block)
- No existing test covered intra-block hyphenation

### Impact Prediction
After fix:
- Abstract retention: 23.4% → **85%+** (4x improvement)
- Introduction retention: 29.3% → **75%+** (2.5x improvement)
- Method retention: 66.9% → **85%+**
- Results retention: 55.5% → **80%+**
- **Overall: 71.1% → 85%+**

### Confidence Assessment
- **Root cause diagnosis**: 98% confidence ✅✅✅
- **Fix approach**: 95% confidence ✅✅
- **Won't break tests**: 90% confidence ✅ (need to test)

---

## 🎯 Commit Message
```
docs(pdf): OODA Loop 50 - Architecture analysis reveals block vs line gap

Examined HyphenContinuationProcessor implementation:
- Currently only handles block-to-block hyphenation ✅
- Does NOT handle line-to-line within blocks ❌

SpaceTimePilot paper has paragraph blocks with embedded line breaks:
"...ren-\ndering..." stays as "ren- " (hyphen + space + lost word)

Root cause: Algorithm iterates over blocks, not lines within blocks.
Academic papers use multi-line paragraph blocks, so intra-block
hyphenation is NEVER processed.

Fix: Add line-level processing within each block before block-level
processing. This will handle BOTH cases.

Next: Implement fix with tests (Loop 51).
Expected: 71% → 85%+ retention.
```
