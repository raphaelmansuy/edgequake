# Iteration 09: DECIDE - Extend Monospace Font Detection

## Decision

Expand `looks_like_code()` in `schema/block.rs` with comprehensive
font pattern detection covering 20+ monospace fonts.

## Font Categories to Add

### 1. Programming Fonts

- `fira` (Fira Code, Fira Mono)
- `inconsolata`
- `jetbrains`
- `hack`
- `iosevka`

### 2. System Monospace Fonts

- `monaco` (macOS - doesn't contain "mono"!)
- `menlo` (macOS)
- `sf mono` (macOS)
- `lucida console` (Windows)
- `dejavu sans mono`
- `liberation mono`
- `ubuntu mono`
- `roboto mono`

### 3. Classic/Terminal Fonts

- `typewriter`
- `terminal`
- `fixedsys`
- `fixed`
- `letter gothic`
- `prestige`
- `ocr` (OCR-A, OCR-B)

## Implementation Plan

```
[x] Read schema/block.rs looks_like_code()
[x] Add font pattern checks for all categories
[x] Add test for extended monospace detection
[x] Run all code tests (16 should pass)
[x] Run full test suite (516 should pass)
[ ] Commit changes
```

## Expected Outcomes

- Better inline code detection with backticks
- More code blocks properly identified
- No false positives (patterns are specific)
- Code score improvement: 70→75+ (estimated)
