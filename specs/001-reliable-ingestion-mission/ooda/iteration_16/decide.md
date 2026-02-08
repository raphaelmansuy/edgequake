# OODA Iteration 16 - Decide Phase

## Decision: Implement Strict Pipeline Mode

### Selected Fix: OODA-16-FIX-001

**Approach**: Modify `get_workspace_pipeline` to return `Result<Arc<Pipeline>, String>` and fail when in strict mode.

### Implementation Plan

#### Step 1: Add `get_workspace_pipeline_strict` Method

```rust
/// OODA-16: Strict variant that returns error instead of falling back
async fn get_workspace_pipeline_strict(
    &self,
    workspace_id: Option<&str>
) -> Result<Arc<Pipeline>, String> {
    // Same logic as get_workspace_pipeline but returns Err on failure
}
```

#### Step 2: Update `process_document_task`

Change from:

```rust
let pipeline = self.get_workspace_pipeline(workspace_id).await;
```

To:

```rust
let pipeline = if self.strict_workspace_mode {
    self.get_workspace_pipeline_strict(workspace_id).await
        .map_err(|e| format!("Workspace pipeline error: {}", e))?
} else {
    self.get_workspace_pipeline(workspace_id).await
};
```

#### Step 3: Update Error Handling in Task Processing

When pipeline creation fails in strict mode, the task should:

1. Update document status to `Failed`
2. Store clear error message about missing provider configuration
3. NOT fall back to default pipeline

### Expected Outcomes

| Scenario                     | Before                           | After                                    |
| ---------------------------- | -------------------------------- | ---------------------------------------- |
| OpenAI key missing (strict)  | Process with Ollama              | Task FAILS with "OPENAI_API_KEY not set" |
| Workspace not found (strict) | Process with default             | Task FAILS with "Workspace not found"    |
| Valid config                 | Process with workspace providers | Same                                     |

### Test Plan

1. **Unit Test**: Create processor with strict mode, call without valid workspace → expect error
2. **Integration Test**: Upload document to workspace with OpenAI config but no API key → expect Failed status
3. **E2E Test**: Re-run parallel ingestion test after fix

### Rollback Plan

If the fix causes issues:

1. Revert to non-strict behavior for pipeline (keep storage strict)
2. Add configuration flag to control pipeline strictness separately

### Files to Modify

1. `edgequake-api/src/processor.rs` - Add strict pipeline method + update process_document_task

### Acceptance Criteria

- [ ] `get_workspace_pipeline_strict` implemented
- [ ] `process_document_task` uses strict method when `strict_workspace_mode=true`
- [ ] Failed tasks have clear error messages
- [ ] Existing tests pass
- [ ] Parallel ingestion test passes with Ollama running
