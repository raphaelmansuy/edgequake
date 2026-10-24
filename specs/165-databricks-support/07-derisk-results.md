# 07 — Probe results

Parent: [README](README.md) · Previous: [06 Implementation plan](06-implementation-plan.md) · Next: [08 Risks](08-risks.md)

**When:** 2026-10-21 (UTC).  
**Workspace:** `https://dbc-cb61f1e1-bd8e.cloud.databricks.com`.  
**Auth:** a personal access token in the process environment. The token is not stored in this repository. Rotate it.

No connection string, password, or credential token is recorded here.

## Inventory

| Call | HTTP | Result |
|------|------|--------|
| `GET /api/2.0/preview/scim/v2/Me` | 200 | Caller is active. Identity is not copied into this doc |
| `GET /api/2.0/apps` | 200 | `{}` |
| `GET /api/2.0/postgres/projects` | 200 | `{}` before the create |
| `GET /api/2.0/serving-endpoints` | 200 | 12 endpoints, all `READY` |
| `GET /api/2.1/unity-catalog/models?catalog_name=system&schema_name=ai` | 200 | First page has 100 models and a next page |

READY endpoint names included `databricks-gpt-oss-20b`, `databricks-llama-4-maverick`, `databricks-gemma-3-12b`, `databricks-gte-large-en`, `databricks-bge-large-en`, `databricks-qwen3-embedding-0-6b`.

## AI Gateway

Base: `/ai-gateway/mlflow/v1`.

| Kind | Model | HTTP | Body fact |
|------|-------|------|-----------|
| Embeddings | `system.ai.gte_large_en_v1_5` | 404 | "does not exist" |
| Embeddings | `system.ai.gte-large-en` | 200 | Vector length **1024** |
| Chat | `system.ai.gemma-3-12b-it` | 404 | "does not exist" |
| Chat | `system.ai.meta_llama_v3_1_8b_instruct` | 404 | "does not exist" |
| Chat | `databricks-meta-llama-3-1-8b-instruct` | 403 | No longer available. Use a Unity Catalog model service |
| Chat | `system.ai.gpt-oss-20b` | 200 | `content` is a list. Part type observed: `reasoning`. Message keys: `content`, `role` |
| Chat | `system.ai.llama-4-maverick` | 200 | `content` is a string. Lowercase body contained `pong`. Length 4 |
| Chat | `system.ai.qwen3-next-80b-a3b-instruct` | 200 | `content` is a string |
| Legacy invocations | `POST /serving-endpoints/databricks-gemma-3-12b/invocations` | 200 | `content` is a string |

`usage` on the `gpt-oss-20b` call included `cache_read_input_tokens`. That field is extra. The blocking field is the list-shaped `content`.

## Lakebase project

Created because the project list was empty.

| Item | Value |
|------|-------|
| Create | `POST /api/2.0/postgres/projects?project_id=edgequake-spec165-probe` with `pg_version` 18. HTTP 200. The body was an operation. A following GET showed the project |
| Project | `projects/edgequake-spec165-probe` |
| Branch | `production`. `status.current_state` was 5 characters, the length of `READY`. The endpoint state below was read in full |
| Endpoint | `primary`, `ENDPOINT_TYPE_READ_WRITE`, `current_state` `ACTIVE` |
| Host | `ep-gentle-flower-d885svsz.database.us-east-2.cloud.databricks.com` |
| Database | `databricks_postgres` |
| Suspend | `suspend_timeout_duration` 300s. `no_suspension` was not set |
| Credential | `POST /api/2.0/postgres/credentials` HTTP 200. Requested TTL 300s. `expire_time` was returned. Token length 2082. Token not stored |
| Login | Project owner email as `user`, token as password, `sslmode=require`. Connected |

The project is still there. This study did not delete it.

## SQL

| Statement | Result |
|-----------|--------|
| `SELECT version()` | PostgreSQL 18.6 (`653b019`) |
| `rolsuper`, `rolcreatedb`, `rolcreaterole` for `current_user` | false, true, true |
| `SHOW max_connections` | 225 |
| Available: `vector` | 0.8.6, not installed until the next statement |
| Available: `pg_trgm` 1.6, `uuid-ossp` 1.1, `btree_gin` 1.3, `databricks_auth` 1.0, `lakebase_vector` 1.1.1 | Present, not installed yet |
| Available: `age` | No row |
| `CREATE EXTENSION vector` | Success. `extversion` 0.8.6 |
| `CREATE EXTENSION pg_trgm`, `uuid-ossp`, `btree_gin` | Success |
| `'[1,2,3]'::halfvec(3)` | `[1,2,3]` |
| `uuidv7()` | Returned a uuid (version nibble 7) |
| `gen_random_uuid()` | Returned a uuid |
| `pg_advisory_lock(42)` then `pg_advisory_unlock(42)` | Unlock returned true |
| `CREATE EXTENSION age` | Fail. `extension "age" is not in the allowed extensions list` |
| HNSW on `spec165_probe.items (halfvec(3))` with `halfvec_cosine_ops` | Index created. `ORDER BY embedding <=> '[1,0,0]'` returned id 1 |
| `CREATE EXTENSION lakebase_vector` | Fail. `lakebase_vector must be loaded via shared_preload_libraries`. Not left installed |

Installed after the probe: `btree_gin` 1.3, `pg_trgm` 1.6, `plpgsql` 1.0, `uuid-ossp` 1.1, `vector` 0.8.6.

Schema `spec165_probe` and its one-row table remain, so the HNSW result can be repeated.

## What was not run

- `edgequake migrate` and `edgequake` against this database.
- An App deploy.
- Scale-to-zero during an advisory lock or `LISTEN`.
- A second embedding model (BGE, Qwen3). Dimension 1024 is measured for GTE large only.
- Postgres 17.
