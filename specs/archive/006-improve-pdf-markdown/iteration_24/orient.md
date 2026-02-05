# OODA-24 Orient: Analysis of Encodings Documentation Gap

## Context

PDF font encoding is one of the most confusing aspects of PDF extraction. Developers encountering this code need to understand:

1. Why PDF fonts don't just use UTF-8 like modern documents
2. What the magic byte values for ligatures mean
3. How ToUnicode CMaps work as escape hatches

## Risk Assessment

| Factor             | Risk   | Mitigation                              |
| ------------------ | ------ | --------------------------------------- |
| Complex domain     | Medium | Add WHY comments explaining PDF history |
| Magic numbers      | High   | Document the PostScript/Windows origins |
| Encoding fallbacks | Low    | Already has good fallback chain         |

## Decision Factors

**Add WHY comments to:**

1. `get_ligature_expansion()` - Explain byte value origins
2. `Encoding::Identity` decode arm - Explain UTF-16BE format
3. `ToUnicodeMap::parse()` - Add ASCII diagram of CMap format

**Don't touch:**

- The static encoding tables (they're self-documenting data)
- Test functions (already clear)

## Alignment with Mission

Mission 006 goals:

- ✅ High signal comments explaining WHY → Adding to encoding functions
- ✅ Clean code quality → Module already clean, adding docs
- ✅ No clippy errors → Currently 0 warnings

## Decision

Add 3-4 targeted WHY comments to the most complex functions that handle encoding edge cases.
