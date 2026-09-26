# 02 — Incident catalogue

Parent: [README](README.md) · Principles: [01](01-first-principles.md) · Defects: [05](05-root-cause-analysis.md)

Twenty-nine migration/upgrade incidents (26 closed, 2 open, 1 internal). Sources: spec packs 041/093/110/111/120/136/137/139, `CHANGELOG.md`, `git log`, GitHub issues via `gh`.

**Not in this list:** SPEC-140 / #388 (pagination, no schema delta 0.24.2→0.26.3), SPEC-044 (AGE Cypher bind), #356 (`graphid` query). SPEC-93 is a **validation run**, not an incident (GREEN, 600-doc synthetic).

## Class legend

| Tag | Meaning |
|-----|---------|
| Edit | Shipped `NNN_*.sql` edited; sqlx checksum drift |
| Card | `ON CONFLICT` 21000 or unique 23505 |
| Backfill | Data-copy or readiness lying / incomplete |
| Ext | AGE / pgvector / PG major layout |
| SP | `search_path` or stale view |
| DDL | Long lock / `lock_timeout` / SHARE index build |
| Order | Numbering, drop-gating, flag tokens, image skew |
| Stale | Code still touches a dropped relation |

## Early era (v0.9 – v0.12)

| ID | When | Operator symptom | Class | Fix | Kind |
|----|------|------------------|-------|-----|------|
| MIG-019-dup | 2026-01-28 | Duplicate numbers 014/015/018; second run failed | Order | Deleted dupes; idempotent 019 | Patch |
| v0.9.5 view | →0.9.5 | `column "error" does not exist` | SP | 031 recreates `SELECT *` views | Patch (recurred SPEC-057) |
| v0.9.6 ledger | →0.9.6 | duplicate `_sqlx_migrations_pkey` every restart | SP | Pin `search_path TO public` on every connection | **Structural** |
| v0.9.7 status | →0.9.7 | `documents_valid_status` rejects `partial_failure` | Wrong DDL | 032 | Patch |
| 019 edited | 0.10.1→0.10.12 | `migration 19 was previously applied but has been modified` | **Edit** | Restored bytes in 0.11.0 | Patch, **no repair** |
| sqlx-cli dup | 0.10.12 | Duplicate key on second `sqlx migrate run` | SP | URL `search_path=public`; 001 `SET LOCAL` — **caused #195** | Patch |
| [#195](https://github.com/raphaelmansuy/edgequake/issues/195) | 0.10.12→0.11.0 ECS | `migration 1 was previously applied but has been modified` | **Edit** | 0.11.1 restore 001; `checksums.lock` + CI (`220d44e44`) | Patch then **structural** |
| M028/M037 | →0.12.6 | Columns missing after 028 | Missing | 037 `IF NOT EXISTS` + backfill | Patch |
| FIX-MIG038 | 0.12.7–0.12.8 | GIN indexes silently absent (`json` vs `jsonb`) | Ext+DDL | Marker 038; real DDL in `support/038`; `/ready` 503 | Partial structural |

## v0.13 – v0.22 (AGE, pgvector, large graphs)

| ID | When | Operator symptom | Class | Fix | Kind |
|----|------|------------------|-------|-----|------|
| [#273](https://github.com/raphaelmansuy/edgequake/issues/273) / SPEC-041 | ≤0.13.1→0.13.3 | `operator does not exist: json ->>> unknown` (M078) | Ext+Edit | Typo `->>>`; checksum repair; M079; CI grep | Hybrid |
| [#275](https://github.com/raphaelmansuy/edgequake/issues/275) / SPEC-042 | 0.13.3 | HNSW "too many dimensions" (pgvector 2000) | Ext+Edit | M071 dimension-aware / halfvec; m071 repair | Hybrid |
| 0.14.0 fresh | fresh PG16 | Repair queried `_sqlx_migrations` before create | Caused-by-fix | Table-exists guard | Patch |
| [#280](https://github.com/raphaelmansuy/edgequake/issues/280) | 0.14 PG18 image | `pg_ctlcluster` crash (data dir layout) | Infra | Compose volume `/var/lib/postgresql` | Patch |
| [#288](https://github.com/raphaelmansuy/edgequake/issues/288) | →0.15.0 | Login 401 | Backfill | Bootstrap admin + KV import | Patch |
| SPEC-057 view | 0.19.0 | `lease_*` missing on `edgequake.tasks` | SP | View refresh + pre-commit hook `5a8da3503` | Guard |
| SPEC-083 X-03 | 0.20.2 ~178k nodes | `column e.eq_source_id does not exist`; ingest storm | DDL | `EDGEQUAKE_EQ_MAINTENANCE=1` (120s lock_timeout, batches) | Partial |
| v0.13.0 indexes | 0.12.11→0.13.0 | Writes blocked during 070/071/074 SHARE builds | DDL | Runbook only; CONCURRENTLY still "future" | Docs |

## SPEC-091 cutover (v0.23 – v0.26)

| ID | When | Operator symptom | Class | Fix | Kind |
|----|------|------------------|-------|-----|------|
| 0.23.0 internal | cut | Pending 128–132 stuck behind gated 131; soak abort 125 | Order+Backfill | Expandable-first apply | **Structural** |
| LD-15 | 0.23.0 | (policy) boot could apply drops | Policy | Verify-only boot; exit 78 | **Structural** |
| [#362](https://github.com/raphaelmansuy/edgequake/issues/362) | 0.23/0.24.1 | `statement timeout` on ~72k residue rows | Backfill+**Edit** | `::uuid` in residue.rs **and M125** | Patch+Edit |
| [#363](https://github.com/raphaelmansuy/edgequake/issues/363) | 0.23.0 | iw2 dropped ~99.7% relationships, `failed_count: 0` | **Backfill silent** | Honest counts (0.24.2) | Patch |
| [#364](https://github.com/raphaelmansuy/edgequake/issues/364) | 0.24.1 | Drop readiness required legacy count already 0 | Gate | Coverage (`uncovered_*==0`) | Structural-ish |
| SPEC-110 | 0.24.1 stuck 117 | M118 `ON CONFLICT DO UPDATE cannot affect row a second time` | **Card** | `DISTINCT ON`; edited 118+121 | Patch+**Edit** |
| SPEC-111 | 0.24.1 / `make dev` | `Migration 125 checksum drift` | **Edit** | Scoped `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` | Policy |
| M143→M144 | 0.24.2 | `idx_entity_embeddings_legacy_vector_id` 23505 | **Card** | Unique `(workspace_id, legacy_vector_id)` | New migration |
| [#374](https://github.com/raphaelmansuy/edgequake/issues/374) / SPEC-120 | 0.24.2–0.24.3 | Same 23505 under concurrent ingest | **Card** | Absorb on INSERT (0.24.4) | Patch |
| [#377](https://github.com/raphaelmansuy/edgequake/issues/377) / SPEC-136 | →0.25.0 | 23505 on stamp-once UPDATE; retry identical | **Card** | Absorb 23505 (0.26.0) | Patch |
| [#383](https://github.com/raphaelmansuy/edgequake/issues/383) | 0.25.0 | Compensation DELETE on dropped `eq_*_vectors` | **Stale** | Skip if gone (0.26.0) | Patch |
| SPEC-137 | 0.25→0.26 | `--drop-confirm` ignored; Wave D abort; wrong `pg_locks` hint | Order | Consent SSOT + alias; abort class (0.26.1) | **Structural** CLI |
| SPEC-139 | 0.26.1 field | iw2 21000; W3 44580 vs 18503; uncovered_fleet frozen | Card+Backfill+Order | Dedupe batches; per-table coverage; remainder jobs (0.26.3) | Mostly structural |
| [#396](https://github.com/raphaelmansuy/edgequake/issues/396) **OPEN** | 0.26.0→0.26.5 | `guard` still RED | Backfill+skew | Unresolved (CLI/API skew; cursor advances on skip) | Open |
| [#405](https://github.com/raphaelmansuy/edgequake/issues/405) **OPEN** | 0.26.0 | `42P01` FTS then in-memory BM25 fallback | **Stale** | None | Open |

No field incident for: kubelet killing a **migrate Job** (API pods yes, via probes on the API), replica races on `MIGRATOR.run` (sqlx advisory lock covers that slice only), or GRANT/ownership failures.

## Recurrence (top 5)

| Rank | Class | Count | Lesson for SPEC-150 |
|------|-------|-------|---------------------|
| 1 | Backfill / verify lying | 8 | Data phase must not advance past failed batches; coverage is per-table not global |
| 2 | Edited shipped SQL | 7 events / 8 files | Variant registry (LAW-150-4); stop editing bodies |
| 3 | Cardinality / unique | 6 | Lint `ON CONFLICT` targets; multi-workspace seeds in CI |
| 4 | Order / gating / flags | 5 | Manifest phases; reject unknown CLI flags (already 0.26.1) |
| 5 | AGE / pgvector / search_path | 4 (+ SP 4 if split) | CI with AGE + non-empty graphs; pin search_path (done) |

## Fix-causes-next-incident chain

```text
  SET LOCAL on 001  -->  #195 checksum 001
  M078 typo fix     -->  checksum repair queries missing table (fresh PG16)
  M143 unique idx   -->  #374 race  -->  #377 stamp  -->  #383 DELETE dropped
  M125 uuid cast    -->  SPEC-111 checksum 125/131
  DISTINCT ON 118   -->  "smallest workspace wins" (SPEC-110 honesty)
```

## What prior specs already said was unfinished

- SPEC-041: M071 HNSW >2000 dims out of scope — became #275 the same day.
- SPEC-110: no GHCR 0.24.2 image, no partner DB, checksum UPDATE untested against a real DB.
- SPEC-111 brutal honesty: ship with runbook, not blind upgrade; 125/131 need allowlist.
- SPEC-90 §8: `support/` not checksum-locked; every-boot reconcile unbounded; integer numbering can collide across branches. (Its claim that CI does not check `checksums.lock` is **false**: `ci.yml:32-41`.)
- SPEC-93 non-goals: no 100k+ vector soak, not a PR gate.

Cross-ref: [03](03-release-schema-evolution.md) for hashes; [05](05-root-cause-analysis.md) for D-ids; [12](12-risks-honest-assessment.md) for #396/#405.
