# IT34 — Observe

## Problem Statement

The edgequake-pdf crate has accumulated verbose INFO-level logging in production hot paths:

- Column detection: 6+ INFO messages per page (COLUMN-DETECT prefix)
- Reading order: 4+ INFO messages per page (READING-ORDER prefix)
- Table detection: 6+ INFO messages per table caption
- Layout processing: 8+ INFO messages per page
- Structure detection: 3+ INFO messages per page
- Markdown renderer: 1 INFO message per Table block

For a 16-page PDF like lighrag_2410.05779v3.pdf, this produces ~60+ unnecessary log lines.

Additionally, 5 clippy warnings exist in edgequake-pdf:

1. `skip(..).next()` instead of `nth()` in table_detection.rs
2. Doc list item indentation in markdown.rs (2 warnings)
3. Manual prefix stripping in markdown.rs
4. Manual suffix stripping in markdown.rs

## Evidence

Running the lighrag PDF before IT34 produces:

```
2026-02-06T04:22:50.428275Z  INFO Starting PDF extraction...
2026-02-06T04:22:50.435434Z  INFO COLUMN-DETECT: 14 items, page_width=612
2026-02-06T04:22:50.435439Z  INFO COLUMN-DETECT: filtered 14 items to 14...
2026-02-06T04:22:50.435447Z  INFO COLUMN-DETECT: clusters too close...
...60+ more INFO lines...
```
