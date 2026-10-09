---
title: Knowledge Graph
description: How EdgeQuake stores entities and relationships in PostgreSQL with Apache AGE and pgvector, and how tenants and workspaces are isolated.
---

> **Released: v0.32.2** · Contract: [OpenAPI snapshot](../../edgequake_webui/openapi/openapi.snapshot.json) · Ops: [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md)

# Knowledge Graph

The knowledge graph is where EdgeQuake keeps what it learned from your documents: entities as nodes and relationships as edges. This page explains what is stored, where, and how tenants stay separate. It is for developers and operators.

## What is in the graph

| Item | Meaning | Examples of fields |
|------|---------|--------------------|
| Node (entity) | A person, place, concept or other thing | `entity_name`, `entity_type`, `description`, `source_id`, `degree` |
| Edge (relationship) | A link between two entities | `src_id`, `tgt_id`, `relation_type`, `keywords`, `weight`, `description` |
| Properties | Extra data on nodes and edges | Descriptions, weights, source chunk IDs |

Entity names are normalized to upper case with underscores, for example `SARAH_CHEN`. Each node tracks the chunks it came from, so answers can cite their sources. You can read the graph through `GET /api/v1/graph/entities` and `GET /api/v1/graph/relationships`; both return `items`.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    A["SARAH_CHEN, PERSON"] -->|works at| B["QUANTUM_LAB, ORGANIZATION"]
    A -->|collaborates with| C["BOB_SMITH, PERSON"]
    C -->|works at| B
```

Read the chart as sentences. Each arrow is an edge with a relation label. The same pair of nodes can be reached by more than one path.

## Where the data lives

Everything is in one PostgreSQL database. There is no separate graph server and no in-memory mode.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    DB["PostgreSQL"] --> AGE["Apache AGE: nodes and edges"]
    DB --> VEC["pgvector: embeddings"]
    DB --> SQL["Tables: documents, chunks, tasks"]
    AGE --> Q["Query engine"]
    VEC --> Q
    SQL --> Q
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class DB,AGE,SQL eqStore
class VEC eqLlm
```

Read the chart from the top. The query engine combines the three stores.

| Store | Holds | Used for |
|-------|-------|----------|
| Apache AGE | Entity nodes and relationship edges | Graph traversal with Cypher |
| pgvector | Embeddings for chunks, entities and relationships (`vector` or `halfvec`) | Similarity search with HNSW indexes |
| Standard tables | Documents, chunk text, conversations, tasks | Filtering and metadata |

The vector size comes from the embedding model you choose, so there is no fixed width. See [Vector storage](../deep-dives/vector-storage.md) and [Graph storage](../deep-dives/graph-storage.md).

## Vector plus graph

A question usually uses all three stores:

1. **Vector search** finds entities and chunks that look like the question.
2. **Graph traversal** walks edges from those entities to find neighbors. The default walk is breadth-first (`bfs`). Personalized PageRank (`ppr`) is opt-in with `EDGEQUAKE_GRAPH_WALK=ppr`.
3. **Fusion** merges both results into the prompt for the model.

The [hybrid retrieval](hybrid-retrieval.md) page covers the modes in detail.

## Tenants and workspaces

EdgeQuake isolates data in two levels. A **tenant** is an organization. A **workspace** is a project inside a tenant. Documents, entities, relationships and vectors all belong to one workspace.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    T1["Tenant acme-corp"] --> W1["Workspace A"]
    T1 --> W2["Workspace B"]
    T2["Tenant globex"] --> W3["Workspace X"]
    W1 --> D1["Documents, graph, vectors"]
```

Read the chart from the tenant down. A workspace never sees data from another workspace or tenant.

Send `X-Tenant-ID` and `X-Workspace-ID` headers to choose the scope. If you omit them, the default tenant and the default workspace apply. Since v0.32.0, PostgreSQL row-level security enforces the scope inside the database, in addition to the application checks.

## Communities

EdgeQuake can group closely linked entities into communities with the Louvain algorithm. Detection is workspace-scoped, and it is skipped for graphs above 50,000 nodes. See [Community detection](../deep-dives/community-detection.md).

## Limits to know

| Limit | Value |
|-------|-------|
| Nodes returned by one `GET /api/v1/graph` call | 500 |
| Traversal depth for that call | 5 |
| Page size on list endpoints | 100 |

## Learn more

- [Entity extraction](entity-extraction.md): how entities are found.
- [Hybrid retrieval](hybrid-retrieval.md): how queries use the graph.
- [LightRAG algorithm](../deep-dives/lightrag-algorithm.md)

## Source code

- [Graph storage trait](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-storage/src/traits/graph.rs)
- [Vector storage trait](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-storage/src/traits/vector.rs)
- [PostgreSQL adapters](https://github.com/raphaelmansuy/edgequake/tree/edgequake-main/edgequake/crates/edgequake-storage/src/adapters/postgres)
