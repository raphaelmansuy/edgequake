# OODA Iteration 34 - Observe Phase

## Objective

Identify quality gaps between extracted markdown and gold standard to prioritize improvements.

## Side-by-Side Comparison: agent_2510.09244v1

### Gold Standard (Expected)

```markdown
**Victor de Lamo Castrillo¹, Habtom Kahsay Gidey², Alexander Lenz², and Alois Knoll²**

¹ Universitat Politècnica de Catalunya, Barcelona, Spain `victor.de.lamo@estudiantat.upc.edu`
² Technische Universität München, München, Germany `{habtom.gidey, alex.lenz, knoll}@tum.de`
```

### Our Extraction (Actual)

```markdown
Victor de Lamo Castrillo, Habtom Kahsay Gidey

, Alexander Lenz, and

Alois Knoll

Universitat Politècnica de Catalunya, Barcelona, Spain

victor.de.lamo@estudiantat.upc.edu

Technische Universität München, München, Germany

{habtom.gidey, alex.lenz, knoll}@tum.de
```

## Quality Issues Identified

### Issue 1: Reading Order Line Breaks (SFS Impact: HIGH)

- **Problem**: Names split across multiple lines with spurious newlines
- **Expected**: `Victor de Lamo Castrillo, Habtom Kahsay Gidey, Alexander Lenz, and Alois Knoll`
- **Actual**: Names on separate lines with trailing commas hanging
- **Root Cause**: Text grouper not merging lines that flow together
- **SFS Impact**: ~5-10% penalty

### Issue 2: Missing Bold Formatting (SFS Impact: MEDIUM)

- **Problem**: Author names should be **bold** but appear plain
- **Expected**: `**Victor de Lamo Castrillo¹...**`
- **Actual**: `Victor de Lamo Castrillo...`
- **Root Cause**: Font weight detection not triggering bold markers
- **SFS Impact**: ~3-5% penalty

### Issue 3: Missing Superscript Affiliation Numbers (SFS Impact: MEDIUM)

- **Problem**: Superscript numbers (¹, ²) not preserved
- **Expected**: `Castrillo¹, Gidey²`
- **Actual**: `Castrillo, Gidey`
- **Root Cause**: Superscript text not being detected/preserved
- **SFS Impact**: ~2-3% penalty

### Issue 4: Missing Code/Monospace for Emails (SFS Impact: LOW)

- **Problem**: Emails should be in backticks
- **Expected**: `` `victor.de.lamo@estudiantat.upc.edu` ``
- **Actual**: `victor.de.lamo@estudiantat.upc.edu`
- **Root Cause**: No monospace font detection → code marker
- **SFS Impact**: ~1-2% penalty

### Issue 5: Italics Fragmentation (SFS Impact: MEDIUM)

- **Problem**: Course name italics split across lines
- **Expected**: `*Trends in Autonomous Agents: Advances in Architecture and Practice*`
- **Actual**:
  ```
   *Trends in Au-*
  *tonomous Agents: Advances in Architecture and Practice*
  ```
- **Root Cause**: Hyphenated line break not being rejoined
- **SFS Impact**: ~3-5% penalty

### Issue 6: Truncated Numbered List Items (SFS Impact: HIGH)

- **Problem**: List item 2 is truncated
- **Expected**: `2. Examine reasoning architectures, such as Chain-of-Thought (CoT) and Tree-of-Thought (ToT)...`
- **Actual**: `2. and Tree-of-Thought (ToT)...` (missing most of the sentence!)
- **Root Cause**: Reading order algorithm losing text
- **SFS Impact**: ~5-10% penalty (text loss is critical)

## Summary of Quality Gap Analysis

| Issue                 | Impact       | Difficulty | Priority |
| --------------------- | ------------ | ---------- | -------- |
| Truncated list items  | HIGH (~10%)  | HIGH       | P0       |
| Reading order breaks  | HIGH (~10%)  | MEDIUM     | P0       |
| Italics fragmentation | MEDIUM (~5%) | MEDIUM     | P1       |
| Missing bold          | MEDIUM (~5%) | LOW        | P2       |
| Missing superscripts  | MEDIUM (~3%) | MEDIUM     | P2       |
| Missing code markers  | LOW (~2%)    | LOW        | P3       |

**Total Estimated SFS Gap: ~35%** (matches observed 68% vs 95% target = 27% gap)

## Test Verification

```bash
cargo test --test quick_smoke --release 2>&1 | tail -3
# Expected: all passing
```

## Metrics Before This Iteration

- TPS: 81.3%
- SFS: 68.0%
- Speed: ✅ ACHIEVED (0.028-0.104s/page)
