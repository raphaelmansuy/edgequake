# OODA Loop 4 - DECIDE

**Date:** 2026-01-03  
**Directory Scope:** `crates/edgequake-pdf/src/backend/lattice.rs`  
**Fix:** Implement text position clustering to split under-gridded cells

## Patch Plan

### Change 1: Add X-Position Clustering Function

**Location:** `lattice.rs`, before `extract_text_in_rect()`

**New function:**

```rust
/// Cluster text elements by X-position.
/// Returns groups of elements that are horizontally aligned (within tolerance).
fn cluster_by_x_position<'a>(
    elements: &[&'a TextElement],
    tolerance: f32,
) -> Vec<Vec<&'a TextElement>> {
    if elements.is_empty() {
        return vec![];
    }

    // Sort by X position
    let mut sorted: Vec<&TextElement> = elements.iter().copied().collect();
    sorted.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));

    let mut clusters: Vec<Vec<&TextElement>> = vec![];
    let mut current_cluster: Vec<&TextElement> = vec![sorted[0]];
    let mut cluster_x = sorted[0].x;

    for elem in sorted.iter().skip(1) {
        if (elem.x - cluster_x).abs() <= tolerance {
            // Same cluster
            current_cluster.push(elem);
        } else {
            // New cluster
            clusters.push(current_cluster);
            current_cluster = vec![elem];
            cluster_x = elem.x;
        }
    }

    if !current_cluster.is_empty() {
        clusters.push(current_cluster);
    }

    clusters
}
```

**First Principles Rationale:**

- Text elements at similar X-coordinates belong to same column
- Tolerance of 5pt handles minor positioning variations
- Clustering is more robust than hard thresholds

### Change 2: Modify extract_text_in_rect to Return Vec<String>

**Location:** `lattice.rs`, function `extract_text_in_rect`

**Changes:**

1. Return type: `String` → `Vec<String>`
2. After filtering and sorting elements, call `cluster_by_x_position()`
3. For each cluster, concatenate text elements
4. Return vector of cluster texts

**Pseudocode:**

```rust
fn extract_text_in_rect(...) -> Vec<String> {
    let contained: Vec<&TextElement> = /* existing filtering logic */;

    // Sort by Y first (to group rows)
    contained.sort_by(/* existing Y-then-X sort */);

    // Group by Y-position to handle rows separately
    let rows = group_by_y_position(&contained, 2.0); // 2pt tolerance for same row

    let mut all_columns: Vec<String> = vec![];

    for row_elements in rows {
        // Cluster by X-position within this row
        let clusters = cluster_by_x_position(&row_elements, 5.0);

        for (i, cluster) in clusters.iter().enumerate() {
            if i >= all_columns.len() {
                all_columns.resize(i + 1, String::new());
            }

            // Add text from this cluster to corresponding column
            let cluster_text: String = cluster
                .iter()
                .filter(|e| !is_decorative(e))
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            if !all_columns[i].is_empty() && !cluster_text.is_empty() {
                all_columns[i].push(' ');
            }
            all_columns[i].push_str(&cluster_text);
        }
    }

    // Remove empty columns
    all_columns.into_iter().filter(|s| !s.is_empty()).collect()
}
```

**Wait - this gets complicated with multiple rows in one cell.**

**Simpler approach:**

```rust
fn extract_text_in_rect(...) -> Vec<String> {
    let mut contained: Vec<&TextElement> = /* existing filtering */;

    // Sort by Y then X
    contained.sort_by(/* existing logic */);

    // Just cluster by X - ignore Y for now
    let clusters = cluster_by_x_position(&contained, 5.0);

    clusters
        .into_iter()
        .map(|cluster| {
            cluster
                .iter()
                .filter(|e| !is_decorative(e))
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|s| !s.trim().is_empty())
        .collect()
}
```

### Change 3: Update Cell Extraction to Handle Multiple Subcells

**Location:** `lattice.rs`, lines 388-402 (cell extraction loop)

**Current:**

```rust
for j in 0..unique_x.len() - 1 {
    let left = unique_x[j];
    let right = unique_x[j + 1];
    let cell_text = self.extract_text_in_rect(text_elements, left, bottom, right, top);
    row_cells.push(cell_text);
}
```

**Changed:**

```rust
for j in 0..unique_x.len() - 1 {
    let left = unique_x[j];
    let right = unique_x[j + 1];
    let cell_texts = self.extract_text_in_rect(text_elements, left, bottom, right, top);

    // If cell contains multiple columns, add them all
    if cell_texts.is_empty() {
        row_cells.push(String::new());
    } else if cell_texts.len() == 1 {
        row_cells.push(cell_texts[0].clone());
    } else {
        // Multiple subcells detected - add each as separate column
        for text in cell_texts {
            row_cells.push(text);
        }
    }
}
```

**Problem:** This breaks the grid structure - rows might have different column counts!

**Better approach:** Only split if ALL cells in column have same number of subcells.

Actually, **SIMPLER APPROACH:**

Just return single String but use better clustering internally:

```rust
fn extract_text_in_rect(...) -> String {
    let contained: Vec<&TextElement> = /* filter by bbox */;

    // Sort by Y-position first to preserve reading order
    contained.sort_by(/* existing Y-then-X logic */);

    // Cluster by X-position
    let clusters = cluster_by_x_position(&contained, 5.0);

    // Join clusters with " | " to mark column boundaries
    clusters
        .into_iter()
        .map(|cluster| {
            cluster
                .iter()
                .filter(|e| !is_decorative(e))
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" | ")
}
```

Then later parse " | " to split cells!

**Actually this is getting too complex.**

## Revised Decision: KISS Approach

**Problem:** We're trying to fix a structural issue (under-gridded tables) within the existing lattice framework.

**Better approach:** Accept that lattice detector works for PROPERLY GRIDDED tables. Don't try to make it handle under-gridded tables.

**Instead:**

1. Keep lattice.rs focused on lattice tables
2. Add SEPARATE detector for under-gridded/whitespace tables
3. Use text-position-based column detection for that detector

**This aligns with first principles:**

- Lattice detector: for tables with full grid lines
- Whitespace detector: for tables with minimal/no grid lines

## Final Decision

**Defer cell splitting within lattice detector.**

**Reason:** Adding column inference to lattice detector violates single responsibility principle and adds complexity.

**Alternative: Post-processing fix**

Add a post-processing step that:

1. Detects when a cell contains text with large X-gaps
2. Splits such cells by inserting additional columns
3. Re-balances table structure

**Implementation:**

```rust
fn fix_undergridded_cells(rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    // For each cell, check if it has text clusters with large X-gaps
    // If so, split it into multiple cells
    // Rebalance column counts across rows
}
```

**Actually, this is STILL complex.**

## REAL Decision (First Principles)

**The validator is correct.** The generated markdown is wrong.

**The code is handling gridded tables correctly.** The problem is that one_tool PDF is under-gridded.

**Solution:** Don't try to make lattice detector handle under-gridded tables.

**Instead:** Fix the VALIDATOR or GOLD DATA to reflect that some PDFs are inherently ambiguous.

**NO WAIT - that's avoiding the problem!**

## TRUE First Principles Decision

Looking at the actual error again:

**GOLD line 135:**

```
| Agent Pipeline | Model | Function-level Recall | Funct Precision | ...
```

**GENERATED line 377:**

```
| CoSIL | Training | Free | 48.61 | ...
```

The problem is NOT just cell splitting. The problem is:

1. **Wrong starting row** - generator started mid-table instead of at header
2. **Wrong column count** - 8 columns instead of 10

This suggests the lattice detector is finding WRONG grid lines or MISSING grid lines!

**Let me check if the PDF actually has 10 vertical lines or fewer.**

Actually, I need to see the actual lattice detection output for this table.

## Simplified Decision

**Acceptance Criteria:**

1. one_tool table accuracy improves from 11% to >30%
2. No regression in 2900_Goyal (currently 0% table, but 37% style)
3. Code changes < 100 lines

**Patch:**
Add diagnostic output to understand what grid lines are being detected for one_tool tables.

Then decide on fix based on that data.

**Next:** ACT phase will add debug output and analyze actual grid structure.
