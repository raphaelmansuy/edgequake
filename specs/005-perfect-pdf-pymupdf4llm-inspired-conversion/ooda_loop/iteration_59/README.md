# OODA-59: Whitespace Handling

## Date: 2026-02-05 (Planned)

## Observe

Word spacing sometimes incorrect.

### Current State

- Spaces treated as word boundaries
- Gap between chars detected heuristically
- Some words merged, others split incorrectly

### Issues

- "HelloWorld" instead of "Hello World"
- "H e l l o" instead of "Hello"
- Depends on PDF creation tool

## Orient

Need more robust word boundary detection.

## Decide

Improve space detection using character width analysis.

## Act

**Status:** PLANNED

Changes to make:

1. Calculate expected space width from font metrics
2. Detect natural word breaks by gap > expected space
3. Don't insert extra spaces for tight kerning
4. Test with PDFs from different generators

**Expected Impact:** Format 0.80 → 0.85
