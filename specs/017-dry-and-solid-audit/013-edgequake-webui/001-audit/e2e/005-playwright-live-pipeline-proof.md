# Full pipeline via WebUI

**Status:** ✅ Sync + async proven  
**Date:** 2026-06-04 18:40 UTC

| Path | API | UI proof |
|------|-----|----------|
| Sync text | `POST /documents` sync | `06-sync-pipeline-completed.png` |
| Async text | `POST /documents` async + poll | `07-async-pipeline-completed.png` |

## Assertions (Playwright)

- `chunk_count > 0`, `entity_count > 0`, status `processed|completed`
- Documents page shows title + **Completed** badge

## Gaps (honest)

- PDF multipart UI not re-run in this webui spec (see `e2e/spec017-api-query-documents.spec.ts` under API audit).
