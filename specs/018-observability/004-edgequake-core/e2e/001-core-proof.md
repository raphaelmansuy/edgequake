# SPEC-018 — edgequake-core proof

**Status:** ✅ Proven (build)  
**Date:** 2026-06-05

## Evidence

```bash
cargo test -p edgequake-core --lib --quiet
```

Core uses `tracing` macros; request context is owned by API middleware (DIP).
