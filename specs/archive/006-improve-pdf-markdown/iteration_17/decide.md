# OODA-17: Decide - Add Column Layout Diagram to Module Docs

## Decision

Enhance the module doc comment in text_grouping.rs with ASCII diagram.

## Implementation Plan

Add detailed module documentation explaining:

1. Two-column layout detection algorithm
2. Page zones with Y thresholds
3. Column boundary with margin zones
4. Reading order for academic papers

## New Module Docs

````rust
//! Text grouping and line merging utilities.
//!
//! ## Two-Column Layout Detection
//!
//! Academic papers often use two-column layouts. This module detects and handles
//! them by:
//!
//! 1. **Detecting column boundary** at ~page_width/2
//! 2. **Classifying elements** into regions based on Y and X position
//! 3. **Processing columns** separately to preserve reading order
//! 4. **Merging results** with proper interleaving
//!
//! ## Page Zones (Y-normalized: Y=0 is TOP of page)
//!
//! ```text
//! Y=0    ┌────────────────────────────────────────┐
//!        │ HEADER (running headers, page numbers)  │
//! Y=15   ├────────────────────────────────────────┤
//!        │ TITLE/AUTHORS (spanning, large font)    │
//! Y=100  ├────────────────────────────────────────┤
//!        │         │ column  │                     │
//!        │  LEFT   │ boundary│  RIGHT              │
//!        │ COLUMN  │  ±15pt  │ COLUMN              │
//!        │         │ margin  │                     │
//! Y=700  ├────────────────────────────────────────┤
//!        │ FOOTER (page numbers, affiliations)     │
//! Y=792  └────────────────────────────────────────┘
//! ```
//!
//! ## Reading Order
//!
//! For two-column papers, content is read column-by-column:
//! - All of left column top-to-bottom
//! - All of right column top-to-bottom
//! - NOT interleaved by Y position
````

## Risk Assessment

- **Risk**: Low - documentation only
- **Benefit**: High - clarifies complex algorithm

## Success Criteria

- [ ] ASCII diagram added to module docs
- [ ] Tests pass
- [ ] Documentation compiles
