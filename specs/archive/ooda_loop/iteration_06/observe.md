# Iteration 06 - OBSERVE Phase

## Objective

Create comprehensive E2E tests for error handling and document reprocessing

## Current Test Coverage

### Existing E2E Tests

- `document-reprocess.spec.ts` - Basic reprocess tests
- `rebuild-operations.spec.ts` - Rebuild button tests

### Missing Test Coverage

1. Error message popover functionality
2. Copy to clipboard verification
3. Retry from error popover
4. Reprocess failed documents button
5. Error state persistence after page reload
6. Multiple failed document handling

## Components to Test

### ErrorMessagePopover

- Popover opens on click
- Full error message displayed
- Copy button works
- Retry button triggers reprocess
- Popover closes after retry

### ReprocessFailedButton

- Button visible when failed count > 0
- Button hidden when failed count = 0
- Confirmation dialog appears
- Reprocessing starts on confirm
- Toast notification appears
- Documents list refreshes

## Test Environment Requirements

- Backend running with test documents
- At least one failed document for error tests
- Ollama or mock LLM for processing tests
