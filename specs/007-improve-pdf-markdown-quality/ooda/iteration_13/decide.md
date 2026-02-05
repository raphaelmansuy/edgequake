# IT13 Decide: Fix Code Block False Positives

## Decision

**Add content-based filtering to exclude emails and URLs from code block detection.**

## Implementation Plan

### 1. Create Content Filter Helper Functions

**Location**: `src/processors/structure_detection.rs`

```rust
/// Check if text looks like email addresses only (not code).
/// 
/// WHY: Emails often appear in monospace fonts in academic PDFs
/// but should NOT be marked as code blocks.
fn is_email_only_content(text: &str) -> bool {
    let trimmed = text.trim();
    // Check each word - all must be email addresses
    trimmed.split_whitespace().all(|word| {
        // Simple email pattern: word@domain
        word.contains('@') && word.contains('.') && !word.contains("=")
    })
}

/// Check if text looks like URL only (not code).
fn is_url_only_content(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("www.")
        || trimmed.starts_with("ftp://")
}
```

### 2. Modify CodeBlockDetectionProcessor

**Add content filter before marking as code:**

```rust
// In CodeBlockDetectionProcessor::process()

let all_code = !block.spans.is_empty()
    && block.spans.iter().all(|s| s.style.looks_like_code());

// NEW: Content-based filtering
let excluded_by_content = is_email_only_content(&block.text)
    || is_url_only_content(&block.text);

if all_code && !excluded_by_content {
    block.block_type = BlockType::Code;
}
```

### 3. Add Tests

```rust
#[test]
fn test_email_not_code() {
    // Test that emails in monospace font are NOT marked as code
}

#[test]
fn test_url_not_code() {
    // Test that URLs in monospace font are NOT marked as code
}
```

## Rationale

1. **Minimal Change**: Add simple content checks rather than overhauling detection
2. **High Precision**: Email and URL patterns are unambiguous
3. **No False Negatives**: Real code rarely consists of only emails/URLs
4. **Testable**: Easy to add unit tests for edge cases

## Expected Outcome

- LightRAG paper output: 4 code blocks → 2 code blocks (or 0 if all are false positives)
- No regression in actual code block detection

## Commit Message

```
OODA-IT13: Exclude emails/URLs from code block detection

Code blocks were incorrectly detecting email addresses and URLs
rendered in monospace fonts. Added content-based filtering to
exclude patterns that are clearly not code.

WHY: Academic PDFs often use monospace for author emails, but
these should not appear as code blocks in the output.
```
