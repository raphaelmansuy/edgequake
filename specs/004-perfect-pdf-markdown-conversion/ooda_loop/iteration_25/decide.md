# OODA-25 Decide: Caption Continuation & Format Fix

## Decision Summary

Implement two-part fix for Figure/Table captions:

1. **Caption Continuation Detection** in CaptionDetectionProcessor
2. **Caption Format Update** in markdown.rs render_caption()

## Implementation Plan

### Change 1: CaptionDetectionProcessor Enhancement

**File:** `src/processors/structure_detection.rs`
**Location:** Lines 334-352 (process method)

**Current:**

```rust
fn process(&self, mut document: Document) -> Result<Document> {
    let caption_regex = Regex::new(r"^(Figure|Fig\.|Table|Tab\.)\s*\d+[:.]").unwrap();

    for page in &mut document.pages {
        for block in &mut page.blocks {
            if block.block_type != BlockType::Text {
                continue;
            }
            let text = block.text.trim();
            if caption_regex.is_match(text) {
                block.block_type = BlockType::Caption;
            }
        }
    }
    Ok(document)
}
```

**New:**

1. First pass: Mark blocks matching regex as Caption
2. Second pass: For each Caption ending with hyphen, check if next block is continuation
3. Mark continuation blocks as Caption too

### Change 2: render_caption() Format Update

**File:** `src/renderers/markdown.rs`
**Location:** Line ~666

**Current:**

```rust
fn render_caption(&self, block: &Block, output: &mut String) {
    let text = self.clean_text(&block.text);
    output.push_str(&format!("*{}*\n\n", text));  // Italics
}
```

**New:**

```rust
fn render_caption(&self, block: &Block, output: &mut String) {
    let text = self.clean_text(&block.text);
    // WHY blockquote: Gold standard uses `> **Figure N:** description`
    // This provides visual separation and semantic meaning
    output.push_str(&format!("> {}\n>\n\n", text));
}
```

## Expected Outcome

Before:

```
*Figure 1.Illustration of a LLM navigating through a code reposi-*

tory. The LLM is equipped...
```

After:

```
> Figure 1. Illustration of a LLM navigating through a code repository. The LLM is equipped with a single yet powerful tool: jump, which is realized through a language server.
>
```

## Risk Assessment

| Risk                             | Mitigation                                               |
| -------------------------------- | -------------------------------------------------------- |
| Over-merging unrelated blocks    | Only merge if hyphenation detected or sentence continues |
| Breaking existing captions       | Run comprehensive tests to verify                        |
| Format change breaking consumers | Blockquote is standard markdown, widely supported        |

## Priority: HIGH

This directly improves structural fidelity score by properly handling figure captions.
