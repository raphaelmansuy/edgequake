---
title: Architecture
description: Entry page for the EdgeQuake architecture docs, with a short map of how the system fits together and links to each topic.
---

# Architecture

These pages explain how EdgeQuake is built: what runs, how data moves, and where it is stored. Start here if you want to understand the system before you change it or run it in production.

EdgeQuake is a Rust service that turns documents into a knowledge graph plus vectors, then answers questions from both. The released product is v0.32.2. Parts marked v0.33.0 are at repo HEAD and not yet released.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    user["User or SDK"] --> web["WebUI"]
    user --> api["REST API"]
    web --> api
    api --> engine["Ingestion and query engines"]
    engine --> db["PostgreSQL"]
    engine --> llm["LLM providers"]
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class user,web eqActor
class db eqStore
class llm eqLlm
```

Read it left to right. Everything goes through the API. The engines use PostgreSQL for storage and an LLM provider for extraction and answers.

## Where to go next

| I want to know... | Read |
| ----------------- | ---- |
| What the pieces are and how they deploy | [Overview](./overview.md) |
| How a document becomes knowledge | [Data flow](./data-flow.md) |
| How a question becomes an answer | [Query flow](./query-flow.md) |
| Where data is stored and which tables exist | [Storage model](./storage-model.md) |
| Full PostgreSQL E/R diagrams by domain | [Schema E/R diagrams](../data-layer/schema-er.md) |
| How tenants, auth, and LLM provider choice work | [Tenancy and providers](./tenancy-and-providers.md) |
| How to trace an answer to its source | [Lineage tracking](./lineage-tracking.md) |
| What each crate does | [Crate list](./crates/index.md) and [crate dependency graphs](./crates/README.md) |
| Table names, indexes, and SQL | [Data layer deep dive](../deep-dives/data-layer.md) |
| How cancel, fairness, and restart work | [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md) |

## Facts to remember

- The workspace has 15 Rust crates under `edgequake/crates/`. LLM provider clients come from the external `edgequake-llm` crate.
- PostgreSQL is required. It holds relational data, vectors (pgvector), and the graph (Apache AGE). There is no in-memory server mode.
- Only `edgequake migrate` changes the database schema. The API never does.
- Each workspace has its own settings, embedding model, and vector table.
