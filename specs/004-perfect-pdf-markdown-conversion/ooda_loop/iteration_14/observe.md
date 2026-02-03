# OODA-14: Observe

## Current State (Post OODA-13)
- **Text Preservation**: 84.7%
- **Structure Fidelity**: 81.0%
- **Overall Quality**: 82.8%
- **Target**: 95%+
- **Gap**: 12.2 percentage points

## Observations

### 1. Lowest Performing Documents
- `agent_2510.09244v1`: 79.2% (lowest)
- `one_tool_2512.20957v2`: 80.2%
- `AlphaEvolve`: 81.0% (Structure 76.2% lowest)

### 2. Numbered List Spacing Issue
When analyzing `agent_2510.09244v1`, discovered:
- Our output: `1.Explore` (no space after period)
- Gold file: `1. Explore` (proper markdown format)

The `number_regex` in `ListDetectionProcessor` was:
```regex
r"^\d+[\.)]\s+"
```
This requires a space after the marker, so "1.Text" without space was NOT detected as a list item.

### 3. Section Header Confusion Risk
When making space optional in the regex, section headers like "1.1 Task A" 
could be incorrectly matched as numbered lists.

### 4. Rust Regex Limitation
Attempted to use negative lookahead `(?!\d)` to exclude "1.1" patterns.
**FAILED**: Rust's regex crate does NOT support lookahead/lookbehind.

## Key Data Points

```
Goyal document regression when space made optional:
- Before: 85.7%
- After: 79.2% (-6.5%)
- Cause: "1.1 Task A" headers converted to list items
```

## Files Analyzed
- `crates/edgequake-pdf/src/processors/structure_detection.rs`
- `crates/edgequake-pdf/src/renderers/markdown.rs`
- Gold files in `test-data/gold/`
