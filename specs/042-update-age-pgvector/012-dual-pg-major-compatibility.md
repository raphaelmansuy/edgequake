# SPEC-042 — Multi-PostgreSQL Major Compatibility (PG16 + PG17 + PG18)

**Date:** 2026-07-03  
**Status:** **DECISION — triple-track supported**  
**Question:** Can EdgeQuake support PG16, PG17, and PG18 concurrently?

**Answer:** **Yes** — three **infrastructure profiles**, one **application binary**.

---

## Decision summary

| Tier | PostgreSQL | AGE | pgvector | Role |
| ---- | ---------- | --- | -------- | ---- |
| **Legacy supported** | 16 | 1.6.0 | 0.8.3 | Existing deployments — no forced migration |
| **Modern supported** | 17 | **1.7.0** | 0.8.3 | Managed PG17, full [#161](https://github.com/raphaelmansuy/edgequake/issues/161) on AGE |
| **Recommended** | 18 | **1.7.0** | 0.8.3 | New installs — longest PG support runway |
| **Not supported** | ≤15 | — | — | Out of matrix |

**Key insight:** PG17 and PG18 share the **same AGE capability tier (1.7.0)** — they differ only in PostgreSQL server version. PG16 is the **only** tier stuck on AGE 1.6.0.

Official sources:

- [AGE download matrix](https://age.apache.org/download/) — PG16 → 1.6.0; PG17/PG18 → 1.7.0
- [AGE PG17/v1.7.0 release](https://github.com/apache/age/releases) — upgrade script `age--1.6.0--1.7.0.sql` applies when coming from 1.6 on PG17

---

## First principles (PG17 addition)

### P6 — PG17 is a stepping stone, not a fork

PG17 + AGE 1.7.0 delivers **the same application capability** as PG18 + AGE 1.7.0. Operators choose PG17 when:

- Cloud provider offers PG17 + extensions before PG18
- Incremental migration PG16 → PG17 → PG18 is lower risk than PG16 → PG18 jump
- Org standard is "N-1" PostgreSQL major

**Invariant:** PG17 and PG18 share REQ-042C-04 gate (`extversion >= 1.7.0` for 1.7-only features). PG16 remains at `1.6.0` intersection.

### P7 — Migration is always major-to-major via dump

Valid paths (all via `migrate_postgres_major.sh` — target pins auto-detected):

| From | To | AGE transition | Notes |
| ---- | -- | -------------- | ----- |
| PG16 | PG17 | 1.6.0 → 1.7.0 | May run long 1.6→1.7 upgrade on existing AGE data |
| PG16 | PG18 | fresh 1.7.0 or 1.6 dump | Same as today |
| PG17 | PG18 | 1.7.0 → 1.7.0 | PG major only; lighter than from PG16 |
| PG16 | PG16 | — | Stay — M042/M043 only |

---

## Architecture (three images)

```
                 edgequake-api (single binary)
                           │
     ┌─────────────────────┼─────────────────────┐
     ▼                     ▼                     ▼
 Dockerfile.postgres  Dockerfile.postgres.pg17  Dockerfile.postgres.pg18
 PG16 + AGE 1.6.0      PG17 + AGE 1.7.0         PG18 + AGE 1.7.0
 profile: pg16         profile: pg17            profile: pg18
```

SSOT: `extension-pins.sh` — `EQ_POSTGRES_PROFILE=pg16|pg17|pg18`

---

## Support matrix (REQ-042C)

| Capability | PG16 | PG17 | PG18 |
| ---------- | ---- | ---- | ---- |
| Graph ingest/query | ✅ | ✅ | ✅ |
| pgvector ≥0.8 iterative scan | ✅ | ✅ | ✅ |
| M042/M043 bootstrap | ✅ | ✅ | ✅ |
| AGE 1.7.0 features | ❌ | ✅ | ✅ |
| Issue #161 full closure | Partial | **Full** | **Full** |
| PG community EOL | ~2028-11 | ~2029-11 | ~2030-11 |

---

## Pros of adding PG17

| Pro | Why |
| --- | --- |
| **Cloud availability** | Many hosts ship PG17 before PG18 |
| **Full AGE 1.7 without PG18** | Same extension tier as recommended stack |
| **Smaller migration steps** | PG16→PG17 lower risk than PG16→PG18 for large DBs |
| **Minimal extra code** | Copy Dockerfile.pg18 pattern; same AGE_MIN |
| **Same Cypher intersection rules** | PG17 grouped with PG18 for 1.7 feature gates |

## Cons of adding PG17

| Con | Mitigation |
| --- | ---------- |
| **3× Docker images + CI** | `check_extension_pins.sh all`; parallel CI jobs |
| **3× E2E on release** | Shared bootstrap code; profile-specific image verify |
| **Operator choice paralysis** | Clear tier labels in docs + `/health` |
| **PG17 AGE 1.6→1.7 upgrade path** | Document in migration runbook when restoring PG16 dumps to PG17 |

---

## Operator selection guide

| Situation | Choose |
| --------- | ------ |
| Existing PG16 production | **Stay PG16** or migrate to PG17/PG18 when ready |
| RDS offers PG17 + AGE, not PG18 yet | **PG17** — `make postgres-image-build-pg17` |
| New greenfield install | **PG18** (recommended) |
| Need AGE 1.7.0 | **PG17 or PG18** (not PG16) |
| Incremental migration | PG16 → PG17 → PG18 via dump each hop |

---

## Requirements (REQ-042C — updated)

| ID | Requirement |
| -- | ----------- |
| REQ-042C-01 | PG16 image verified |
| REQ-042C-02 | PG17 image verified (`Dockerfile.postgres.pg17`, AGE 1.7.0) |
| REQ-042C-03 | PG18 image verified |
| REQ-042C-04 | Single app binary; AGE 1.6 intersection OR `extversion >= 1.7.0` gate |
| REQ-042C-05 | `/health` ext versions |
| REQ-042C-06 | E2E: PG16 + PG17 + PG18 image proofs on release |
| REQ-042C-07 | PG16 supported until ~2028-11 |
| REQ-042C-08 | PG18 recommended; PG17 modern supported; none exclusive |
| REQ-042C-09 | `migrate_postgres_major.sh` auto-detects target major (16/17/18) |

---

## Verdict

**PG17 support: YES** — same AGE 1.7.0 tier as PG18, minimal incremental cost, high value for managed-cloud reality. Triple-track replaces dual-track; PG16 legacy path unchanged.
