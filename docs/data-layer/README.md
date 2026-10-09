---
title: "Data layer overview"
description: "How EdgeQuake stores documents, vectors, and the knowledge graph in one PostgreSQL database: core tables, write and read paths, isolation, and where the detailed pages live."
---

# Data layer overview

EdgeQuake keeps all of its data in one PostgreSQL database. Plain tables hold documents, chunks, tasks, and users. The pgvector extension holds embeddings. The Apache AGE extension holds the knowledge graph. This page is the map; the other pages go deep.

## Stack

| Component | Value | Source of truth |
|---|---|---|
| PostgreSQL | 16, 17, or 18 (image default: 18) | `edgequake/docker/extension-pins.sh` |
| pgvector (vector search) | 0.8.5 on all three majors. Anything below 0.8.2 is flagged as unsafe. | `extension-pins.sh`; checked at runtime by `edgequake-storage/src/adapters/postgres/capabilities.rs` |
| Apache AGE (graph) | 1.6.0 on PG16, 1.7.0 on PG17, 1.8.0 on PG18 | `extension-pins.sh` |
| Other extensions | `pg_trgm`, `btree_gin`, `uuid-ossp` | `edgequake/docker/init-extensions.sql` |
| Driver | `sqlx` 0.8, raw SQL, no ORM | `edgequake/Cargo.toml` |
| Schema writer | `edgequake migrate` only. The API never changes the schema. | [Upgrading](../operations/upgrading.md) |

`DATABASE_URL` is required. There is no in-memory mode. By default one database serves all three roles (relational, graph, vector). The code also has optional providers for other backends (SQLite, Neo4j, Qdrant, standalone pgvector). This section covers the default setup only.

## Where each kind of data lives

| Data | Where | Page |
|---|---|---|
| Tenants, workspaces, users, API keys | Plain tables | [postgres.md](./postgres.md) |
| Documents and their chunk text | `documents`, `chunks` | [postgres.md](./postgres.md) |
| Embeddings | `chunk_embeddings`, `entity_embeddings`, `relationship_embeddings`, `report_embeddings` | [pgvector.md](./pgvector.md) |
| Knowledge graph | One AGE graph, labels `Node` and `EDGE` | [age.md](./age.md) |
| Keyword search over chunks | `chunks.content_tsv` with a GIN index | [pgvector.md](./pgvector.md) |
| Background jobs | `tasks` (partitioned by month) | [postgres.md](./postgres.md) |
| LLM response cache | `llm_cache` | [llm-cache-scope.md](./llm-cache-scope.md) |
| Encrypted provider connections (SPEC-163) | `provider_connections` | [postgres.md](./postgres.md) |

The old per-workspace `eq_*_kv` and `eq_*_vectors` tables were dropped by migrations 125, 126, and 131. Current databases do not have them.

## Core tables

This diagram shows the main tables and how they link. Dashed lines are links by ID with no foreign key constraint. Read it from top to bottom: a tenant owns workspaces, a workspace owns documents, and documents split into chunks that carry embeddings.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
erDiagram
    tenants ||--o{ workspaces : owns
    workspaces ||..o{ documents : "scopes"
    documents ||..o{ chunks : "split into"
    chunks ||--o{ chunk_embeddings : "has vector"
    embedding_models ||--o{ chunk_embeddings : "produced by"
    chunks ||--o| chunk_serving_state : "visibility"
    workspaces ||..o{ entities : "scopes"
    workspaces ||..o{ relationships : "scopes"
    entities ||--o{ entity_embeddings : "has vector"
```

## How a document becomes data

A document goes through one write path. The pipeline commits chunks and embedding records in one transaction. It then merges entities and relationships into the graph. Last, it marks the chunks `ready` so queries can see them. This sentence introduces the diagram; read it left to right.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    A["Upload"] --> B["Task row in tasks"]
    B --> C["Worker claims task"]
    C --> D["Chunk and extract"]
    D --> E["One transaction: chunks and embeddings"]
    E --> F["Merge into AGE graph"]
    F --> G["Mark chunks ready"]
    G --> H["Visible to queries"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class E eqLlm
class F eqStore
```

Key points:

- The worker claims tasks with `FOR UPDATE SKIP LOCKED` and a lease, so two workers never take the same task.
- The transaction in step E also writes an idempotency record (`mutation_requests`), so a retry does not double-write.
- Step G sets `chunk_serving_state.state = 'ready'`. The serving fence (on by default) hides chunks that are not `ready`. See [serving-fence-decision.md](./serving-fence-decision.md).

## How queries read data

Each storage type has its own read path. All of them are scoped by tenant and workspace. The diagram shows the three paths side by side; start at "Query" and follow each branch.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    Q["Query"] --> V["Vector path"]
    Q --> K["Keyword path"]
    Q --> G["Graph path"]
    V --> V1["chunk_embeddings, HNSW index"]
    K --> K1["chunks.content_tsv, GIN index"]
    G --> G1["AGE Node and EDGE tables"]
    V1 --> F["Fence check: state is ready"]
    K1 --> F
    G1 --> R["Context for the answer"]
    F --> R
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class V1 eqLlm
class G1 eqStore
```

## Tenant and workspace isolation

- Every scoped table carries `tenant_id` and/or `workspace_id`.
- Scoped data-access transactions set the tenant context and switch to the `edgequake_tenant_access` role. That role cannot bypass Row-Level Security (RLS, rules that filter rows per session). The application layer also filters by scope, because not every query uses a scoped transaction.
- The AGE graph is shared. Isolation there comes from `tenant_id` and `workspace_id` properties on nodes and edges, plus application filters.

Details and the history of this decision: [rls-superuser-acceptance.md](./rls-superuser-acceptance.md).

## Page map

| Page | What it covers |
|---|---|
| [schema-er.md](./schema-er.md) | Full Mermaid E/R diagrams of every domain (tenancy, documents, PDF, graph read models, embeddings, tasks, durable writes, SSO, caches) |
| [postgres.md](./postgres.md) | Tables, tenancy, connection pools, timeouts, migrations, plain-SQL operation catalog |
| [pgvector.md](./pgvector.md) | Embedding tables, HNSW indexes, search tuning, vector operation catalog |
| [age.md](./age.md) | Graph model, Cypher and native SQL, graph operation catalog |
| [indexes.md](./indexes.md) | Which operations use which index |
| [complexity-matrix.md](./complexity-matrix.md) | Expected cost and failure modes per operation class |
| [00-inventory.md](./00-inventory.md) | What Ref IDs are and how many exist |
| [version-matrix.md](./version-matrix.md) | PG16, PG17, PG18 differences and test status |
| [version-matrix-results.md](./version-matrix-results.md) | One captured run of the test suites |
| [pg17-differential.md](./pg17-differential.md), [pg18-adoption.md](./pg18-adoption.md) | Decisions about version-specific features |
| [serving-fence-decision.md](./serving-fence-decision.md), [llm-cache-scope.md](./llm-cache-scope.md), [jsonb-envelope-acceptance.md](./jsonb-envelope-acceptance.md), [rls-superuser-acceptance.md](./rls-superuser-acceptance.md) | Design decisions |
| [improvements.md](./improvements.md) | Historical record of the SPEC-088 performance work |
| [benchmarks/](./benchmarks/README.md) | Plan templates per hot operation |

## Ref IDs and tests

Every operation in the original SPEC-088 inventory has a Ref ID such as `DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046`. IDs never change. They appear as `/* DATA-... */` comment prefixes on SQL, so you can find them in `pg_stat_statements`. Metrics use the same ID (`TimedStorageOp::start_dataop`). See [00-inventory.md](./00-inventory.md).

```bash
# Unit tests, no database needed
cargo test -p edgequake-storage --lib dataop

# Operation matrix and contract tests (need DATABASE_URL)
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
cargo test -p edgequake-storage --features postgres --test data_layer_ops_matrix -- --test-threads=4
cargo test -p edgequake-storage --features postgres --test e2e_spec088_improvements
cargo test -p edgequake-storage --features postgres --test e2e_spec060_age_expand_perf
cargo test -p edgequake-tasks --features postgres --test postgres_claim_lease -- --test-threads=1

# Check that the inventory, code constants, and spec docs agree
python3 specs/088-data-layer/scripts/lint_dataop_xref.py
```

Mission statement and original spec: [specs/088-data-layer/00-mission.md](../../specs/088-data-layer/00-mission.md).
