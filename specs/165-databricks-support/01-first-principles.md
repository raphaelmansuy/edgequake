# 01 — First principles

Parent: [README](README.md) · Previous: [00 Why](00-why.md) · Next: [02 Platform facts](02-platform-facts.md)

Each law names the code or the page that forces it.

## LAW-165-1 — One graph, the one SPEC-164 already shipped

Product hops read and write `public.eq_graph_nodes` and `public.eq_graph_edges` ([`graph/plain/read.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/plain/read.rs)). Migration 170 marks a greenfield database ready when `ag_catalog` is absent ([`170_spec164_plain_graph.sql`](../../edgequake/migrations/170_spec164_plain_graph.sql)). Databricks support uses that path. It does not add a third graph store.

## LAW-165-2 — A password is fetched when a connection opens

`connect_role` passes one URL into `PgPool::connect` ([`pool_bundle.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/pool_bundle.rs)). Lakebase accepts that password for at most 60 minutes (`POST /api/2.0/postgres/credentials`). The probe asked for 300 seconds and received an expiry. Docker keeps the static URL. Lakebase mode (`EDGEQUAKE_DB_AUTH=lakebase_oauth`) mints a credential inside the connect path. `app.yaml` stores resource keys, not secrets.

## LAW-165-3 — Models stay on the OpenAI-compatible client

`edgequake-llm` 0.10.9 posts to `{base}/chat/completions` and `{base}/embeddings`. The Gateway base that answered is `https://<workspace>/ai-gateway/mlflow/v1`. The client decodes `content` as `Option<String>` (`MessageContent` in `openai_compatible.rs`). A profile may set the base URL and the bearer token. A profile may not add a second chat stack. Models whose `content` is a list stay off the default profile until that struct accepts both shapes.

## LAW-165-4 — The binary is a Volume file

App source files are limited to 10 MB ([Develop apps](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/app-development), 11 Sep 2026). The server binary, with embedded pdfium, is the artifact in a Unity Catalog volume. The App directory holds `app.yaml` and a small launcher.

## LAW-165-5 — Postgres 18, TLS, and no scale-to-zero for the production project

The probe project is Postgres 18.6 with `sslmode=require`. pgvector 0.8.6 is above the 0.8.2 floor. Scale-to-zero drops advisory locks and `LISTEN` / `NOTIFY` ([Postgres compatibility](https://docs.databricks.com/aws/en/oltp/projects/compatibility), 13 Oct 2026). EdgeQuake uses advisory locks and a pool lifetime of 1800 seconds. Production compute sets `no_suspension`. The probe project uses a 300 second suspend timeout so it does not run all day.

## LAW-165-6 — Say which graph features are absent

AGE Cypher is absent on Lakebase. The plain tables cover the product hops SPEC-164 measured. This study did not run `edgequake migrate` on the probe, so schema readiness on Lakebase is a later proof (S1), not a claim of this document.

## LAW-165-7 — v1 uses the app identity

`DATABRICKS_CLIENT_ID` and `DATABRICKS_CLIENT_SECRET` belong to the app service principal ([Configure authorization](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/auth), 9 Oct 2026). That identity opens Lakebase and calls models. `x-forwarded-access-token` is the signed-in user, one request at a time. v1 does not map that header onto an EdgeQuake tenant. Background ingest has no user request, so it must use the app identity anyway.

## LAW-165-8 — Dimension is part of the workspace

`EDGEQUAKE_EMBEDDING_DIMENSION` must match the model. The measured Gateway embedding is 1024. The product default for `text-embedding-3-small` is 1536 ([`docs/operations/configuration.md`](../../docs/operations/configuration.md)). A Databricks workspace is created at 1024 and stays on that model.

## Rejected shortcuts

| Shortcut | Why it fails the law |
|----------|----------------------|
| Point `DATABASE_URL` at Lakebase and keep the pool for 30 minutes | LAW-165-2. New connections after expiry use the old password. Default `max_lifetime` is 1800 seconds. |
| Install AGE from source on Lakebase | LAW-165-6. The server rejected `CREATE EXTENSION age`. There is no superuser. |
| Build a new relational graph for SPEC-165 | LAW-165-1. SPEC-164 already did that. |
| Call `/serving-endpoints/<legacy-name>/invocations` as the only path | The legacy name on the Gateway path returned 403 and told the caller to use a Unity Catalog model service. Keep the Gateway base URL. |
| Upload the release binary in the App source | LAW-165-4. |
