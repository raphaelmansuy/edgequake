# 00 — WHY

Parent: [README](README.md) · Next: [01 First principles](01-first-principles.md)

## The problem in one sentence

A Databricks workspace already has Postgres (Lakebase) and governed models (AI Gateway), and EdgeQuake still assumes a Docker host, a long-lived database password, and a 1536-dimension OpenAI embedding.

## Five WHYs

**WHY-165-1 — Why run inside the workspace at all?**  
The customer already governs tables, volumes, and models in one workspace. A second Postgres and a second model key are a second control plane. Lakebase is Postgres. AI Gateway speaks the OpenAI chat and embeddings HTTP shape. Those two facts are the reason to try.

**WHY-165-2 — Why is `DATABASE_URL` plus an OpenAI key not enough?**  
Four host rules differ from Docker.

| Rule | Docker today | Databricks |
|------|----------------|------------|
| Process | Any binary, any port | One process on `DATABRICKS_APP_PORT`. Each source file ≤ 10 MB |
| Database password | Static in `DATABASE_URL` | OAuth token, 5 to 60 minutes |
| Extensions | Image can include AGE | Allow-list. AGE is rejected. pgvector on Postgres 18 is 0.8.6 |
| Model URL and vectors | `api.openai.com`, often 1536 | `https://<workspace>/ai-gateway/mlflow/v1`, measured 1024 |

**WHY-165-3 — Why does the graph still matter if AGE is missing?**  
Lakebase rejects `CREATE EXTENSION age` ([07](07-derisk-results.md)). SPEC-164 already moved product hops to `eq_graph_nodes` and `eq_graph_edges`. Boot continues when `ag_catalog` is absent ([03](03-code-as-is.md)). The remaining graph work is to run that migrate on this project. It is not a new engine.

**WHY-165-4 — Why must the project be Postgres 18?**  
EdgeQuake treats pgvector below 0.8.2 as below the CVE-2026-3172 floor (`PGVECTOR_MIN_CVE_SAFE` in [`capabilities.rs`](../../edgequake/crates/edgequake-storage/src/adapters/postgres/capabilities.rs)). The Lakebase extension table ships pgvector 0.8.0 on Postgres 16 and 17, and 0.8.6 on Postgres 18 ([02](02-platform-facts.md)). The probe project is Postgres 18.6 and reports `vector` 0.8.6.

**WHY-165-5 — Why does a static API key with the 1536 default fail?**  
Gateway credentials expire. The chat client deserializes `content` as a string. `system.ai.gpt-oss-20b` returned a list. `system.ai.gte-large-en` returned 1024 floats. A workspace created at dimension 1536 cannot store those vectors in the existing column.

## Causal chain

```text
  one workspace
       │
       ├── Lakebase Postgres 18 ──► vectors + plain graph
       │         │
       │         └── password expires ──► refresh on each new connection
       │
       └── AI Gateway ──► chat string + 1024-d embeddings
                 │
                 └── some models return content as a list
       │
       ▼
  Apps runtime (one port, 10 MB files, Ubuntu 22.04)
       │
       ▼
  launcher + Volume binary + existing server
```

## What would make this study wrong

- A Postgres 17 Lakebase project whose pgvector build is already ≥ 0.8.2. The published table says 0.8.0. This study did not open a Postgres 17 project.
- A Gateway model pinned by the product that returns string `content` and 1536 dimensions. The models that answered here do not.
- An Apps runtime that accepts a custom container and files larger than 10 MB. The pages cited in [02](02-platform-facts.md) do not say that.
