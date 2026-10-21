# 06 — Implementation plan

Parent: [README](README.md) · Previous: [05 Target architecture](05-target-architecture.md) · Next: [07 Probe results](07-derisk-results.md)

This file is the plan for a later change. SPEC-165 does not implement it.

DRY: one migrate train, one OpenAI-compatible client, one graph (SPEC-164).  
SOLID: `DbCredential` is the only new port. Callers of pools and of `LLMProvider` stay as they are.

## Correction to an earlier draft of this plan

An early draft said WP-C builds a new relational graph because the server still required Apache AGE. The tree this study read already has the plain graph. [`connection.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs) continues when `CREATE EXTENSION age` fails and `ag_catalog` is missing. WP-C proves that path on Lakebase. It does not add a second engine.

## WP-A — Credential source

**Change.** Add `DbCredential` next to `connect_role`.

- `StaticUrl` wraps today's `PgPool::connect(url)`.
- `LakebaseOAuth` runs only when `EDGEQUAKE_DB_AUTH=lakebase_oauth`.
- On each new physical connection: client-credentials token from `DATABRICKS_HOST`, then `POST /api/2.0/postgres/credentials` with the endpoint resource name. Use the returned `token` as the password. Set `sslmode=require`.
- Keep `max_lifetime` below the TTL. Cache a token only until `expire_time` minus a margin, and still mint at connect time so a connection cannot outlive the cache by accident.
- Log the URL through `redact_database_url`.

**Do not.** Add a second pool crate. Switch Docker off the static URL.

**Done when.** A unit test with a fake token endpoint opens two connections and shows the second request after the first token is expired. A live opt-in test against `edgequake-spec165-probe` holds a pool past one TTL.

## WP-B — Postgres 18 profile

**Change.** Document the profile: Postgres 18, pgvector ≥ 0.8.2, extensions `vector`, `pg_trgm`, `uuid-ossp`, `btree_gin`, `no_suspension` for production.

**Do not.** Lower `PGVECTOR_MIN_CVE_SAFE`. Enable `lakebase_vector` in v1. The probe's `CREATE EXTENSION lakebase_vector` failed on `shared_preload_libraries`.

**Done when.** Readiness on the probe reports vector 0.8.6 and `uuidv7_available`. A Postgres 17 project is not in the profile.

## WP-C — Prove the plain graph

**Change.** Run the existing migrate train on `edgequake-spec165-probe` (or a clone). Confirm `eq_graph_cutover` is ready, `age` is not installed, and one node write plus `search_nodes` succeeds. Fix only boot or migrate defects that this run shows.

**Do not.** Add tables beside `eq_graph_nodes` / `eq_graph_edges`. Port Cypher.

**Done when.** S1 in the [README](README.md). Record the schema version in the WP-C proof note.

## WP-D — Gateway profile

**Change.**

- Base URL `https://<DATABRICKS_HOST>/ai-gateway/mlflow/v1` without a second trailing `/v1`.
- Chat model from an allow-list of string-content models. Seed it with `system.ai.llama-4-maverick` until a product owner picks another model that returns a string.
- Embedding model `system.ai.gte-large-en`, dimension 1024, via the existing `EDGEQUAKE_EMBEDDING_*` variables.
- Bearer from the same OAuth client as WP-A, refreshed before expiry.
- Optional later: teach `MessageContent.content` to accept a string or an array. Until that lands, `system.ai.gpt-oss-20b` stays off the profile. The probe showed `content` as a list of `reasoning` parts, which `Option<String>` will reject.

**Do not.** Call legacy `/serving-endpoints/<databricks-name>/invocations` as the supported path. That name on the Gateway URL returned 403.

**Done when.** S3. A contract test feeds a string body and an array body. The array body is rejected with a clear error until the parser grows.

## WP-E — App package

**Change.** `app.yaml` plus a launcher:

- `command` runs the launcher.
- `env` uses `valueFrom` for the Lakebase endpoint and the model service. No secret literals.
- Launcher sets `HOST`, `PORT`, and execs the Volume binary.
- Build the binary for Ubuntu 22.04 glibc.

**Do not.** Put the Web UI in this package. Put the binary in the App source tree.

**Done when.** S4 on a real App. This study did not create an App (`GET /api/2.0/apps` was empty).

## WP-F — Proof

**Change.** CI uses `edgequake-fake-llm` on `/v1/chat/completions` and `/v1/embeddings` with a string `content`. One opt-in job, secrets from the environment, hits this workspace: credential refresh, vector version, one chat, one embedding, dimension 1024.

**Do not.** Store the personal access token in CI. The App's own service principal is the identity under test.

## Order

WP-A and WP-D can land together because they share the OAuth client. WP-B is a short doc and readiness check. WP-C needs WP-A if migrate runs from the server against Lakebase. WP-E is last. WP-F tracks each package.

## Files a later change is allowed to touch

| File | Why |
|------|-----|
| [`pool_bundle.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/pool_bundle.rs) | Call `DbCredential` from `connect_role` |
| New module beside `connection.rs` | Lakebase token exchange |
| [`provider_setup.rs`](../../edgequake/crates/edgequake-api/src/state/provider_setup.rs) | Gateway profile defaults |
| `edgequake-llm` `MessageContent` | Only if WP-D takes the array-content follow-up |
| New `deploy/databricks/` | `app.yaml` and launcher |

Migrate SQL changes only if WP-C finds a statement Lakebase rejects.
