# 02 — Platform facts

Parent: [README](README.md) · Previous: [01 First principles](01-first-principles.md) · Next: [03 Code as it is](03-code-as-is.md)

Facts below come from Databricks docs (date on the page) or from the 2026-10-21 probe ([07](07-derisk-results.md)). A row marked **Doc** was not re-executed in the workspace. A row marked **Probe** was.

## Apps runtime

| Fact | Source | Kind |
|------|--------|------|
| OS Ubuntu 22.04. Python 3.11. Node.js 22.16. `uv` 0.10.2 | [Apps environment](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/system-env), 11 Sep 2026 | Doc |
| Deploy installs Python (`requirements.txt` or `uv`) and, when `package.json` exists, Node. Then it runs `command` from `app.yaml` | [Deploy](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/deploy), 23 Sep 2026 | Doc |
| `command` is not a shell. `DATABRICKS_APP_PORT` is expanded in the command | [app.yaml](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/app-runtime), 11 Sep 2026 | Doc |
| Each file in the app directory ≤ 10 MB | [Develop apps](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/app-development), 11 Sep 2026 | Doc |
| Compute: Medium 2 vCPU / 6 GB, Large 4 vCPU / 12 GB, XLarge 12 vCPU / 48 GB | [Compute size](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/compute-size), 25 Sep 2026 | Doc |
| Injected env: `DATABRICKS_HOST`, `DATABRICKS_APP_PORT`, `DATABRICKS_CLIENT_ID`, `DATABRICKS_CLIENT_SECRET` | [Apps environment](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/system-env) | Doc |
| User token header: `x-forwarded-access-token`. Also `X-Forwarded-Email`, `X-Forwarded-User` | [Authorization](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/auth), 9 Oct 2026 · [HTTP headers](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/http-headers), 11 Sep 2026 | Doc |
| This workspace had no apps (`GET /api/2.0/apps` → `{}`) | Probe | Probe |

Custom containers on this platform are for cluster compute ([Container Services](https://docs.databricks.com/aws/en/compute/custom-containers)), not for the Apps source deploy described above.

## Lakebase

| Fact | Source | Kind |
|------|--------|------|
| Autoscaling projects contain branches and endpoints. New provisioned instances stopped on 12 Mar 2026 | [Add a Lakebase resource](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/lakebase), 9 Oct 2026 | Doc |
| App resource env: `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, `PGSSLMODE`. `PGUSER` is the service principal client id. `PGPASSWORD` is not injected | Same page | Doc |
| Password is an OAuth token from `POST /api/2.0/postgres/credentials`. TTL 300–3600 seconds. Login checks expiry. Open sessions stay open | [Authentication](https://docs.databricks.com/aws/en/oltp/projects/authentication) · [Generate credential](https://docs.databricks.com/api/postgres/v1/generate-database-credential.md) | Doc |
| Built-in PgBouncer pool does not accept OAuth | [Authentication](https://docs.databricks.com/aws/en/oltp/projects/authentication) | Doc |
| TLS required (`sslmode=require`) | [Connection strings](https://docs.databricks.com/aws/en/oltp/projects/connection-strings), 11 Sep 2026 | Doc |
| No superuser. `databricks_superuser` replaces it. No tablespaces | [Compatibility](https://docs.databricks.com/aws/en/oltp/projects/compatibility), 13 Oct 2026 | Doc |
| Scale-to-zero drops temp tables, prepared statements, advisory locks, and `LISTEN` / `NOTIFY` | Same page | Doc |
| Extension table: `vector` 0.8.0 / 0.8.0 / **0.8.6** on PG 16 / 17 / 18. Also `uuid-ossp` 1.1, `pg_trgm` 1.6, `btree_gin` 1.3, `databricks_auth` 1.0. `pg_uuidv7` is absent. PG 18 has built-in `uuidv7()` | [Extensions](https://docs.databricks.com/aws/en/oltp/projects/extensions), 16 Oct 2026 | Doc |
| `lakebase_vector` 1.1.1 is listed after Lakebase Search is enabled. The index type is `lakebase_ann` | [lakebase_vector](https://docs.databricks.com/aws/en/oltp/projects/lakebase-vector), 15 Sep 2026 | Doc |
| Probe project `edgequake-spec165-probe`: Postgres **18.6**, endpoint `primary` **ACTIVE**, `max_connections` **225** | Probe | Probe |
| Installed by the owner role (not superuser): `vector` **0.8.6**, `pg_trgm` 1.6, `uuid-ossp` 1.1, `btree_gin` 1.3 | Probe | Probe |
| `halfvec(3)` plus HNSW `halfvec_cosine_ops` returned the inserted row | Probe | Probe |
| `uuidv7()` and `gen_random_uuid()` returned values. `pg_advisory_lock` / `unlock` returned | Probe | Probe |
| `age` is not in `pg_available_extensions`. `CREATE EXTENSION age` error: not in the allowed extensions list | Probe | Probe |
| `CREATE EXTENSION lakebase_vector` error: must be loaded via `shared_preload_libraries`. The extension is not installed | Probe | Probe |

The owner role has `rolsuper = false`, `rolcreatedb = true`, `rolcreaterole = true`.

## AI Gateway

| Fact | Source | Kind |
|------|--------|------|
| Unified chat: `POST /ai-gateway/mlflow/v1/chat/completions`. Embeddings: `POST /ai-gateway/mlflow/v1/embeddings`. Bearer is a Databricks token. Model example in the doc: `system.ai.gpt-5-6-sol` | [Query foundation and embedding models](https://docs.databricks.com/aws/en/ai-gateway/query-model-services), 16 Oct 2026 | Doc |
| External providers use `/ai-gateway/openai/v1/...` plus header `Databricks-Model-Provider-Service` | [Query external model providers](https://docs.databricks.com/aws/en/ai-gateway/query-model-provider-services), 16 Oct 2026 | Doc |
| App resource `valueFrom` for a Unity Gateway model service is the three-part name | [Environment variables](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/environment-variables), 9 Oct 2026 | Doc |
| Foundation endpoints whose names start with `databricks-` are marked legacy in the app resource picker | [Model serving resource](https://docs.databricks.com/aws/en/dev-tools/databricks-apps/model-serving), 2 Oct 2026 | Doc |
| Workspace had 12 READY serving endpoints, including `databricks-gte-large-en` and `databricks-llama-4-maverick` | Probe | Probe |
| `system.ai.gte-large-en` embeddings: HTTP 200, **1024** dimensions. `system.ai.gte_large_en_v1_5`: HTTP 404 | Probe | Probe |
| `system.ai.llama-4-maverick` and `system.ai.qwen3-next-80b-a3b-instruct`: HTTP 200, `content` is a string. Llama body contained `pong` | Probe | Probe |
| `system.ai.gpt-oss-20b`: HTTP 200, `content` is a list of parts, part type `reasoning` only (at `max_tokens` 24) | Probe | Probe |
| `system.ai.gemma-3-12b-it` and `system.ai.meta_llama_v3_1_8b_instruct`: HTTP 404, model does not exist | Probe | Probe |
| Name `databricks-meta-llama-3-1-8b-instruct` on the Gateway path: HTTP 403, "no longer available. Use Unity Catalog model services." | Probe | Probe |
| `POST /serving-endpoints/databricks-gemma-3-12b/invocations`: HTTP 200, `content` is a string | Probe | Probe |

Doc embedding sizes, for models that also have READY endpoints here: GTE large and BGE large are 1024 dimensions ([supported models](https://docs.databricks.com/aws/en/machine-learning/foundation-model-apis/supported-models)). Qwen3-Embedding-0.6B is configurable up to 1024. This probe called GTE only.
