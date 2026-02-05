# OODA-52: List Detection Enhancement

## Date: 2026-02-05 (Planned)

## Observe

List detection relies on bullet character patterns.

### Current State

- Detects top-level bullets (•, -, \*, etc.)
- Marks blocks as `BlockType::ListItem`
- No indentation tracking

### Issues

- Nested lists appear flat
- No sub-item detection
- Indentation lost in output

## Orient

Need to track x-coordinate indentation for list nesting.

## Decide

Add indentation-based list level detection.

## Act

**Status:** PLANNED

Changes to make:

1. Calculate base indentation from first bullet
2. Detect indentation levels (every ~20pt = one level)
3. Generate proper nested markdown (`  -` for sub-items)
4. Test with real PDF list examples

**Expected Impact:** Structure 0.50 → 0.55
