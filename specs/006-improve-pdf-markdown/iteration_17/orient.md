# OODA-17: Orient - Add Column Layout ASCII Diagram

## Analysis

### Two-Column Layout Algorithm

The algorithm processes a page in these phases:
1. Calculate adaptive region thresholds
2. Classify elements into: spanning, left_column, right_column, left_footer, right_footer
3. Process each region separately
4. Merge results in reading order

### Page Zones (Y-normalized: Y=0 is TOP)

```
Y=0    ┌────────────────────────────────────────────┐
       │ HEADER ZONE (Y < header_threshold)          │
Y=15   ├────────────────────────────────────────────┤
       │ TITLE ZONE (small Y, large font)            │
Y=80   ├────────────────────────────────────────────┤
       │ AUTHOR ZONE (15 < Y < 80)                   │
Y=100  ├────────────────────────────────────────────┤
       │           SPANNING ELEMENTS                 │
       ├──────────────────┬─────────────────────────┤
       │   LEFT COLUMN    │    RIGHT COLUMN         │
       │   X < boundary-15│    X > boundary+15      │
       │                  │                          │
       │                  │                          │
Y=700  ├──────────────────┴─────────────────────────┤
       │ FOOTER ZONE (Y > footer_threshold)          │
Y=792  └────────────────────────────────────────────┘
```

### Column Boundary Detection

- boundary = page_width / 2 (~306pt for US Letter)
- margin = 15pt for gap zone
- Elements in gap zone use heuristics

## Prioritization

Add ASCII diagram to module doc comment explaining:
1. Y-coordinate system (Y=0 at top)
2. Region zones with thresholds
3. Column boundary with margin zones
