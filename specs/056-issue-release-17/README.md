# SPEC-056 — Issue #300 (v0.17.0 Vision Ingest Upload Stuck)

**Issue:** [raphaelmansuy/edgequake#300](https://github.com/raphaelmansuy/edgequake/issues/300)  
**Release under test:** `v0.17.0` published GHCR images  
**Provider stack:** Mistral (`mistral-small-latest` + `mistral-embed`)  
**Date:** 2026-07-16

## Verdict

**Partially reproduced.** Backend PDF vision ingest with Mistral **works** (documents reach `completed`). The user-visible “stuck on loading” symptom is explained by a **client vs server `track_id` split**: the WebUI / progress API listen on the client-supplied `track_id`, while pipeline progress is written under the server `pdf-<uuid>` task id. Progress for the client id stays at `0%` / “Waiting for Upload” forever.

## Documents

| File | Purpose |
|------|---------|
| [001-5-whys.md](./001-5-whys.md) | First-principles 5 Whys |
| [002-reproduction.md](./002-reproduction.md) | Exact repro from published Docker images + Mistral |
| [003-root-cause.md](./003-root-cause.md) | Code-level root cause + fix sketch |
| [artifacts/](./artifacts/) | Health, upload responses, dual progress polls |

## Quick evidence

```text
CLIENT track  ui_track_*     → overall 0.0   phases all pending ("Waiting for Upload")
SERVER track  pdf-71ab2f81-* → overall >0    pdf_conversion active/complete
Document list                → status extracting/completed under server track_id
```

Images used:

- `ghcr.io/raphaelmansuy/edgequake:0.17.0`
- `ghcr.io/raphaelmansuy/edgequake-frontend:0.17.0`
- `ghcr.io/raphaelmansuy/edgequake-postgres:0.17.0`
