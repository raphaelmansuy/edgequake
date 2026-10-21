# 04 — Gap matrix

Parent: [README](README.md) · Previous: [03 Code as it is](03-code-as-is.md) · Next: [05 Target architecture](05-target-architecture.md)

Lenses: [platform](lenses/LENS-platform.md) · [storage](lenses/LENS-storage.md) · [model](lenses/LENS-model.md) · [security](lenses/LENS-security.md) · [product](lenses/LENS-product.md)

| ID | Requirement | Code today | Databricks fact | Gap | Work |
|----|-------------|------------|-----------------|-----|------|
| G1 | Postgres that passes the 0.8.2 pgvector floor | Floor in `capabilities.rs` | PG 18 probe: vector **0.8.6**. Doc: PG 16/17 ship **0.8.0** | Profile must select PG 18 | WP-B |
| G2 | `vector`, `pg_trgm`, `uuid-ossp`, `btree_gin` | Docker init and migrations | Owner role installed all four on the probe | None for install rights | WP-B |
| G3 | Product graph without AGE | SPEC-164 plain tables. Boot continues when `ag_catalog` is absent | `CREATE EXTENSION age` rejected. `age` not in `pg_available_extensions` | Migrate has not been run on this project | WP-C |
| G4 | Password lives for the process lifetime | One URL in `PgPool::connect`. Lifetime default 1800 s | Credential TTL 300–3600 s. Probe expiry honored 300 s | New connections after expiry use the old password | WP-A |
| G5 | `sslmode=require` | URL is caller-defined | Lakebase requires TLS | Launcher must set `sslmode=require` | WP-A |
| G6 | Advisory locks and `LISTEN` survive | Pools and cancel notify use them | Scale-to-zero drops session state | Production endpoint: `no_suspension` | WP-B |
| G7 | Chat and embeddings on one OpenAI-shaped client | `{base}/chat/completions` and `{base}/embeddings`. `content: Option<String>` | Gateway base works. Llama returns a string. `gpt-oss-20b` returns a list. GTE returns 1024 | Pin string models. Dimension 1024. List `content` is a parser gap | WP-D |
| G8 | Model names | UC registry uses underscores (`gte_large_en_v1_5`) | Gateway accepts `system.ai.gte-large-en`. Underscore name returned 404 | Document the hyphenated service name | WP-D |
| G9 | Bearer token for models | `OPENAI_API_KEY` or SPEC-163 secret | App has `DATABRICKS_CLIENT_ID` / `SECRET`. Workspace token expires | Refresh the bearer the same way as the database password | WP-D |
| G10 | One listening process | `HOST` + `PORT` (default 8080) | Apps set `DATABRICKS_APP_PORT` only | Launcher copies the port | WP-E |
| G11 | Ship the binary | Release image is a container | App file ≤ 10 MB. No custom App container | Binary in a Volume. Launcher downloads it | WP-E |
| G12 | Web UI | Separate Next.js server | One public port | Out of v1 | — |
| G13 | `lakebase_vector` / `lakebase_ann` | Product indexes are pgvector HNSW cosine | Extension listed. `CREATE` failed: `shared_preload_libraries` | v1 stays on pgvector. Search enablement is later | WP-B note |
| G14 | Identity | SPEC-158 OIDC and API keys | App service principal, plus optional user header | v1 uses the app identity for storage and models | WP-E |
| G15 | Pool headroom | Default 34 connections | Probe `max_connections` 225 | Fits. Re-check if the endpoint size changes the limit | WP-A |

## Closed by evidence, not by new code

- G2 install rights: measured.
- G1 on Postgres 18: measured. Postgres 17 remains unsupported because of the published 0.8.0 build.
- G3 design: SPEC-164. Only the migrate proof is open.
- G13: pgvector HNSW cosine on `halfvec` returned a row. `lakebase_vector` is not required for v1.

## Still open after this study

G4, G7 (parser and dimension pin), G9, G10, G11, and the migrate proof inside G3. Those are [06](06-implementation-plan.md).
