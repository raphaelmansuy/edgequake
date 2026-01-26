# OODA-44: Orient + Decide

## Analysis

Title edge case tests:
1. Unicode/emoji titles
2. Very long titles (1000+ chars)
3. Titles with special chars (quotes, slashes, etc.)

## Action Plan

Add 2 tests:
1. `test_document_with_unicode_title` - Unicode/emoji handling
2. `test_document_with_long_title` - Long title handling

## Success Criteria

- Tests pass
- Total deletion tests: 62
