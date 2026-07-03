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

## 4. Immutability vs. never-applied fixes — three-layer repair

sqlx records SHA-384 per applied migration. **First-principles response:** do not rely on a single mechanism.

| Layer | When | Mechanism |
| ----- | ---- | --------- |
| **L1 Pre-sqlx** | v0.13.2 skip-path (v78 old checksum) | `repair_migration_078_checksum_if_needed()` in bootstrap |
| **L2 sqlx** | Pending migrations | Fixed M078 + M079 idempotent CREATE INDEX |
| **L3 Post-bootstrap** | v78/v79 recorded, indexes missing | `reconcile_migration_078()` → `support/078/apply.sql` |

**DRY SSOT:** `migrations/support/078/apply.sql` — single index-creation logic for L3; M078/M079 sqlx files stay aligned.

### Population matrix

| Population | `_sqlx_migrations` v78 | Fix path |
| ---------- | ---------------------- | -------- |
| **Blocked** (AGE + Node, CREATE failed) | Not recorded | L2 fixed M078 applies on retry |
| **Skip-path** (no Node at upgrade) | Recorded, old checksum | L1 repair → L2 M079 → L3 if graphs added later |
| **Fresh ≤ v0.13.1** | Not recorded | L2 M078 + M079 |
| **Manual ops** | Any | `repair_migration_078_checksum.sh` (L1 equivalent) |

Fixing M078 in place is required for blocked installs. L1 removes manual ops for skip-path upgrades.

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
