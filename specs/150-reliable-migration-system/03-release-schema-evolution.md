# 03 — Release and schema evolution

Parent: [README](README.md) · Incidents: [02](02-incident-catalogue.md) · Upgrade matrix: [07](07-upgrade-path-matrix.md)

Evidence: `git tag --sort=creatordate`, `git cat-file -p <tag>:edgequake/migrations/... | shasum -a 384`, `git diff --name-status` consecutive tags, HEAD `ls`. Captured 2026-09-26. Path has always been `edgequake/migrations/` (v0.2.0 through HEAD).

## Epochs (consecutive tags with identical numbered set collapsed)

| Tag(s) | Date (tagger) | Count | Highest file |
|--------|---------------|------:|--------------|
| v0.2.0 – v0.4.1 | 2026-02-12 → 02-23 | 23 | `024_fix_task_status_constraint.sql` |
| v0.5.1 | 2026-02-24 | 24 | `025_add_pdf_unique_checksum.sql` |
| v0.5.5 – v0.6.0 | 2026-03-17 | 25 | `026_fix_task_type_constraint.sql` |
| v0.7.0 | 2026-03-18 | 28 | `029_add_vector_btree_indexes.sql` |
| v0.8.0 – v0.9.4 | 2026-04-03 → 04-08 | 29 | `030_add_server_config.sql` |
| v0.9.5 – v0.9.6 | 2026-04-08 | 30 | `031_refresh_edgequake_tasks_view.sql` |
| v0.9.7 – v0.9.19 | 2026-04-08 → 04-10 | 31 | `032_fix_document_status_constraints.sql` |
| v0.10.0 – v0.10.5 | 2026-04-11 → 04-19 | 33 | `034_fix_pdf_documents_edgeparse_constraint.sql` |
| v0.10.6 – v0.11.2 | 2026-04-19 → 04-29 | 34 | `035_harden_task_compatibility_defaults.sql` |
| v0.11.3 – v0.12.5 | 2026-05-06 → 05-28 | 35 | `036_add_edge_property_indexes.sql` |
| v0.12.6 | 2026-05-29 | 36 | `037_backfill_vector_tenant_workspace_columns.sql` |
| v0.12.7 – v0.12.11 | 2026-06-07 → 06-09 | 37 | `038_add_source_ids_gin_indexes.sql` |
| v0.13.0 – v0.13.1 | 2026-07-02 | 76 | `077_post_startup_index_cleanup.sql` (+39) |
| v0.13.2 | 2026-07-03 | 77 | `078_age_child_workspace_stats.sql` |
| v0.13.3 | 2026-07-03 | 78 | `079_age_child_node_index_reconcile.sql` |
| v0.14.0 – v0.15.1 | 2026-07-04 → 07-09 | 80 | `081_age_graph_rls_marker.sql` |
| v0.16.0 | 2026-07-10 | 82 | `083_age_native_unique_index_reconcile.sql` |
| v0.17.0 – v0.18.0 | 2026-07-14 → 07-16 | 85 | `086_edge_bfs_index_reconcile.sql` |
| v0.19.0 | 2026-07-17 | 88 | `089_refresh_tasks_view_lease_columns.sql` |
| v0.20.0 | 2026-07-21 | 93 | `094_extend_task_type_deletion.sql` |
| v0.20.1 – v0.20.2 | 2026-07-22 | 94 | `095_extend_task_type_workspace_wipe.sql` |
| v0.21.0 | 2026-07-23 | 96 | `097_edge_multigraph_rel_type.sql` |
| v0.21.1 – v0.21.3 | 2026-07-24 → 07-26 | 97 | `098_batch_deletion_task_type_and_claim_index.sql` |
| v0.22.0 | 2026-07-26 | 104 | `105_pdf_blob_cutover.sql` |
| v0.23.0 | 2026-08-03 | 139 | `141_spec098_document_lifecycle_status.sql` (+35) |
| v0.24.0 – v0.24.1 | 2026-08-03 | 140 | `142_spec105_legacy_cutover_assert.sql` |
| v0.24.2 – v0.24.3 | 2026-08-07 | 142 | `144_spec111_legacy_vector_id_ws_unique.sql` |
| v0.24.4 | 2026-08-12 | 145 | `147_messages_llm_lineage.sql` |
| v0.25.0 | 2026-08-17 | 146 | `148_document_pages_layout.sql` |
| v0.26.0 – **v0.26.10** | 2026-08-24 → 09-19 | 147 | `149_tasks_document_id_column.sql` |
| HEAD (2026-09-26) | unreleased | 156 | `158_graph_lineage_orphan_document_repair.sql` |

97 release tags `vX.Y.Z` plus 12 ignored per-crate `*-v0.2.2` tags. GHCR workflow first tagged **v0.23.0** (`release-docker.yml` added 2026-04-08, first contained tag v0.23.0) — older epochs must be built from git tags, not pulled.

## Immutability violations (shipped numbered file later `M`)

No numbered file was deleted or renamed after a tag. These `M` diffs are exactly sqlx `VersionMismatch`:

| File | First shipped | Changed in | SHA-384 prefix shipped → changed | Repair module? |
|------|---------------|------------|----------------------------------|----------------|
| `019_add_tenant_workspace_to_tasks.sql` | v0.2.0 | **v0.10.6** then revert **v0.11.0** | `1f538faa36762ad72045e005` → `7b544306c5da16b05ec0607a` → `1f538faa…` | **None** |
| `001_init_database.sql` | v0.2.0 | **v0.11.0** then revert **v0.11.1** | `bb40c61f7d5cbeafa7827f2e` → `9e44513e1b22ab482a3703f3` → `bb40c61f…` | **None** |
| `078_age_child_workspace_stats.sql` | v0.13.2 | v0.13.3 | `d22cc6d8416c6a8ccf28542c` → `a043177271c82c65a7509855` | `reconcile/m078.rs` |
| `071_hnsw_optimize.sql` | v0.13.0 | v0.14.0 | `fa6cce9c4b088b5dbc850764` → `fea7b113e1aab4f88d0c22a0` | `reconcile/m071.rs` |
| `118_spec091_wsdoc_backfill.sql` | v0.23.0 | v0.24.2 | `331967467fdbeb58aeeb41ca` → `a35e70d52e12215abe84283e` | `reconcile/m118.rs` |
| `121_spec091_injection_backfill.sql` | v0.23.0 | v0.24.2 | `da347384f34eb9db99d635f4` → `57088e874c47e6c558279388` | `reconcile/m121.rs` |
| `125_spec091_kv_drop.sql` | v0.23.0 | v0.24.2 | `67b73fd0f683dd5cae06213a` → `9ae99858a9c88ec9b0a19544` | `reconcile/m125.rs` |
| `131_spec091_fleet_vector_drop.sql` | v0.23.0 | v0.24.2 | `461fa2a7c560513df711f954` → `1b42205577666dc31fa346c4` | `reconcile/m131.rs` |

Repair allowlist (code + Makefile twin, **excludes 001 and 019**):

```24:24:edgequake/crates/edgequake-api/src/state/migration_bootstrap/checksum_repair.rs
pub const KNOWN_CHECKSUM_REPAIR_VERSIONS: &[i64] = &[71, 78, 118, 121, 125, 131];
```

```1167:1167:Makefile
KNOWN_CHECKSUM_REPAIR_VERSIONS := 71,78,118,121,125,131
```

Repair rewrites checksum **only** when stored hash equals the known-broken value **and** `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` contains the version or `EDGEQUAKE_DEV_MODE` is truthy (`checksum_repair.rs:52-59`). Production default: deny.

`checksums.lock` (first in v0.11.2) matches HEAD 156 files (`shasum -a 384` vs lock, 2026-09-26). Header says never modify a deployed line; lines for 071/078/118/121/125/131 were rewritten anyway. Lock does **not** record 001/019 fossils.

## HEAD numbering

- 156 files, `001`..`158`, **gaps 018 and 127** (never existed in `git log --all`).
- No duplicate prefixes.
- Unreleased vs v0.26.10 (`git diff --name-status`): **A** 150–158 only. `NOTES.md` still says "Current max: 105" (stale).

## Schema domains at HEAD (numbered files)

~75 `CREATE TABLE` names. Schemas: `public` (ledger + core), `edgequake` (ops), `ag_catalog` (Apache AGE). Extensions: `uuid-ossp`, `vector`, `age`, `pg_trgm`; `btree_gin` in Docker init.

| Domain | Representative tables / origin |
|--------|--------------------------------|
| Tenancy | `tenants`, `workspaces`, `memberships`, `users`, `api_keys` (001/007/008) |
| Documents | `documents`, `folders`, `pdf_documents`, `pdf_document_blobs` (103/105), `document_pages` (148) |
| Chunks | `chunks`, `chunk_entity_links` (066) |
| Graph | AGE graphs (013); `entities`, `relationships`; `graph_contributions` (150) |
| Vectors | `chunk_embeddings` (108), `entity_embeddings` (130), `embedding_projections` (151); legacy `eq_*_kv` / `eq_*_vectors` dropped 125/126/131/142 |
| Tasks | `tasks` (partitioned 104), `jobs`, `edgequake.provider_slot` (110) |
| Auth / audit | `audit_logs` (012), RLS (009/096) |
| Outbox / projections | 150–155 provider-access ledger family |
| Ops | `server_config` (030), `edgequake_reconcile_state` (102), `edgequake_migration_job` (106) |

Marker files (<15 lines): 042–065, 080, 081, 092, 097, 100, 102, 123, 135–137, 145, 147. Real work often in `support/NNN` + `reconcile/mNNN.rs`.

Largest numbered files (`wc -l`): 001 725, 009 422, 022 405, 012 389, 013 290, 039 244, 010 232, 150 213, 125 209, 131 181.

## Risky SQL (speed / correctness)

| File | Risk |
|------|------|
| 128, 129, 130, 132, 143, 144 | Inner `BEGIN;`/`COMMIT;` while sqlx already wraps a txn — ledger insert can commit separately |
| 156, 158 | Full AGE Node/EDGE rewrite in **one** `DO` / one sqlx txn; `statement_timeout = 0` |
| 132, 129, 130, 071 | `ALTER COLUMN … TYPE halfvec` + non-concurrent HNSW (ACCESS EXCL / SHARE) |
| 125, 126, 131, 142 | Irreversible DROP, gated `--confirm-drop` |
| 117–122 | Unbounded `INSERT…SELECT` KV → typed |
| 141 | `NOT VALID` + `VALIDATE` **same** transaction |
| support/140, 141 | Spawned at serving boot: ALTER, full UPDATE, unique index, CHECK validate |

Zero numbered files start with `-- no-transaction`. Zero use `CREATE INDEX CONCURRENTLY` (comments in 128 admit this). `support/038` concurrent path is `scripts/migrations/apply_038.sh --concurrent` only.

## Divergent SQL trees (not sqlx)

| Path | Role |
|------|------|
| `edgequake/docker/init-extensions.sql` | Compose initdb: extensions + `search_path` |
| `deploy/kubernetes/helm/edgequake/files/init-extensions.sql` | Same, shorter comments |
| `edgequake/docker/init.sql` | Legacy 1147-line full schema; **not mounted**; drifted from 039+ |
| `edgequake/docker/migrations/002_add_age_vertex_indexes.sql` | Orphan; number collides conceptually with sqlx 002 |
| `migrations/scripts/reset_migrations.sql` | `DROP TABLE _sqlx_migrations` — operator footgun |
| `edgequake-storage/migrations_sqlite/001–003` | Separate SQLite track; 003 untracked at SPEC-149 push time |

`support/**` is **not** in `checksums.lock`.
