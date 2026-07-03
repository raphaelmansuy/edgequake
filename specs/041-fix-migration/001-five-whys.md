# SPEC-041 — Five Whys (#273)

**Issue:** Migration 078 fails at startup — `operator does not exist: json ->>> unknown`

---

## WHY 1 — Why does the backend fail to start?

Migration 78 executes `CREATE INDEX ... agtype_to_json(properties)->>>'workspace_id'` inside a `DO $$` block. PostgreSQL rejects `->>>` as an unknown operator. sqlx aborts the migration batch; the API never binds to port 8080.

**Evidence:** [Issue #273](https://github.com/raphaelmansuy/edgequake/issues/273) — ECS Fargate v0.13.2 upgrade from migration ≤ 70.

---

## WHY 2 — Why does PostgreSQL reject the operator?

PostgreSQL JSON/JSONB exposes exactly two key navigators:

| Operator | Returns | Example |
| -------- | ------- | ------- |
| `->` | json/jsonb | `'{"a":1}'::json->'a'` → `1` |
| `->>` | text | `'{"a":1}'::json->>'a'` → `"1"` |

`->>>` does not exist. `ag_catalog.agtype_to_json(properties)` returns `json`, so only `->` and `->>` apply.

**Evidence:** PostgreSQL docs; every other migration in repo uses `->>` (M014, M036, M046, M074, `graph_lifecycle.rs`).

---

## WHY 3 — Why did `->>>` appear in M078?

SPEC-040 implementation (2026-07-02) introduced M078 by adapting the M046 / `graph_lifecycle.rs` pattern. A transcription typo added an extra `>` when doubling quotes inside `format()` / `EXECUTE format()` strings (`''workspace_id''`).

**Evidence:** `specs/040-edgequake-issues/006-postgres-age-pgvector-lens.md:95-98` reproduces the same typo in proposed SQL — copy-paste propagation.

---

## WHY 4 — Why wasn't this caught before v0.13.2 release?

| Gap | Detail |
| --- | ------ |
| CI migration path | Dev/CI DB often has AGE installed but **empty graphs** — M078 loops, finds no `"Node"` table, skips CREATE INDEX, migration **succeeds** |
| No static grep | No CI rule banning `->>>` in `migrations/` |
| No AGE+graph E2E gate | `migration_bootstrap_proof.rs` checks M038/M046 markers, not M078 index DDL |
| SPEC-040 perf proof | `measure_graph_stats_perf.sh` assumes M078 already applied — never exercised CREATE INDEX on populated graph |

**Evidence:** `008-implementation-plan.md` — "Migration M078 auto-deploy verified on local DB" without Node-table CREATE path.

---

## WHY 5 — Why does this matter in production?

1. **Total outage** — migrations run at startup; failure is fatal (no degraded mode).
2. **Upgrade trap** — users on ≤ M070 upgrading to v0.13.2 hit M071–M078 in one batch; blocked at 78.
3. **SPEC-040 intent inverted** — M078 was meant to **fix** graph stats timeouts (#262); instead it **prevents** service start.

---

## Root cause (single sentence)

> **Human typo (`->>>`) in M078 DDL, combined with CI that never applied CREATE INDEX against an AGE graph with a `"Node"` child table.**

## Corrective actions

1. Fix operator in M078 + concurrent script (REQ-041-01/02)
2. Add static grep gate (REQ-041-04)
3. Add AGE-graph migration E2E (REQ-041-05/06/07)
4. Document checksum repair for no-graph success path (REQ-041-08)
