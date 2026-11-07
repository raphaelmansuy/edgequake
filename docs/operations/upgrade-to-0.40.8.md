---
title: "Upgrade to EdgeQuake v0.40.8"
description: "Upgrade from v0.40.7 to v0.40.8. Schema moves 227 → 231."
---

# Upgrade to EdgeQuake v0.40.8

> **From:** v0.40.7 (schema **227**) · **To:** v0.40.8 (schema **231**)

Run `edgequake migrate` before you start the new API replicas. Four migrations land. All four are `expand`. They rewrite policy text, replace two indexes, and add one nullable column. They drop no data. A v0.40.7 binary keeps working against the migrated schema, so the migration can run ahead of the image bump.

| Version | File | Adds |
| ------- | ---- | ---- |
| **228** | `228_clearance_bypass_initplan.sql` | The clearance policy evaluates its row-independent half once per statement. `clearance_covers()` still means the same thing. |
| **229** | `229_tenant_workspace_clearance_indexes.sql` | `(tenant_id, workspace_id)` on `chunks` and `documents` covers the clearance columns. Two duplicate indexes are dropped. |
| **230** | `230_rls_initplan_backfill.sql` | Six policies added after migration 208 wrap `current_tenant_id()`, `current_workspace_id()`, and `current_user_id()` as InitPlans. |
| **231** | `231_vertex_google_project.sql` | A Vertex connection stores its Google Cloud project. Queries use that project. |

## Order of operations

```bash
# 1. Back up, then migrate. The advisory lock makes this safe with the old
#    replicas still serving.
edgequake migrate

# 2. Roll the API replicas to 0.40.8.
# 3. Roll the WebUI to 0.40.8.
```

`EDGEQUAKE_SCHEMA_GATE` controls what a replica does when it boots ahead of the migration. The default `fail` exits with code 78. Helm and compose set `wait`, which serves `/live` and answers `503` on `/ready` until the ledger reports the schema current.

Take a backup before the migrate. The statements take an access-exclusive lock while they replace the policies and the two indexes. On the measured database (120,000 chunks) the read path after the migrate is a few milliseconds. Query results do not change: a scoped read still sees the same rows, and a wrong tenant still sees none.

## What changes for an operator

**The Google Cloud project is saved on the Vertex connection.** On Connections, choose Vertex AI and fill in Google Cloud project. Save. Queries use that project. The server does not need `GOOGLE_CLOUD_PROJECT`. A service-account file can leave the field blank; the project inside the file is saved. The deployment provider card shows the saved project.

**Row security does less work per row.** A check that does not read the row runs once per statement. Counting chunks or documents under the tenant role reads the scope index and does not visit the heap for clearance columns. When clearance enforcement is off, or break-glass is on, the per-row clearance branch does not run.

**Full-text search on the tenant role is unchanged.** The `@@` operator is not leakproof, so that role still cannot use the GIN index. The default application role bypasses row security and still uses it. A reader that would bypass row security to reach the GIN index was measured and not shipped.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/packs/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
