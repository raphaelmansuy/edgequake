# SPEC-165 — Databricks Apps, Lakebase, and AI Gateway

> **Status:** Study. No product code in this spec.  
> **Probe date:** 2026-10-21 (UTC).  
> **Workspace:** `https://dbc-cb61f1e1-bd8e.cloud.databricks.com`  
> **Lakebase project left in place:** `edgequake-spec165-probe` (Postgres 18, branch `production`, endpoint `primary`).  
> **Parent graph decision:** [SPEC-164](../164-pure-postgres-study/README.md) already ships the plain graph (`eq_graph_nodes` / `eq_graph_edges`).

## Result in one paragraph

EdgeQuake can store vectors and the plain graph on Lakebase Postgres 18. The same workspace can serve chat and embeddings through AI Gateway. The current server cannot be uploaded as a Databricks App source tree. The database password expires. Some Gateway models return chat `content` as a list, and the OpenAI-compatible client accepts only a string. Embedding vectors from the model that answered are 1024 dimensions.

## What this study measured

| Claim | Evidence |
|-------|----------|
| Postgres 18.6, pgvector 0.8.6, `halfvec` cosine HNSW | [07](07-derisk-results.md) |
| `CREATE EXTENSION age` is rejected | [07](07-derisk-results.md) |
| Project owner is not a superuser and can install `vector` | [07](07-derisk-results.md) |
| `uuidv7()` exists | [07](07-derisk-results.md) |
| AI Gateway embedding `system.ai.gte-large-en` returns 1024 floats | [07](07-derisk-results.md) |
| AI Gateway chat `system.ai.llama-4-maverick` returns a string | [07](07-derisk-results.md) |
| AI Gateway chat `system.ai.gpt-oss-20b` returns `content` as a list | [07](07-derisk-results.md) |
| EdgeQuake binary on an App, and `edgequake migrate` on this project | Not run |

## Reading order

1. [00 Why](00-why.md) · [01 First principles](01-first-principles.md)
2. [02 Platform facts](02-platform-facts.md) · [03 Code as it is](03-code-as-is.md)
3. [04 Gap matrix](04-gap-matrix.md) and the [lenses](lenses/)
4. [05 Target architecture](05-target-architecture.md) · [06 Implementation plan](06-implementation-plan.md)
5. [07 Probe results](07-derisk-results.md) · [08 Risks](08-risks.md) · [09 Cross-reference](09-cross-ref.md)

## Success criteria for a later implementation

| ID | Criterion | Bound |
|----|-----------|-------|
| S1 | `edgequake migrate` finishes on `edgequake-spec165-probe` | Postgres 18, no AGE |
| S2 | Serve stays up for more than one credential lifetime | Password refreshed per new connection |
| S3 | One chat completion and one embedding through AI Gateway from the server | String `content`, dimension 1024 |
| S4 | App process listens on `DATABRICKS_APP_PORT` | Binary loaded from a Volume, source files ≤ 10 MB |
| S5 | Health reports pgvector ≥ 0.8.2 | Same floor as today |

## Non-goals

- A second graph engine. SPEC-164 is the graph.
- A second HTTP client for models.
- Per-user Unity Catalog enforcement in v1.
- Replacing the Docker image that operators run outside Databricks.
- Storing the probe token, a database password, or a connection string in git.

## Secret handling

The personal access token used for the probe is not in this tree. Rotate that token. It was pasted into a chat. The probe project remains. Its compute suspends after 300 seconds of idle time.
