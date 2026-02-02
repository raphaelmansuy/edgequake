# OODA Iteration 04 - Observe

## New Test Documents Evaluated

Three new PDFs in `zz_test_docs/` were tested:

| Document                     | Pages | Type          | MCP Markitdown | EdgeQuake Output |
| ---------------------------- | ----- | ------------- | -------------- | ---------------- |
| Apple-Sandbox-Guide-v1.0.pdf | 48    | Technical doc | ✅ 48 pages    | ⚠️ Garbled TOC   |
| agentfail_2601.22984v1.pdf   | 39    | arXiv paper   | ✅ 39 pages    | ⚠️ Order issues  |
| hotmess_2601.23045v1.pdf     | 40    | arXiv paper   | ✅ 40 pages    | ⚠️ Order issues  |

## Critical Issue: Reading Order Reversed

### Evidence from hotmess_2601.23045v1.pdf

**Markitdown output (correct)**:

```markdown
THE HOT MESS OF AI: HOW WILL AI FAIL WITH TASK COMPLEXITY?
...authors...
A B S T R A C T
As AI becomes more capable, we entrust it with more general...
```

**EdgeQuake output (reversed)**:

```markdown
these trained properties will tend to be more robust, and which...
(LLMs), prior to reinforcement learning, are dynamical systems...
However, this scenario assumes that unintended behavior...
...
As AI becomes more capable, we entrust it with more general...
```

The text appears in **reverse order** (bottom paragraphs first, abstract last).

## Root Cause Analysis

### Code Path

1. `extraction_engine.rs` L386-394:

   ```rust
   // Normal coordinate system: lower Y = bottom of page
   // Normalize by shifting: normalized_y = visual_y - min_y
   // This makes content at min_y become Y=0 (but still bottom-first)
   elements.into_iter().map(|mut e| { e.y -= min_y; e }).collect()
   ```

2. `text_grouping.rs` L62-63:
   ```rust
   // Sort by Y ascending (lower Y = top of page after normalization)
   // After Y-normalization in extraction engine, Y=0 is at top of page
   elements.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(...));
   ```

### The Bug

The comment in `text_grouping.rs` is **WRONG** for normal PDFs:

- For **flipped** PDFs: Y is correctly normalized (max_y - y) → Y=0 at top ✓
- For **normal** PDFs: Y is just shifted (y - min_y) → Y=0 at **bottom** ✗

Since `text_grouping.rs` sorts ascending Y, this puts **bottom content first**.

### Coordinate Flow

```
PDF (raw Y)    →    extraction_engine.rs    →    text_grouping.rs
                        ↓                             ↓
                   is_flipped=true             sorts ascending Y
                   max_y - y → OK                   ↓
                        ↓                       lower Y first
                   is_flipped=false                 ↓
                   y - min_y → WRONG!          bottom content first!
```

## Secondary Issues Observed

1. **Text Fragmentation**: Lines concatenated incorrectly ("failuresbe trained")
2. **TOC Corruption**: Apple Sandbox Guide Page 2 shows "55555..." patterns
3. **Author Block Misdetection**: Author affiliations parsed as table
4. **Formula Detection**: Math equations appear as plain text fragments

## Files Involved

| File                 | Line    | Issue                                       |
| -------------------- | ------- | ------------------------------------------- |
| extraction_engine.rs | 386-394 | Y normalization for non-flipped PDFs        |
| text_grouping.rs     | 62-63   | Incorrect comment about Y=0 position        |
| reading_order.rs     | 96-105  | Sorts ascending Y (correct if Y normalized) |

## Test Coverage

Current quality tests (`quality_extraction.rs`) only test:

- Qwen.pdf (Type3 fonts, was flipped → now fixed)
- 001-BEYONG-TRANFORMER... (worked by accident)
- AgenticPlatformReference... (mostly worked)

None of the new arXiv papers or Apple guide are tested.
