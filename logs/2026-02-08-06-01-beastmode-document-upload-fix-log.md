# Task Log: Document Upload & Pipeline Fixes

**Date**: 2026-02-08 05:01 - 06:01 UTC
**Mode**: beastmode
**Duration**: ~60 minutes

## Summary

Successfully fixed multiple issues in the EdgeQuake document upload and entity extraction pipeline, enabling complete PDF → Markdown → Entity Extraction → Knowledge Graph workflow.

---

## Actions Performed

1. **Added DELETE /documents endpoint** - Fixed 405 Method Not Allowed
2. **Implemented stuck document detection** - Documents stuck >1hr at 100% progress
3. **Added PDF documents cleanup** - With `#[cfg(feature = "postgres")]` guard
4. **Updated workspace LLM settings** - Changed from OpenAI (quota exceeded) to Ollama
5. **Increased chunk extraction timeout** - From 60s to 180s for slow local models
6. **Fixed database constraint** - `tasks_valid_status` had wrong values (`running`/`completed` instead of `processing`/`indexed`)
7. **Switched to faster model** - From `gemma3:latest` (12B, ~90s/call) to `llama3.2:latest` (3B, ~10s/call)
8. **Verified full pipeline** - PDF upload → extraction → KG population

---

## Decisions Made

- Used Ollama local models instead of OpenAI due to API quota limits
- Selected `llama3.2:latest` for entity extraction (9x faster than gemma3)
- Implemented adaptive max_tokens retry logic to handle JSON truncation
- Added PDF cleanup to delete endpoint to prevent duplicate detection blocking re-uploads

---

## Files Modified

| File                                            | Change                                                                  |
| ----------------------------------------------- | ----------------------------------------------------------------------- |
| `edgequake-api/src/handlers/documents_types.rs` | Added `DeleteAllDocumentsResponse` with `total_pdfs_deleted` field      |
| `edgequake-api/src/handlers/documents.rs`       | Added `delete_all_documents` handler with stuck detection + PDF cleanup |
| `edgequake-api/src/routes.rs`                   | Added DELETE /documents route                                           |
| `edgequake-pipeline/src/pipeline.rs`            | Increased `default_chunk_timeout` from 60 to 180 seconds                |

---

## Database Changes

```sql
ALTER TABLE tasks DROP CONSTRAINT tasks_valid_status;
ALTER TABLE tasks ADD CONSTRAINT tasks_valid_status
  CHECK (status IN ('pending', 'processing', 'indexed', 'failed', 'cancelled'));
```

---

## Test Results

| Test                | Result                                       |
| ------------------- | -------------------------------------------- |
| PDF Upload          | ✅ national-capitals.pdf (6 pages)           |
| Markdown Conversion | ✅ 8,630 characters                          |
| Entity Extraction   | ✅ 175 entities extracted                    |
| Relationships       | ✅ 29 relationships created                  |
| Knowledge Graph     | ✅ 247 total entities, 7 connections visible |
| Document Status     | ✅ Completed                                 |

---

## Next Steps

1. Consider adding automatic model fallback when extraction times out
2. Add real-time progress WebSocket notifications for extraction stages
3. Investigate PDF-document linking foreign key constraint issue
4. Add retry queue for failed chunks to enable background re-processing

---

## Lessons Learned

- Local Ollama models can be very slow (3+ minutes per LLM call for large models)
- JSON truncation from LLMs is common - need robust retry with increased max_tokens
- Database constraints must match actual application status values
- Model selection significantly impacts extraction speed (gemma3 12B vs llama3.2 3B = 9x difference)
