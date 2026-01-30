# OODA Iteration 08 - Observe

**Date**: 2025-01-XX
**Focus**: Gleaning Deep-Dive Documentation

## 🔍 Observations

### 1. Gleaning Implementation

**File**: `edgequake-pipeline/src/extractor.rs` (lines 1070-1250)

Key components:

- `GleaningConfig`: max_gleaning (default: 1), always_glean (default: false)
- `GleaningExtractor`: Wraps base extractor with multi-pass capability
- `build_gleaning_prompt()`: Creates re-extraction prompt
- `merge_results()`: Combines passes, keeps longer descriptions

### 2. Why Gleaning?

LLMs miss entities due to:

- Attention limits on long texts
- Implicit entities ("the company" → "Apple")
- Context overload with many entities

Research finding: 1-2 iterations improve recall by 15-25%

### 3. Gleaning Prompt Structure

```
MANY entities and relationships were missed...
## Already Identified Entities: [list]
## Instructions: Focus on implicit, contextual entities
## Text to Re-Analyze: [text]
```

### 4. Merge Strategy

When gleaning finds existing entity:

- Compare descriptions
- Keep the longer (richer) description
- Avoid duplicates by name matching (case-insensitive)
