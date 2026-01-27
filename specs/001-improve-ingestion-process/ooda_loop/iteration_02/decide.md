# Iteration 02: Decide

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Prioritized Action Plan

### Decision 1: Add Processing Stage Status Updates

**Action**: Modify `process_text_insert()` in processor.rs to update document status at each stage.

**Locations**:

1. Before `pipeline.process()` → status: "chunking"
2. After chunk storage → status: "extracting"
3. After extraction complete → status: "embedding"
4. Before graph storage → status: "indexing"

**Existing status updates to keep**:

- Initial: "processing" (line 603)
- On error: "failed" (lines 613, 654)
- Final: "completed" via `update_document_status_with_stats()`

### Decision 2: Add Stage Timestamps (Optional Enhancement)

Add timestamps for each stage to enable ETA calculation in future:

```rust
updated.insert("stage_started_at".to_string(), json!(Utc::now().to_rfc3339()));
```

**Defer to iteration 03** - focus on core functionality first.

### Decision 3: Keep Backward Compatibility

Old clients that don't recognize new status values will:

- Fall back to treating unknown status as "processing"
- This is handled by `normalizeStatus()` in status-badge.tsx

---

## Changes for This Iteration

| #   | File         | Change                                            | Lines |
| --- | ------------ | ------------------------------------------------- | ----- |
| 1   | processor.rs | Update status to "chunking" before processing     | ~598  |
| 2   | processor.rs | Update status to "extracting" after chunks stored | ~655  |
| 3   | processor.rs | Update status to "embedding" after extraction     | ~720  |
| 4   | processor.rs | Update status to "indexing" before graph storage  | ~890  |

---

## Acceptance Criteria

- [ ] Document shows "Chunking" status during chunking phase
- [ ] Document shows "Extracting" status during LLM extraction
- [ ] Document shows "Embedding" status during embedding generation
- [ ] Document shows "Indexing" status during graph/vector storage
- [ ] Error status still shows with error message
- [ ] Completed status works as before

---

## Next Step

Proceed to **Act** phase to implement the changes.
