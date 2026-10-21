# 09 — Cross-reference

Parent: [README](README.md) · Previous: [08 Risks](08-risks.md)

| Spec or code | What SPEC-165 uses from it | What SPEC-165 does not change |
|--------------|----------------------------|-------------------------------|
| [SPEC-164](../164-pure-postgres-study/README.md) | Plain `eq_graph_*` is the product graph. AGE is optional. Greenfield ledger is ready without `ag_catalog` | No new graph tables. WP-C only runs that train on Lakebase |
| [SPEC-164 decision](../164-pure-postgres-study/09-decision.md) | Hops do not need AGE for the measured complexity class | Does not re-benchmark hops |
| SPEC-042 / [`capabilities.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/capabilities.rs) | `PGVECTOR_MIN_CVE_SAFE = 0.8.2`. Cosine only. `halfvec` HNSW up to 4000 dimensions | Floor stays. Probe used `halfvec_cosine_ops` |
| SPEC-090 / [`pool_bundle.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/pool_bundle.rs) | Four pools, 34 connections, labeled `application_name` | WP-A changes how the password is supplied, not the four roles |
| [`connection.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs) | `pool_max_lifetime` default 1800 s. AGE skip when the ledger is ready or `ag_catalog` is absent | WP-A must stay under the Lakebase TTL |
| SPEC-150 | Schema gate and migrate lock. `PORT` bind in [`main.rs`](../../edgequake/src/main.rs) before serve | Lakebase `no_suspension` so a migrate advisory lock is not dropped by scale-to-zero |
| SPEC-158 | Product identity stays inside EdgeQuake | v1 does not map `x-forwarded-access-token` to a tenant |
| SPEC-163 / [`provider_setup.rs`](../../edgequake/crates/edgequake-api/src/state/provider_setup.rs) | Connection base URL, embedding base URL, dimension | Gateway profile is another base URL, not a new provider type |
| `edgequake-llm` 0.10.9 | `{base}/chat/completions`, `{base}/embeddings`, `content: Option<String>` | Array `content` is a follow-up inside WP-D, off by default |
| [`id_allocation.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/id_allocation.rs) | `uuidv7()` when the server has it | Probe confirmed the function on PG 18.6 |
| [Extensions doc](https://docs.databricks.com/aws/en/oltp/projects/extensions) | Allow-list and per-major pgvector versions | Probe confirmed the PG 18 column (0.8.6) and the AGE rejection |
| [AI Gateway query doc](https://docs.databricks.com/aws/en/ai-gateway/query-model-services) | `/ai-gateway/mlflow/v1` chat and embeddings | Probe confirmed the path, the 1024-d embedding, and the list-shaped chat on one model |

## ID index

| ID | Document |
|----|----------|
| WHY-165-1 .. WHY-165-5 | [00](00-why.md) |
| LAW-165-1 .. LAW-165-8 | [01](01-first-principles.md) |
| G1 .. G15 | [04](04-gap-matrix.md) |
| R1 .. R17 | [08](08-risks.md) |
| WP-A .. WP-F | [06](06-implementation-plan.md) |
| S1 .. S5 | [README](README.md) |
