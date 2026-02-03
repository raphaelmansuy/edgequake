# OODA-21: ArXiv Metadata Extraction - DECIDE

## Decision

Extract arXiv identifier from rotated text and place it after the title.

## Implementation Plan

### Step 1: Modify extraction_engine.rs

Instead of just filtering rotated elements, extract arXiv identifiers:

```rust
// Check if rotated element is an arXiv identifier
fn extract_arxiv_id(rotated_elements: &[TextElement]) -> Option<String> {
    for elem in rotated_elements {
        if elem.text.contains("arXiv:") {
            return Some(elem.text.trim().to_string());
        }
    }
    None
}
```

### Step 2: Pass arXiv ID to text grouper

Add arXiv metadata to page extraction result.

### Step 3: Insert arXiv at document top

In markdown renderer, if arXiv ID present:

- Insert after title header
- Format as bold: `**arXiv:...**`

## Expected Outcome

- ArXiv identifier appears after title in markdown
- Matches gold standard format
- Text preservation score increases for arXiv papers

## Code Locations

- extraction_engine.rs: Extract arXiv from rotated text
- text_grouping.rs: Pass through metadata
- markdown_renderer.rs: Insert arXiv line

## Test Verification

```bash
cargo build -p edgequake-pdf --release
cargo test -p edgequake-pdf --test comprehensive_quality --features comprehensive-tests --release
```
