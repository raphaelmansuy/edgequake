# OODA Loop Iteration 62 - Decide

## Decision Summary

### Decisions Made

1. **REQ-22 Implementation**: Display model inline with tokens/second rather than in a separate badge to avoid UI clutter.

2. **REQ-23 Button Layout**: Use side-by-side buttons (Close | Cancel) rather than stacked for better UX.

3. **REQ-24 Logging Strategy**: Use HashMap for skip reasons to provide structured debugging info in logs.

4. **REQ-25 Validation Approach**:

   - Warn but don't block incompatible model changes (flexibility for advanced users)
   - Use `context_length` field from models.toml (already exists for all embedding models)
   - Default to 8192 if model not found in config (safe fallback)

5. **REQ-28 Key Forwarding**: Forward key from environment rather than requiring explicit export in Makefile.

### Deferred Items

- **REQ-26 (Stop Extraction)**: Requires task cancellation API, deferred to next iteration
- **REQ-27 (Scroll Audit)**: Already verified complete in OODA 283

### Next Steps

1. Build and test the full stack
2. Verify model display in query page
3. Verify rebuild dialog behavior
4. Test rebuild embeddings with logging
5. Document in OODA summary

## Quality Checklist

- [x] TypeScript types match Rust response
- [x] Cargo check passes
- [x] OODA documentation complete
- [ ] E2E test verification pending
- [ ] Commit pending
