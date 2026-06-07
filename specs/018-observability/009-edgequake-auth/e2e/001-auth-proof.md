# SPEC-018 — edgequake-auth proof

**Status:** ✅ Proven (build)  
**Date:** 2026-06-05

```bash
cargo test -p edgequake-auth --lib --quiet
```

Auth failures surface via API `ApiError` logging with `request_id` (SPEC-018 API layer).
