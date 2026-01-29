# OODA Iteration 07 - Observe

**Date**: 2025-01-XX
**Focus**: Entity Normalization Deep-Dive

## 🔍 Observations

### 1. Normalization Code Analysis

**File**: `edgequake-pipeline/src/prompts/normalizer.rs` (~170 lines)

Key function `normalize_entity_name()` applies:

1. Trim whitespace
2. Remove prefixes (The, A, An)
3. Remove possessive suffixes ('s)
4. Title case each word
5. Join with underscores
6. UPPERCASE final result

Examples:

- "John Doe" → "JOHN_DOE"
- "The Company" → "COMPANY"
- "Sarah's Research" → "SARAH_RESEARCH"

### 2. Merger Implementation

**File**: `edgequake-pipeline/src/merger.rs` (~863 lines)

Implements FEAT0006 (Entity Deduplication):

- Normalizes entity names before graph lookup
- Merges descriptions via LLM summarization
- Accumulates source_id for lineage
- Handles max 10 sources, 4096 char descriptions

### 3. Description Merging Strategy

When same entity appears in multiple chunks:

1. Existing description + new description
2. LLM summarizes into unified description
3. Falls back to concatenation if LLM fails

### 4. Statistics from Production

From production example:

- Real LLM: 20 entities → 12 unique nodes (40% deduplication)
- Mock LLM: 9 entities → 6 unique nodes (33% deduplication)

This shows normalization achieves significant deduplication.
