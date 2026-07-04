# SPEC-042 — DRY/SOLID Improvements (Phase F+)

**Date:** 2026-07-04
**Method:** First Principles applied to infrastructure and test code

---

## 1. Unified Dockerfile (DRY)

**Problem:** Three Dockerfiles (`Dockerfile.postgres`, `.pg17`, `.pg18`) share 95% identical code. Only 5 parameters differ per PG major.

**First Principle:** A single source of truth for build steps eliminates drift between tiers.

**Solution:** `Dockerfile.postgres.unified` accepts build args:

| ARG | Default | Source |
| --- | ------- | ------ |
| `PG_MAJOR` | 18 | `extension-pins.sh` → `EQ_POSTGRES_MAJOR` |
| `PGVECTOR_VERSION` | v0.8.3 | `extension-pins.sh` → `EQ_PGVECTOR_VERSION` |
| `AGE_GIT_REF` | PG18/v1.7.0-rc0 | `extension-pins.sh` → `EQ_AGE_GIT_REF` |
| `AGE_EXPECTED_VERSION` | 1.7.0 | `extension-pins.sh` → `EQ_AGE_MIN` |

**Makefile target:**

```bash
EQ_POSTGRES_PROFILE=pg16 make postgres-image-build-unified
EQ_POSTGRES_PROFILE=pg17 make postgres-image-build-unified
EQ_POSTGRES_PROFILE=pg18 make postgres-image-build-unified
```

**Backward compatibility:** Original per-major Dockerfiles retained for CI/existing workflows.

---

## 2. E2E UI-only gate mock helper (DRY)

**Problem:** `mockBackendForUiOnly()` was duplicated in `spec037-query-full-chunk.spec.ts` and `spec037-query-settings-scroll.spec.ts`.

**First Principle:** Mock setup for the "no backend" CI gate is a cross-cutting concern.

**Solution:** Extracted to `e2e/helpers/mock-backend.ts`. Any UI-only spec that needs backend mocks imports from one place.

---

## 3. Battle test results (2026-07-04)

All tests executed locally against fresh Docker images.

### Extension pin SSOT

```
✓ pg16 ↔ Dockerfile.postgres    (vector 0.8.3, AGE 1.6.0)
✓ pg17 ↔ Dockerfile.postgres.pg17 (vector 0.8.3, AGE 1.7.0)
✓ pg18 ↔ Dockerfile.postgres.pg18 (vector 0.8.3, AGE 1.7.0)
```

### Version feature battle test (all profiles)

| Probe | PG16 | PG17 | PG18 |
| ----- | ---- | ---- | ---- |
| BT-PV-01 iterative scan GUCs | ✓ | ✓ | ✓ |
| BT-PV-02 filtered HNSW ANN | ✓ | ✓ | ✓ |
| BT-PV-03 halfvec type | ✓ | ✓ | ✓ |
| BT-PV-04 halfvec HNSW + filtered ANN | ✓ | ✓ | ✓ |
| BT-AGE-01 Cypher MERGE/MATCH | ✓ | ✓ | ✓ |
| BT-AGE-02 extversion gates | ✓ | ✓ | ✓ |
| BT-PG uuidv7 | N/A (absent) | N/A | ✓ |
| BT-M042/M043 bootstrap apply | ✓ | ✓ | ✓ |

### HNSW dimension guard (#275)

| Probe | PG16 | PG17 | PG18 |
| ----- | ---- | ---- | ---- |
| BT-275-01 vector(3072) HNSW rejected | ✓ | ✓ | ✓ |
| BT-275-02 M071 halfvec promotion | ✓ | ✓ | ✓ |
| BT-275-03 dim>4000 skip HNSW | ✓ | ✓ | ✓ |

### Phase E (PG17+ only)

| Probe | PG17 | PG18 |
| ----- | ---- | ---- |
| BT-PV-04 halfvec HNSW | ✓ | ✓ |
| E-02.7 AGE RLS isolation | ✓ | ✓ |
| E-03.5 uuidv7 version nibble | N/A | ✓ |
| E-04.6 AGE COPY loader | ✓ | ✓ |

### Rust tests

| Suite | Count | Status |
| ----- | ----- | ------ |
| AnnIndexPolicy unit tests | 3 | ✓ |
| Migration checksum proofs | 1 | ✓ |
| Migration bootstrap lib tests | 19 | ✓ |
| SPEC-022 Cypher param guards | 4 | ✓ |

---

## 4. SOLID analysis

| Principle | Current state | Improvement |
| --------- | ------------- | ----------- |
| **S** (SRP) | Each reconcile module (m042–m081) handles exactly one migration | ✓ Good |
| **O** (Open/Closed) | New PG features added via new phases (E-01 through E-04) without modifying existing migrations | ✓ Good |
| **L** (Liskov) | `GraphStorage` trait works identically across PG16/17/18 | ✓ Good |
| **I** (ISP) | `extension_version_at_least()` gates features without bloating core interfaces | ✓ Good |
| **D** (DIP) | `extension-pins.sh` → consumers source SSOT (Dockerfiles, Makefile, verify scripts) | ✓ Good; unified Dockerfile improves further |

---

## 5. Remaining DRY candidates (future)

| Area | Duplication | Effort | Impact |
| ---- | ----------- | ------ | ------ |
| Reconcile modules m048–m065 | Template pattern (marker + apply SQL) | Medium | Code volume |
| CI triple-publish | Dockerfile per PG in `release-docker.yml` | Low | Adopt unified Dockerfile in CI |
| Init SQL | `create_extension` repeated in battle tests + init.sql | Low | Extract to shared `.sql` partial |
