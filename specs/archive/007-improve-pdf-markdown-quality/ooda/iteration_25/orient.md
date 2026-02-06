# OODA Iteration 25 – Orient

## Analysis

The `convert_standalone_bold_to_headers()` function uses these criteria to promote a standalone bold line to a `##` header:

1. Short (< 60 chars) ✅
2. First char is uppercase ❌ — fails for "2) Software..."
3. Doesn't end with `:`, `.`, `?`, `;`
4. Not a caption pattern (Figure X, Table X)

Section-numbered titles are a common document pattern:

```
0. Section Title
1) Section Title
2. Section Title
```

These fail criterion #2 because they start with a digit, not a letter.

## Fix

Add `starts_with_section_number` check: first char is ASCII digit. Combined with the other constraints (bold, standalone, short, no trailing punctuation), this is highly selective — only section headers match, not regular numbered content.

## Safety

False positive risk is very low because:

- The line must be entirely bold (`**...**`)
- Must be standalone (nothing else on the line)
- Must be short (< 60 chars)
- Must not end with common punctuation
- A digit + these constraints = section number header
