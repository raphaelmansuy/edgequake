# Orient - Iteration 09: Error Categorization Analysis

## Current Error Display Issues

1. All errors shown with same red style
2. Users can't distinguish transient vs permanent failures
3. No guidance on how to resolve errors
4. Technical jargon not user-friendly

## Error Categories Identified

### LLM Errors (Purple)

- Rate limit exceeded → Wait and retry
- API key issues → Check configuration
- Context too long → Split document

### Embedding Errors (Blue)

- Dimension mismatch → Check model config
- API errors → Check embedding provider

### Storage Errors (Orange)

- Database unavailable → Wait and retry
- Constraint violations → Data issue

### Pipeline Errors (Yellow)

- Parse failures → Check document format
- Invalid content → Verify document

### Network Errors (Cyan)

- Timeouts → Check connectivity
- Connection refused → Server may be down

## Implementation Strategy

1. Create error-categories.ts utility
2. Pattern matching for error categorization
3. Color-coded display by category
4. Suggestions for user action
5. Mark transient errors as "Retryable"
6. Expandable technical details

## Impact Assessment

- **High UX Value**: Users understand errors immediately
- **Actionable**: Clear suggestions for resolution
- **No backend changes**: Pure frontend enhancement
