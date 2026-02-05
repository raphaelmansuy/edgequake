# OODA-15: Observe - Number Subsection Detection Test Coverage

## Current State

The `block_classifier.rs` has functions for detecting various subsection patterns:
- `is_roman_numeral_header` - "I. INTRODUCTION"
- `is_letter_subsection_item` - "A. Background"
- `is_number_section_header` - "1. Introduction"
- `is_number_subsection_item` - "2.1. Agentic Training"

## Current Test Coverage

From grep of `#[test]`:
- ✅ `test_bullet_detection`
- ✅ `test_numbered_list_detection`
- ✅ `test_roman_numeral_header`
- ✅ `test_block_classifier`
- ✅ `test_heading_level_classification` (OODA-14)

## Gap Identified

Missing tests for:
1. `is_letter_subsection_item` - "A. Background" pattern
2. `is_number_section_header` - "1. Introduction" pattern
3. `is_number_subsection_item` - "2.1. Subsection" pattern

## Evidence

These functions exist but lack dedicated test coverage:
- Line 301: `is_letter_subsection_item`
- Line 327: `is_number_section_header`
- Line 369: `is_number_subsection_item`
