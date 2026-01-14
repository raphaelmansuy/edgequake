# OODA Loop Iteration 62 - Orient

## Strategic Analysis

### Priority Assessment

| Requirement | Priority | Complexity | Risk |
|-------------|----------|------------|------|
| REQ-25 (Chunk Compat) | CRITICAL | High | Data loss if chunks fail |
| REQ-22 (Model Display) | Medium | Low | UX enhancement |
| REQ-23 (Close Button) | Medium | Low | UX clarity |
| REQ-24 (Rebuild Debug) | High | Medium | Feature broken |
| REQ-28 (OpenAI Key) | High | Low | DX blocker |
| REQ-26 (Stop Extraction) | Low | High | Nice-to-have |
| REQ-27 (Scroll Audit) | Low | Medium | Verified OK in OODA 283 |

### Dependency Chain

```
REQ-28 (Makefile) ─────────────────────────── Independent
REQ-22 (Model Display) ────────────────────── Independent
REQ-23 (Close Button) ─────────────────────── Independent
REQ-24 (Rebuild Debug) ────────────────────── Independent
REQ-25 (Chunk Compat) ─┬─ Depends on models.toml context_length
                       └─ Response type change needed
REQ-26 (Stop Extraction) ──────────────────── Requires task cancellation API
REQ-27 (Scroll Audit) ─────────────────────── Already done
```

### Root Cause Analysis

**REQ-24 (Documents Not Processing)**

Hypothesis 1: Workspace ID mismatch
- Documents stored with `workspace_id: "default"` or old workspace ID
- Reprocess filter skips documents not matching current workspace

Hypothesis 2: Content not found
- `{doc_id}-content` key missing or empty
- Skip happens silently

Hypothesis 3: Status filtering
- Documents in "processing" state get skipped
- include_completed: true may not cover all cases

**Solution**: Add comprehensive debug logging at each skip point.

### Risk Mitigation

**REQ-25 Risk**: If chunks exceed embedding model limit, they fail silently during embedding.

**Mitigation**:
1. Add `model_context_length` to rebuild response
2. Add `compatibility_warning` field for UI display
3. Log warning but don't block operation (for flexibility)
4. Future: Add strict mode option

## Execution Plan

1. ✅ Implement REQ-22 (model display) - Low risk, high value
2. ✅ Implement REQ-23 (close button) - Quick win
3. ✅ Add REQ-24 debug logging - Essential for diagnosis
4. ✅ Implement REQ-25 validation - Critical invariant
5. ✅ Fix REQ-28 (Makefile) - Developer experience
6. ⏸ REQ-26 deferred - Requires larger architecture change
7. ✅ REQ-27 already verified in OODA 283
