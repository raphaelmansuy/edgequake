# SPEC-041 — First Principles

---

## 1. Migrations are the schema contract

Startup applies pending sqlx migrations **before** serving traffic. A migration that fails on a supported configuration (AGE + graph) is a **P0 defect**, not an ops workaround.

**Invariant:** Every migration statement must be valid PostgreSQL on all paths it executes.

---

## 2. Operator SSOT — one pattern everywhere

Expression indexes on AGE node properties follow a single canonical form:

```sql
(ag_catalog.agtype_to_json(properties)->>'property_key')
```

**SSOT hierarchy (DRY):**

| Layer | File | Role |
| ----- | ---- | ---- |
| Runtime bootstrap | `graph_lifecycle.rs:164-177` | Creates indexes on new graphs |
| First migration | `014_add_graph_indexes.sql:43-91` | Parent-table pattern (legacy) |
| Repair migrations | `046/support`, `078`, `072` | Child-table repair |
| Ops concurrent | `support/078/concurrent.sql` | Production large-graph variant |

M078 must **byte-match the operator** used in SSOT — not invent a variant.

---

## 3. Test what you ship

| Environment | M078 behavior | Pre-fix test coverage |
| ----------- | ------------- | --------------------- |
| No AGE | No-op, success | ✅ Covered implicitly |
| AGE, no graphs | Loop empty, success | ✅ Covered implicitly |
| AGE + `"Node"` table | CREATE INDEX executes | ❌ **Missed** — prod path |

**Principle:** E2E must exercise the **highest-risk path** (CREATE INDEX on real `"Node"` rel), not only the skip path.

---

## 4. Immutability vs. never-applied fixes

sqlx records SHA-384 per applied migration. Two populations after v0.13.2:

| Population | `_sqlx_migrations` v78 | Action |
| ---------- | ---------------------- | ------ |
| **Blocked** (AGE + Node, CREATE failed) | Not recorded | Fixed M078 applies on retry ✅ |
| **Passed** (no Node table / no AGE) | Recorded with old checksum | Need checksum repair before upgrade ⚠️ |

Fixing M078 in place is correct for blocked installs. Checksum repair is required for passed installs — documented in [007-release-runbook.md](./007-release-runbook.md).

---

## 5. SOLID mapping

| Principle | Application |
| --------- | ----------- |
| **S** — Single responsibility | M078 only repairs child indexes; operator fix doesn't touch M071 HNSW |
| **O** — Open/closed | Fix via corrected DDL + E2E gate; no new migration number needed for typo |
| **L** — Liskov | Concurrent script must be substitutable for inline M078 (same expressions) |
| **I** — Interface segregation | Static grep (syntax) vs DB E2E (semantics) — separate proofs |
| **D** — Dependency inversion | Tests depend on PostgreSQL operator rules, not on "it worked locally" |

---

## 6. Out of scope (Issue #273 additional context)

M071 HNSW dimension guard (#273 reporter note) is a **separate defect** with separate failure mode. SPEC-041 tracks it in cross-ref matrix but does not fix it here — avoids scope creep and mixed rollback.
