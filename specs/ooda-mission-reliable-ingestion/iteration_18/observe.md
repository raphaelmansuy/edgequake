# OODA-18 Observe: Post-Constraint-Fix Upload Test

## Context
- OODA-17 fixed the `tasks_valid_status` constraint mismatch
- Constraint now accepts: `pending`, `processing`, `indexed`, `failed`, `cancelled`
- Need to verify full pipeline works end-to-end

## Current State (2025-02-08)

### Documents Page Snapshot
- Total documents: 21
- Status distribution:
  - Completed: ~18 documents
  - Failed: 3 documents (Fiscalité PDF with entity extraction errors)

### Key Observations
1. ✅ Constraint errors eliminated from logs
2. ✅ Documents successfully transitioning to "Completed" status
3. ⚠️ 3 failed documents with "Entity extraction" errors (unrelated to constraint)

## Test Plan
1. Upload a new PDF document
2. Monitor task status transitions
3. Verify final "Completed" status
4. Check backend logs for any errors

## PDF to Upload
Using: `zz_test_docs/lighrag_2410.05779v3.pdf` (LightRAG paper)
- Well-structured academic PDF
- Should extract ~100+ entities
- Good test of full pipeline

---
*Observed: 2025-02-08*
