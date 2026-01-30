# Iteration 08 - ORIENT Phase

## Analysis

### Pipeline Status Dialog - Current Features

✅ Progress bar with percentage
✅ Statistics grid (Pending, Processing, Completed, Failed)
✅ Activity log with messages
✅ Cancel functionality with confirmation
✅ Real-time polling (every 2s)
✅ Batch progress tracking

### What's Missing

1. Individual document status in the dialog
2. ETA based on processing rate
3. Current processing stage per document

### Gap Priority

| Gap                 | Impact                                  | Effort |
| ------------------- | --------------------------------------- | ------ |
| ETA display         | High - users want to know when complete | Low    |
| Per-document status | Medium - already in main table          | High   |
| Processing rate     | Medium - nice to have                   | Low    |

## Decision

Focus on adding **ETA calculation** to the pipeline dialog:

- Track processing rate (docs/minute)
- Calculate remaining time
- Display in progress section

This provides high value with low effort.
