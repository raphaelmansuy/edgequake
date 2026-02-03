# OODA-25 Act: Caption Handling Implementation

## Changes Made

### 1. CaptionDetectionProcessor Enhancement

**File:** `src/processors/structure_detection.rs`
**Lines:** 310-475 (rewritten)

**Added functionality:**

- `caption_continues(&self, caption: &Block, next: &Block) -> bool`
  - Detects if next block is caption continuation
  - Checks: ends with hyphen, next starts lowercase, vertically adjacent
- `merge_caption_text(&self, caption_text: &str, continuation_text: &str) -> String`
  - Merges caption with continuation, handling hyphenation
- Two-pass processing in `process()`:
  1. Mark blocks matching "Figure N:" or "Table N:" pattern
  2. Find and merge continuation blocks

### 2. Caption Render Format Update

**File:** `src/renderers/markdown.rs`
**Lines:** 665-678

**Change:**

- Old: `*{}*\n\n` (italics)
- New: `> {}\n>\n\n` (blockquote)

**WHY blockquote:**

- Gold standard uses `> Figure N: description` format
- Provides visual separation from body text
- Semantically marks captions as distinct content

## Test Results

```
$ cargo test --lib
test result: ok. 415 passed; 0 failed; 0 ignored
```

## Output Verification

### Before (OODA-24):

```markdown
_Figure 1.Illustration of a LLM navigating through a code reposi-_

tory. The LLM is equipped with a single yet powerful tool:jump...
```

### After (OODA-25):

```markdown
> Figure 1.Illustration of a LLM navigating through a code repository. The LLM is equipped with a single yet powerful tool:jump,
```

## Improvements

1. ✅ Captions now use blockquote format
2. ✅ "reposi-" + "tory" merged to "repository"
3. ✅ Caption continuation detection working

## Remaining Issues (Future OODA)

- Some caption text appears in wrong reading order (two-column layout issue)
- The continuation "which is realized through a language server" needs better merging

## Commit

```bash
git commit -m "OODA-25: Caption continuation detection and blockquote format"
```

Files changed:

- `src/processors/structure_detection.rs`
- `src/renderers/markdown.rs`
