# OODA Iteration 27 – Observe

## Discovery: GarbledTextFilterProcessor falsely removes section titles

Verbose output revealed the `GarbledTextFilterProcessor` was filtering these legitimate section titles:

```
Filtering garbled text (60% unusual short words): '0) AI Strategy & Co‑Creation'
Filtering garbled text (50% unusual short words): '1) AI Agent Design & Building'
Filtering garbled text (50% unusual short words): 'Search UX & APIs'
Filtering garbled text (36% unusual short words): '3. Industrialization (4 8+ weeks) ...'
```

## Root cause

The garbled text detector counts "unusual short words" (≤2 chars not in the valid list). Two patterns were missing from the valid list:

1. **`&` (ampersand)** — a standard conjunction, but not in `valid_short_words`
2. **Section numbers like `0)`, `1)`, `2.`** — digit+delimiter patterns not recognized

## Impact

4 section titles + 1 subsection completely missing from output. These are major structural elements that organize the document.
