# OODA-55: Code Block Detection

## Date: 2026-02-05 (Planned)

## Observe

Code blocks detected by monospace font check.

### Current State

- `Span::is_monospace()` checks font name
- Looks for "Mono", "Courier", "Console"
- All-monospace blocks become `BlockType::Code`

### Issues

- Some monospace fonts not detected
- Inline code not handled
- False positives with monospace body text

## Orient

Need better monospace font detection and inline code handling.

## Decide

Improve font family matching and add inline code detection.

## Act

**Status:** PLANNED

Changes to make:

1. Expand monospace font patterns
2. Add inline code detection (single span in non-mono line)
3. Use backticks for inline, triple for blocks
4. Test with various PDF code samples

**Expected Impact:** Structure 0.65 → 0.70
