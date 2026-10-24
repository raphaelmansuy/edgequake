# 03 — Code as it is

Parent: [README](README.md) · Previous: [02 Platform facts](02-platform-facts.md) · Next: [04 Gap matrix](04-gap-matrix.md)

Paths match the tree this study was written against.

## Boot and listen address

`DATABASE_URL` is required (`async_main_postgres` in [`main.rs`](../../edgequake/src/main.rs)). The schema-gate bind uses `HOST` (default `0.0.0.0`) and `PORT` (default 8080), before storage setup.

Apps inject `DATABRICKS_APP_PORT`. They do not inject `PORT`. A launcher must copy the value.

## Pools

Four pools, default sizes 16 + 12 + 4 + 2 = 34. Each pool calls `PgPool::connect(url)` with `min_connections(1)`.

```52:58:edgequake/crates/edgequake-storage/src/adapters/postgres/pool_bundle.rs
    fn default_size(self) -> u32 {
        match self {
            Self::Query => 16,
            Self::Ingest => 12,
            Self::Queue => 4,
            Self::Admin => 2,
        }
    }
```

`connect_role` builds `PgPoolOptions` with `min_connections(1)`, `max_lifetime(pool_max_lifetime())`, and then calls `.connect(url)` ([`pool_bundle.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/pool_bundle.rs)).

`pool_max_lifetime` defaults to 1800 seconds (`DEFAULT_POOL_MAX_LIFETIME_SECS` in [`connection.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs)). sqlx 0.8 keeps the password that was in the URL when the pool was built. There is no per-connection password callback on this path.

The probe server reported `max_connections = 225`. Thirty-four sessions fit. Four pools times `min_connections(1)` means four logins at start, and more logins whenever a connection exceeds 1800 seconds.

## Extensions and AGE

`setup_extensions` installs `vector` first. When the cutover ledger is ready, or `age_absent` is set, it returns without creating AGE (`age_absent || plain_ready` in [`connection.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs)). On a fresh database the ledger is missing, so `CREATE EXTENSION age` runs. When that statement fails and `pg_namespace` has no `ag_catalog`, the function logs `Continuing on SPEC-164 plain graph tables` and does not fail the boot.

On a fresh database the ledger does not exist yet, so the `CREATE EXTENSION age` branch runs. Lakebase returns an error and has no `ag_catalog`. The branch above then continues. That is the greenfield path. It is not a new storage type.

Migration 170 creates `eq_graph_nodes` and `eq_graph_edges`. Its header says a greenfield database with no `ag_catalog` marks the ledger ready. Plain reads use those tables:

Plain reads select from `public.eq_graph_nodes` ([`graph/plain/read.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/plain/read.rs)). The Rust type is still named `PostgresAGEGraphStorage`.

The type name is still `PostgresAGEGraphStorage`. The SQL for the plain path is not Cypher.

pgvector floor:

```35:37:edgequake/crates/edgequake-storage/src/adapters/postgres/capabilities.rs
/// CVE-safe floor (CVE-2026-3172 affected 0.8.0/0.8.1 parallel HNSW builds).
/// Prefer image pin >=0.8.5; readiness warns below this floor.
pub const PGVECTOR_MIN_CVE_SAFE: &str = "0.8.2";
```

`uuidv7()` is already probed at runtime ([`id_allocation.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/id_allocation.rs)). Postgres 18 on the probe has that function. `pg_uuidv7` is not required.

Docker init still attempts AGE ([`edgequake/docker/init-extensions.sql`](../../edgequake/docker/init-extensions.sql)). That file is for the image. Lakebase does not run it.

## Models

Embedding override already exists: `EDGEQUAKE_EMBEDDING_BASE_URL`, `EDGEQUAKE_EMBEDDING_API_KEY`, `EDGEQUAKE_EMBEDDING_DIMENSION` ([`provider_setup.rs`](../../edgequake/crates/edgequake-api/src/state/provider_setup.rs)). When the provider name is OpenAI-compatible and a base URL is set, the code calls `OpenAIProvider::compatible`.

The published crate `edgequake-llm` 0.10.9 builds:

- `{base}/chat/completions`
- `{base}/embeddings`

`MessageContent.content` is `Option<String>`. A JSON array in `content` fails deserialization. `extract_answer_content` clones that string. SPEC-163 stores `base_url` on a connection and can carry extra headers. The unified `system.ai.*` path does not need `Databricks-Model-Provider-Service`. The external-provider path does.

## Web UI

The API and the Next.js app are two processes. Apps expose one port. This study does not put the Web UI in v1.

## What this tree already gives Databricks

| Need | Already in the tree |
|------|---------------------|
| Plain graph without AGE | SPEC-164, migration 170, boot branch when `ag_catalog` is absent |
| pgvector cosine / halfvec | `capabilities.rs`, default storage mode halfvec |
| Separate embedding host and dimension | `provider_setup.rs` |
| OpenAI-compatible HTTP | `edgequake-llm` 0.10.9 |
| Listen on `0.0.0.0` | `HOST` default |

| Need | Not in the tree |
|------|-----------------|
| Mint a Lakebase credential per new connection | Pools take one URL |
| Map `DATABRICKS_APP_PORT` to `PORT` | Server reads `PORT` |
| Accept chat `content` as a list | Client expects a string |
| App launcher and Volume fetch | No `app.yaml` |
