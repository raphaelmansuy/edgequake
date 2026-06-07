# SPEC-006 E2E Proof 005 — Upload Limit Alignment

**Covers:** TR-006-019, V-006-012  
**Test:** `resource_safety_upload_limit_ssot`

## Assertion

| Layer | Limit |
|-------|-------|
| `edgequake_core::MAX_UPLOAD_BYTES` | 50 MiB |
| Axum `DefaultBodyLimit` in `server.rs` | `MAX_UPLOAD_BYTES` (same) |
| `AppConfig::max_document_size` | 50 MiB |

**Drift fixed:** server was 100 MiB, config 50 MiB.

## Run

```bash
cargo test -p edgequake-api resource_safety_upload_limit_ssot
```

## Code is law

- `edgequake-core/src/resource/budget.rs::MAX_UPLOAD_BYTES`
- `edgequake-api/src/server.rs`
