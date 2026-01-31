# Iteration 02 - Observe

## Observation Timestamp

**Date**: 2026-01-31T15:25 UTC

## Observed Behavior

After implementing iteration_01 fix, uploaded the same PDF again:

1. **POST `/api/v1/documents/pdf`** → 200 OK
2. **GET `/api/v1/documents/pdf/progress/{track_id}`** → **404 Not Found** ✗

## Backend Log Analysis

```
2026-01-31T15:20:41.634964Z WARN request{method=POST uri=/api/v1/documents/pdf version=HTTP/1.1}: 
edgequake_api::handlers::pdf_upload: Duplicate PDF upload detected: existing_id=b482f592-d71f-47ec-b270-762b0f7d02a8
```

The log shows "Duplicate PDF upload detected" - this is a different code path than the one fixed in iteration_01!

## Code Path Analysis

**File**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`

```rust
// Lines 402-428 - Duplicate detection returns EARLY
if let Some(existing) = pdf_storage
    .find_pdf_by_checksum(&workspace_id, &checksum)
    .await?
{
    warn!("Duplicate PDF upload detected: existing_id={}", existing.pdf_id);
    return Ok(Json(PdfUploadResponse {
        // ... returns with status: "duplicate"
    }));  // <-- RETURNS HERE, NEVER REACHES OODA-01 FIX AT LINE 465
}
```

## Root Cause

The OODA-01 fix at line 465 is only reached for **new** PDF uploads. For **duplicate** PDFs (same checksum), the function returns at line 426 before the progress initialization code.

## Evidence

| Metric | Expected | Actual |
|--------|----------|--------|
| Progress endpoint status | 200 OK | 404 Not Found |
| OODA-01 log message | Present | Absent |
| Duplicate log message | Absent | Present |
| Upload response status | "processing" | "duplicate" |

## Conclusion

Need to add OODA-01 fix to the duplicate detection code path as well.
