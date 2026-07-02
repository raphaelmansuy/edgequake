# SPEC-039 — Fresh Docker Install: AGE Label Bootstrap

**Spec:** `039-fix-docker`  
**Date:** 2026-07-02  
**Status:** `FIXED` — `ensure_graph_labels()` in graph lifecycle (v0.13.1)  
**Method:** Code is law — all claims cross-referenced to live sources and Docker E2E evidence

---

## TL;DR

> Fresh `docker compose` installs created the AGE graph catalog entry but **not** the `Node` / `EDGE` label tables. SPEC-032/034 native batch SQL reads `{graph}."Node"` on first ingestion → `relation does not exist`. Mistral/Ollama extraction succeeded; graph persist and query failed.

**Root fix:** Eager `create_vlabel` + `create_elabel` during `pg_initialize()` after `create_graph()`.

---

## Documents

| File | Lens | Key question |
| ---- | ---- | ------------ |
| [001-five-whys.md](./001-five-whys.md) | 5 WHY | Why does fresh Docker E2E fail? |
| [002-first-principles.md](./002-first-principles.md) | First principles | What must exist before native graph SQL? |
| [003-code-is-law.md](./003-code-is-law.md) | Code is law | Which code paths assume `Node` exists? |
| [008-implementation-plan.md](./008-implementation-plan.md) | Fix plan | Implementation + edge cases |
| [009-cross-reference-matrix.md](./009-cross-reference-matrix.md) | Cross-ref | Evidence map |

## E2E proof

```bash
# After v0.13.1 images are available:
EDGEQUAKE_VERSION=0.13.1 MISTRAL_API_KEY=... ./specs/039-fix-docker/e2e/run_docker_fresh_install_proof.sh mistral
OLLAMA_MODEL=gemma4:e4b ./specs/039-fix-docker/e2e/run_docker_fresh_install_proof.sh ollama
```
