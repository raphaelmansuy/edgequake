# Iteration 23: Processing Status Summary Card - Orient

## Analysis

### Enhancement Approach
Add a compact, clickable status bar that appears when documents are being processed.

### Display Conditions
Show when:
- `pipelineStatus.running_tasks > 0` (actively processing)
- `pipelineStatus.queued_tasks > 0` (waiting in queue)

### Information Shown
- Spinning loader icon
- Count of processing documents
- Count of queued documents (if any)
- Count of completed documents
- "Click for details" hint

### Visual Design
- Blue background (processing color)
- Compact height (py-2)
- Full width below filters
- Clickable to open pipeline dialog

### Interaction
- Click opens PipelineStatusDialog
- Keyboard accessible (Enter key)
- Hover state for clickability
