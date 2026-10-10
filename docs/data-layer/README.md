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
| pgvector (vector search) | 0.8.5 on all three majors. Readiness warns below 0.8.2 (CVE floor). | `extension-pins.sh`; probed at runtime by `edgequake/crates/edgequake-storage/src/adapters/postgres/capabilities.rs` |
| Apache AGE (graph) | 1.6.0 on PG16, 1.7.0 on PG17, 1.8.0 on PG18 | `extension-pins.sh` |
| Other extensions | `pg_trgm`, `btree_gin`, `uuid-ossp` | `edgequake/docker/init-extensions.sql` |
| Driver | `sqlx` 0.8, raw SQL, no ORM | `edgequake/Cargo.toml` |
| Schema changes | Applied by `edgequake migrate`. The server does not change the schema by default; `EDGEQUAKE_SERVE_RECONCILE=1` is a one-release escape hatch. | [Upgrading](../operations/upgrading.md) |

`DATABASE_URL` is required. There is no in-memory mode. By default one database serves all three roles: relational, graph, and vector. The code also has adapters for other backends (memory, Neo4j, Qdrant, SQLite). This section covers the default PostgreSQL setup only.

## Where each kind of data lives

| Data | Where | Page |
|---|---|---|
| Tenants, workspaces, users, API keys | Plain tables | [postgres.md](./postgres.md) |
| Documents and their chunk text | `documents`, `chunks` | [postgres.md](./postgres.md) |
| Embeddings | `chunk_embeddings`, `entity_embeddings`, `relationship_embeddings`, `report_embeddings` | [pgvector.md](./pgvector.md) |
| Keyword search over chunks | `chunks.content_tsv` with a GIN index | [pgvector.md](./pgvector.md) |
| Knowledge graph | One AGE graph, labels `Node` and `EDGE` | [age.md](./age.md) |
| Background jobs | `tasks` (partitioned by month) | [postgres.md](./postgres.md) |
| LLM response cache | `llm_cache` | [llm-cache-scope.md](./llm-cache-scope.md) |
| Encrypted provider connections (SPEC-163) | `provider_connections` (ciphertext columns) | [postgres.md](./postgres.md) |

Migrations 125, 126, and 131 dropped the old per-workspace `eq_*_kv` and `eq_*_vectors` tables.

## Core tables

The diagram shows the main tables and how they link. Solid lines are identifying relationships (the child key includes the parent key). Dotted lines are optional scope links. Read it from top to bottom: a tenant owns workspaces, a workspace scopes documents, and documents split into chunks that carry embeddings.

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

A document goes through one write path. The pipeline commits chunks and embedding records in one transaction, merges entities and relationships into the graph, and then marks the chunks `ready` so queries can see them.

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

- The worker claims tasks with `FOR UPDATE SKIP LOCKED` and a lease, so two workers never take the same task.
- The transaction in step E also writes an idempotency record (`mutation_requests`), so a retry does not write the same work twice.
- Step G sets `chunk_serving_state.state = 'ready'`. The serving fence (on by default) hides chunks that are not `ready`. See [serving-fence-decision.md](./serving-fence-decision.md).

## How queries read data

Each storage type has its own read path, and all of them are scoped by tenant and workspace. Only the vector and keyword paths pass through the serving fence. The graph path reads AGE directly.

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

- Scoped tables carry `tenant_id`, `workspace_id`, or both.
- Scoped data-access transactions set the tenant context and switch to the `edgequake_tenant_access` role. That role has `NOBYPASSRLS`, so Row-Level Security (RLS, policies that filter rows per session) applies to it. The application layer also filters by scope, because not every query runs in a scoped transaction.
- The AGE graph is shared. Isolation there comes from `tenant_id` and `workspace_id` properties on nodes and edges, plus application filters.

For the details and the history of this decision, see [rls-superuser-acceptance.md](./rls-superuser-acceptance.md).

## Page map

| Page | What it covers |
|---|---|
| [schema-er.md](./schema-er.md) | Full Mermaid E/R diagrams of every domain (tenancy, documents, PDF, graph read models, embeddings, tasks, durable writes, SSO, caches) |
| [postgres.md](./postgres.md) | Tables, tenancy, connection pools, timeouts, migrations, plain-SQL operation catalog |
| [pgvector.md](./pgvector.md) | Embedding tables, HNSW indexes, search tuning, vector operation catalog |
| [age.md](./age.md) | Graph model, Cypher and native SQL, graph operation catalog |
| [indexes.md](./indexes.md) | Which indexes exist and which operations use them |
| [complexity-matrix.md](./complexity-matrix.md) | Expected cost and failure modes per operation class |
| [00-inventory.md](./00-inventory.md) | What Ref IDs are and how many exist |
| [version-matrix.md](./version-matrix.md) | PG16, PG17, PG18 differences and test status |
| [version-matrix-results.md](./version-matrix-results.md) | One captured run of the test suites |
| [pg17-differential.md](./pg17-differential.md), [pg18-adoption.md](./pg18-adoption.md) | Decisions about version-specific features |
| [serving-fence-decision.md](./serving-fence-decision.md), [llm-cache-scope.md](./llm-cache-scope.md), [jsonb-envelope-acceptance.md](./jsonb-envelope-acceptance.md), [rls-superuser-acceptance.md](./rls-superuser-acceptance.md) | Design decisions |
| [improvements.md](./improvements.md) | Historical record of the SPEC-088 performance work |
| [benchmarks/](./benchmarks/README.md) | Plan templates per hot operation |

## Ref IDs and tests

Every operation in the original SPEC-088 inventory has a Ref ID such as `DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046`. IDs never change. They appear as `/* DATA-... */` comment prefixes on SQL, so you can match a slow statement to its operation. Metrics use the same ID (`TimedStorageOp::start_dataop`). See [00-inventory.md](./00-inventory.md).

Run the unit tests (no database needed):

```bash
cargo test -p edgequake-storage --lib dataop
```

Run the operation matrix and contract tests (these need `DATABASE_URL`):

```bash
export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
cargo test -p edgequake-storage --features postgres --test data_layer_ops_matrix -- --test-threads=4
cargo test -p edgequake-storage --features postgres --test e2e_spec088_improvements
cargo test -p edgequake-storage --features postgres --test e2e_spec060_age_expand_perf
cargo test -p edgequake-tasks --features postgres --test postgres_claim_lease -- --test-threads=1
```

Check that the inventory, code constants, and spec docs agree:

```bash
python3 specs/088-data-layer/scripts/lint_dataop_xref.py
```

Mission statement and original spec: [specs/088-data-layer/00-mission.md](../../specs/088-data-layer/00-mission.md).
