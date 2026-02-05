# OODA-49: pymupdf_renderer.rs Refactoring

## Date: 2026-02-05 (Planned)

## Observe

`pymupdf_renderer.rs` handles markdown rendering.

### Current State

- Renders blocks to markdown
- Handles bold/italic inline formatting
- Applies header prefixes

### Issues

- Format score is 0.659 (below target 0.95)
- Bold/italic detection mixed with rendering logic

## Orient

SRP violation: renderer does both style detection AND output formatting.

## Decide

Extract style detection to separate helper module.

## Act

**Status:** PLANNED

Changes to make:

1. Create `inline_styles.rs` for bold/italic detection
2. Simplify `pymupdf_renderer.rs` to just rendering
3. Add tests for inline style detection
