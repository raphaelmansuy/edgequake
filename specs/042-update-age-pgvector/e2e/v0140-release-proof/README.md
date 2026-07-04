# v0.14.0 Release E2E Proof

**Date**: 2026-07-04  
**Version**: v0.14.0  
**Registry**: `ghcr.io/raphaelmansuy/edgequake-postgres`

## Summary

Full E2E verification of the v0.14.0 published Docker images across all three PostgreSQL tiers (PG16, PG17, PG18). Each profile passed 8 checks covering image pull, container startup, extension verification, version gates, vector operations (HNSW + halfvec), Apache AGE graph operations (MERGE/MATCH/DROP), ingestion-ready schema simulation, and PG-version-specific features.

## Results Matrix

| Check | PG16 | PG17 | PG18 |
|-------|------|------|------|
| Image pull | PASS | PASS | PASS |
| Container startup | PASS (4s) | PASS (5s) | PASS (5s) |
| Extensions (vector + AGE) | PASS (age=1.6.0, vector=0.8.3) | PASS (age=1.7.0, vector=0.8.3) | PASS (age=1.7.0, vector=0.8.3) |
| PG major version | PASS (16) | PASS (17) | PASS (18) |
| Vector HNSW ANN (384-d, 50 rows) | PASS | PASS | PASS |
| Halfvec HNSW | PASS | PASS | PASS |
| AGE graph (MERGE+MATCH+DROP) | PASS (2 nodes, 1 edge) | PASS (2 nodes, 1 edge) | PASS (2 nodes, 1 edge) |
| Ingestion schema | PASS | PASS | PASS |
| Version-specific | uuidv7() absent (expected) | major=17 confirmed | uuidv7() = 019f2adb-... |

## Extension Versions Per Tier

| Tier | pgvector | Apache AGE | PostgreSQL |
|------|----------|-----------|------------|
| PG16 | 0.8.3 | 1.6.0 | 16.x |
| PG17 | 0.8.3 | 1.7.0 | 17.x |
| PG18 | 0.8.3 | 1.7.0 | 18.x |

## Test Coverage

1. **Image Pull** — Confirms the GHCR image is publicly accessible
2. **Container Startup** — PostgreSQL boots and accepts connections within 5s
3. **Extension Verification** — All required extensions install: `vector`, `pg_trgm`, `btree_gin`, `uuid-ossp`, `age`
4. **PG Major Version** — `server_version_num / 10000` matches expected tier
5. **Vector HNSW ANN** — 384-dimensional vectors, 50 rows, filtered tenant query with `hnsw.iterative_scan = strict_order`
6. **Halfvec HNSW** — Validates SPEC-042 halfvec dimension guard (3-dim test)
7. **AGE Graph** — Full Cypher lifecycle: `create_graph`, `MERGE` nodes + edges, `MATCH` count verification, `drop_graph`
8. **Ingestion Schema** — Simulates EdgeQuake backend schema: `kv_store` (documents, chunks), `vector_store` (embeddings), GIN indexes
9. **Version-Specific** — PG18 `uuidv7()`, PG16 absence check, PG17 tier confirmation

## How to Reproduce

```bash
./specs/042-update-age-pgvector/e2e/run_v0140_release_e2e.sh all
```

Individual profiles:

```bash
./specs/042-update-age-pgvector/e2e/run_v0140_release_e2e.sh pg16
./specs/042-update-age-pgvector/e2e/run_v0140_release_e2e.sh pg17
./specs/042-update-age-pgvector/e2e/run_v0140_release_e2e.sh pg18
```

## Per-Profile Reports

- [pg16-report.txt](pg16-report.txt)
- [pg17-report.txt](pg17-report.txt)
- [pg18-report.txt](pg18-report.txt)
