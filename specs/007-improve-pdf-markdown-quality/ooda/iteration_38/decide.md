# OODA-38 Decide

## Changes Planned

1. **SectionNumberMergeProcessor** — Add Mode B (below-number) matching
   - Mode A: Same line (title to right, Y < 25pt) — PRIORITY
   - Mode B: Next line (title below, Y < 40pt, X within ±20pt) — FALLBACK
   - Mode A always wins over Mode B when both match

2. **GarbledTextFilterProcessor** — Lower thresholds + CamelCase detection
   - Remove outer guard (was `trimmed.len() > 50`)
   - Lower per-word threshold from >40 to >35 chars
   - Add CamelCase check: word >25 chars with ≥2 internal uppercase
   - Add proportion guard: don't filter long paragraphs with one garbled word
   - Lower space-ratio threshold from >80 to >60 chars
   - Add URL exception for space-ratio check

3. **looks_like_section_title** — ALL-CAPS fast path
   - If all alphabetic chars are uppercase → return true (section title, not person name)
   - More robust than expanding the keyword list
