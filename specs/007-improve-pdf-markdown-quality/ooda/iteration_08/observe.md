# Iteration 08: OBSERVE - Table Detection & Rendering

## Focus Area

Tables are a critical priority (50→80 score target). Need to understand:
1. How tables are currently detected
2. How they're rendered to Markdown
3. What's failing (spanning, complex headers)

## Code Inventory

### Table Detection (`src/backend/lattice.rs`)

Primary table detection uses lattice-based approach:
- Looks for horizontal/vertical lines forming cell boundaries
- Detects table structure from line intersections
- R-tree spatial index for O(n log n) intersection queries
- DFS-based connected component detection
- Minimum 4 lines required (simplest table = box)

**Key functions:**
- `detect_tables()` - Main entry point (line 53)
- `filter_lines_enhanced()` - Filters decorative lines (line 386)
- `create_table_block()` - Builds table from detected grid (line 509)
- `merge_horizontal_table_halves()` - Handles split tables (line 168)

### Table Rendering (`src/renderers/markdown.rs`)

Two rendering paths:
1. `render_table_from_children()` - When block has child cells (line 595)
2. Direct text output - For lattice tables with pre-formatted markdown (line 585)

### Test Status

**19 table tests passing:**
- `test_empty_lines_no_tables`
- `test_simple_box_table_detection`
- `test_grid_table_detection`
- `test_table_like_score*`
- `test_table_rendering*`
- `test_table_caption_*`

**15 list tests passing:**
- `test_list_detection*`
- `test_nested_list_items`
- `test_list_item_rendering`

## Gold Test Data Analysis

**Tables (test-data/gold/05-tables/):** 15 test files
- Simple 2x3 to complex 5-column tables
- Long content cells, URL cells, formatted tables

**Lists (test-data/gold/04-lists/):** 15 test files
- Simple bullets/numbered to 5-level deep nesting
- Mixed lists, formatted items, task lists

## Observations

1. **Table detection is comprehensive** - Uses lattice algorithm, handles split tables
2. **Table rendering has two paths** - Children-based or pre-formatted text
3. **List handling is well-tested** - 15 passing tests covering edge cases
4. **Both areas have gold standards** - 15 test files each with expected output

## Areas for Investigation

Looking at the mission again:
- Tables: 50→80 (critical) - Current lattice detection may miss borderless tables
- Lists: 55→85 (high) - May need to improve nested list indentation detection

Let me check if there's a borderless table detection mechanism.

