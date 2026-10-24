# LENS — Product

Parent: [04 Gap matrix](../04-gap-matrix.md)

## Who this is for

An operator who already runs a Databricks workspace and wants EdgeQuake in that workspace: Lakebase for state, AI Gateway for chat and embeddings, Apps for the process.

## v1 they can expect

- API on one App URL.
- One workspace, one embedding model, 1024 dimensions, chosen before the first document.
- Plain graph from SPEC-164. No Cypher.
- Models whose chat `content` is a string. The probe list is llama-4-maverick and qwen3-next-80b. `gpt-oss-20b` is visible and is not the default.
- Compute that does not scale to zero.
- No Web UI inside the App.

## v1 they must not be told

- That ingest speed on XLarge matches the Docker host used for SPEC-001.
- That `lakebase_vector` is on. The probe could not install it.
- That each Databricks user sees a different EdgeQuake tenant.
- That Postgres 17 is safe. The extension table says pgvector 0.8.0, under the CVE floor.

## Later

- Web UI on the same port, after a single-process design exists.
- List-shaped chat `content`, so `gpt-oss` can be selected.
- User header mapped through SPEC-158, with fail-closed behavior when the header is missing.
- Lakebase Search, after `shared_preload_libraries` is enabled on the project.

## Gaps

G12 and the open rows in [04](../04-gap-matrix.md). Product copy for operators belongs with WP-E, not in a second document set.
