# Iteration 06: ORIENT - Analysis

## Current State

The span style preservation fix (IT05) is complete:

- `pdfium_backend.rs` now populates `block.spans` with styled TextSpans
- The markdown renderer already uses `block.spans` for styling

## Gap Analysis

### What Works

1. ✅ `convert_span_to_text_span()` - Tested
2. ✅ `convert_text_block_to_schema_block()` - Tested
3. ✅ `render_spans_styled()` - Has existing tests

### What's Missing

1. ❌ End-to-end test verifying styled PDF text → bold markdown output
2. ❌ Verification with real PDF extraction

## Test Strategy

Create an integration-level test that:

1. Uses `MarkdownRenderer` directly
2. Creates a schema::Block with styled spans
3. Renders to markdown
4. Asserts `**bold**` and `*italic*` markers present

This fills the gap between unit tests and real PDF extraction.

## Risk Assessment

| Risk                             | Likelihood | Impact | Mitigation               |
| -------------------------------- | ---------- | ------ | ------------------------ |
| Renderer ignores spans           | Low        | High   | Add explicit test        |
| consolidate_spans breaks styles  | Low        | Medium | Test adjacent spans      |
| Style markers placed incorrectly | Medium     | Medium | Test whitespace handling |
