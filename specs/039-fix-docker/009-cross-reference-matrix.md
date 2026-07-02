# SPEC-039 — Cross-Reference Matrix

| ID | Claim | Evidence | Status |
| -- | ----- | -------- | ------ |
| RC-1 | Fresh Docker ingest fails on empty graph | E2E logs 2026-07-02, `001-five-whys.md` | CONFIRMED |
| RC-2 | Root cause: missing `Node`/`EDGE` label tables | `psql \dt eq_eq_default_graph.*` pre-fix | CONFIRMED |
| FIX-1 | `ensure_graph_labels` in `create_graph` | `graph_lifecycle.rs` | IMPLEMENTED |
| E2E-1 | Manual `create_vlabel` unblocks Mistral path | Upload completed + query answer | VERIFIED |
| DOC-1 | README Mistral Docker command | `README.md` Quick Start | UPDATED |
| DOC-2 | README Ollama gemma4:e4b command | `README.md` Quick Start | UPDATED |
| REL-1 | v0.13.1 patch release | `VERSION`, `CHANGELOG.md` | READY |
| E2E-2 | Ollama `gemma4:e4b` fresh Docker proof | `run_docker_fresh_install_proof.sh ollama` | VERIFIED 2026-07-02 |
| E2E-3 | Mistral fresh Docker proof | `run_docker_fresh_install_proof.sh mistral` | VERIFIED 2026-07-02 |

## Traceability

| Spec | Relationship |
| ---- | ------------ |
| SPEC-032 | `pg_get_nodes_batch` native SQL introduced read-before-write |
| SPEC-034 | Native write path also requires label tables |
| SPEC-038 | Docker E2E testing surfaced bug during v0.13.0 validation |
| Migration 013 | `create_age_graph_safe` — graph only, no labels |
