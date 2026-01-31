# Iteration 02 - Orient

## Context Analysis

### Code Structure

The `upload_pdf_document()` function has two exit paths:

1. **Duplicate Path** (lines 402-428): Returns early if PDF checksum matches existing file
2. **New Upload Path** (lines 430-495): Creates new PDF and processing task

The iteration_01 fix was added to path #2, but path #1 also needs the fix because:

- Frontend sends `track_id` in upload request
- Frontend polls `/pdf/progress/{track_id}` regardless of response status
- Even for duplicates, the progress entry must exist to avoid 404

### Root Cause Mapping

```
upload_pdf_document()
├── Check duplicates (line 402)
│   └── If duplicate → return "duplicate" status (NO OODA-01 FIX) ← BUG
└── New upload path
    ├── Store PDF
    ├── Create task
    ├── OODA-01: Initialize progress (line 465) ← FIXED IN ITERATION_01
    └── Return "processing" status
```

## Mental Model Update

The original assumption was that duplicates don't need progress tracking because:

- They're already processed
- No background task is created

However, the frontend:

- Generates a unique `track_id` per upload attempt
- Immediately polls that `track_id` for progress
- Does not check response status before polling

This means progress must be initialized for ALL upload responses, not just new uploads.

## Patterns Identified

1. **Early Return Pattern**: Code returns early from function without executing later setup code
2. **Frontend-Backend Contract**: Frontend assumes progress exists for any track_id it generates
3. **Defensive Initialization**: Progress should be initialized before ANY response is sent

## Strategy

Add progress initialization to the duplicate detection path, right before returning the response.

This is a defensive approach that:

- Prevents 404 errors for duplicate uploads
- Maintains frontend-backend contract
- Is idempotent (harmless if called multiple times)
