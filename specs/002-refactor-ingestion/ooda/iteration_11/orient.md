# OODA-11: Orient

## Analysis

### SRP Assessment
The processing status summary is a self-contained UI section that:
- Shows pipeline processing state
- Has its own click behavior (opens dialog)
- Has own visibility condition (pipelineStatus.running_tasks > 0 || queued > 0)
- Displays computed information from documents + pipelineStatus

**Verdict**: Clear SRP candidate - processing status display is distinct responsibility.

### Pattern Consistency
Similar to previously extracted components:
- Receives data via props
- Handles user interaction (click to open dialog)
- Can be tested in isolation

### Integration Points
1. **Parent**: DocumentManager renders conditionally
2. **Data**: Pipeline status from query + filtered documents
3. **Action**: Callback to open pipeline dialog

### Type Considerations
Need to handle `isProcessingStatus` function import - already exists in status-badge module.
