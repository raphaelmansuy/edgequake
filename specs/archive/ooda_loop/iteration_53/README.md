# OODA-53: Section Number Preservation

## Date: 2026-02-05 (Planned)

## Observe

Section numbers (1., 2.1., etc.) are sometimes stripped.

### Current State

- Pattern detection disabled (OODA-10/11)
- Numbers not classified as headers
- But may be stripped in rendering

### Issues

- "1. Introduction" becomes "Introduction"
- Loses document structure reference

## Orient

Need to preserve section numbers in rendered text.

## Decide

Keep section numbers as part of the text content.

## Act

**Status:** PLANNED

Changes to make:

1. Verify section numbers pass through grouper
2. Check renderer doesn't strip numeric prefixes
3. Add test case for numbered sections
4. Validate against gold standards

**Expected Impact:** Structure 0.55 → 0.60
