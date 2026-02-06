# OODA Iteration 39 — Observe

## Target: False `###` Headers in LightRAG Output

### Symptoms

The LightRAG PDF conversion produces 4 false `###` (H3) headers that are actually body text fragments:

| Line | Text                                                                  | Issue                                           |
| ---- | --------------------------------------------------------------------- | ----------------------------------------------- |
| 241  | `### explorations of related entities through low-level retrieval...` | Body text sentence fragment                     |
| 243  | `### 4.5. Figure 2: Comparison of Cost in Terms of To`                | Caption fragment starting with "4.5."           |
| 247  | `### tRAG on the Legal Dataset.`                                      | Word fragment ("LightRAG" split across columns) |
| 259  | `### to handling data changes in dynamic environments.`               | Body text fragment                              |

Real section headers correctly use `####` (H4, level 4):

- `#### 1. INTRODUCTION`
- `#### 2. RETRIEVAL-AUGMENTED GENERATION`
- `#### 3. THE LIGHTRAG ARCHITECTURE`
- `#### 4. EVALUATION`

### Root Cause Discovery

The `classify_blocks()` function in `pdfium_backend.rs` (line 420-427) assigns header levels 1-4 based on font size ratio to body:

```rust
let level = if size_ratio >= 1.8 {
    1 // h1: very large
} else if size_ratio >= 1.5 {
    2 // h2: large
} else if size_ratio >= 1.3 {
    3 // h3: medium → CAUSES FALSE ### HEADERS
} else {
    4 // h4: slightly larger → CAUSES #### FOR REAL SECTIONS
};
```

The downstream processors (StyleDetection, HeaderDetection, SectionPattern) have a guard:

```rust
if block.level.is_some() { continue; }
```

This means blocks classified by the backend are NEVER re-evaluated by the more sophisticated downstream processors that have:

- Prose indicator detection
- Caption filtering
- Sentence boundary detection
- Content-length guards

### Conflict Analysis

The pdfium backend uses **primitive font-size-only classification** with a low 1.2x threshold. The downstream processors use **multi-signal classification** (font size + weight + content analysis + pattern matching) with a conservative 1.4x threshold.

Because the backend runs first and the downstream processors skip blocks with levels already set, the primitive classification wins over the sophisticated classification for ALL blocks that happen to have slightly larger fonts.

### Output Stats

- LightRAG: 59,090 bytes, 239 blocks
- False headers: 4 (all level 3 = `###`)
- Real headers: 4 (all level 4 = `####`, should be level 2 = `##`)
