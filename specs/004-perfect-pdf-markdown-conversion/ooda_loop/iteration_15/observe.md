# OODA-15: Observe

## Current State (Post OODA-14)

- **Text Preservation**: 84.9%
- **Structure Fidelity**: 81.0%
- **Overall Quality**: 83.0%
- **Target**: 95%+
- **Gap**: 12.0 percentage points

## Observations

### 1. Table 1 in AlphaEvolve is NOT Detected

The FunSearch vs AlphaEvolve comparison table (Table 1) is extracted as:

```
*FunSearch[83] AlphaEvolve*

evolves single function evolves entire code file evolves up to 10-20 lines of code...
```

**Expected** (from gold file):

```markdown
| FunSearch [83]          | AlphaEvolve              |
| ----------------------- | ------------------------ |
| evolves single function | evolves entire code file |
```

### 2. Root Cause: Missing Stream Table Detection

Current table detection uses **Lattice method** only:

- Relies on PDF vector lines (borders) to detect tables
- AlphaEvolve Table 1 is a **borderless table** - no lines, just aligned text

Camelot documentation identifies 3 table detection methods:

1. **Lattice**: Line-based (what we have)
2. **Stream**: Whitespace/alignment-based (MISSING)
3. **Network**: Graph-based text alignment (advanced)

### 3. Block Merging Issue

The table content is being extracted as single blocks per row:

- "evolves single function evolves entire code file" - merged into one line
- Left column and right column text concatenated without separation

### 4. Quality Impact

AlphaEvolve Structure score: **76.2%** (lowest in dataset)

- Table 1 contributes significant structure mismatch
- Table 2 (matrix multiplication results) likely also affected

### 5. Page Detection Analysis

From diagnose_tables output:

- 44 pages in AlphaEvolve detected as 2-column layout
- Only 1 table detected (on page 34) - uses Lattice detection
- Table 1 on page ~3 is NOT detected

## Data Points

```
Gold file Table 1 format:
Line 27: | FunSearch [83] | AlphaEvolve |
Line 28: | --- | --- |
Line 29: | evolves single function | evolves entire code file |

Our extraction:
Line 112: *FunSearch[83] AlphaEvolve*
Line 114: evolves single function evolves entire code file...
```

## Files Analyzed

- `/tmp/alpha_extracted.md` - Our extraction output
- `test-data/real_dataset/AlphaEvolve.gold.md` - Gold standard
- `src/backend/lattice.rs` - Current table detection (Lattice only)
- `src/processors/table_detection.rs` - Table block processing
