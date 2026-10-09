---
title: 'Query Flow'
description: Step-by-step view of how EdgeQuake answers a question in each of its six query modes, with one sequence diagram per mode.
---

# Query Flow

This page shows what happens between a question arriving and an answer leaving. It is for developers who change retrieval, and for users who want to pick the right query mode.

The code lives in `edgequake-query` (`engine_impl/query_entry/query_pipeline.rs` and `engine_impl/modes/`). The HTTP entry points are `POST /api/v1/query`, `POST /api/v1/query/stream`, and `POST /api/v1/chat/completions`.

---

## The shared pipeline

Every mode runs the same four stages. Only the retrieval stage differs.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    prep["1. Prepare"] --> ret["2. Retrieve (per mode)"]
    ret --> post["3. Post-process"]
    post --> fin["4. Finalize"]
    byp["Bypass mode"] -.-> fin
```

Read it left to right. Bypass skips straight to the final stage.

| Stage | What happens |
| ----- | ------------ |
| Prepare | The question is embedded. In parallel, an LLM extracts keywords: **high-level** (themes) and **low-level** (specific names). Keywords are cached for 24 hours. If the request supplies keywords, the keyword LLM call is skipped. |
| Retrieve | The mode decides which stores to search. See the diagrams below. |
| Post-process | Filter to requested document ids, drop low-relevancy items, rerank, sort, and trim to the token budget. |
| Finalize | The LLM writes the answer from the context. An answer cache may return a stored answer first, and a sample of answers is checked for faithfulness. |

Engine defaults: up to 60 entities, 60 relationships, and 20 chunks; a 30,000 token context budget; graph depth 2; minimum score 0.1; reranking on with the top 20 kept. Per-request settings can override these.

---

## Which mode to use

| Mode | Best for | Searches |
| ---- | -------- | -------- |
| `naive` | A specific fact in the text | Chunk vectors only |
| `local` | A question about named things | Entity vectors, then the graph around them |
| `global` | A broad theme | Relationship vectors |
| `hybrid` | Mixed questions | Local, global, and naive, interleaved |
| `mix` (default) | General use | Local, global, and naive, blended by score |
| `bypass` | Chat with no documents | Nothing. The LLM answers alone. |

The names differ slightly from LightRAG. In EdgeQuake, `hybrid` includes the naive arm and interleaves round-robin; `mix` blends all three arms with weights or reciprocal rank fusion (RRF). Always send an explicit `mode` when you compare systems.

---

## Naive mode

Naive mode is plain vector search over chunks. It is fast and has no graph step.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
    participant U as User
    participant E as Query engine
    participant V as Vector store
    participant L as LLM
    U->>E: Question, mode naive
    E->>V: Search chunk vectors
    V-->>E: Top chunks
    E->>E: Filter, rerank, trim
    E->>L: Question and chunks
    L-->>U: Answer with sources
```

Read it top to bottom. One search feeds the LLM directly.

## Local mode

Local mode starts from entities that match the specific names in the question, then walks the graph around them.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
    participant U as User
    participant E as Query engine
    participant V as Vector store
    participant G as Graph
    participant L as LLM
    U->>E: Question, mode local
    E->>E: Extract low-level keywords
    E->>V: Search entity vectors
    V-->>E: Seed entities
    E->>G: Expand neighbors, depth 2
    G-->>E: Entities and relationships
    E->>L: Question and graph context
    L-->>U: Answer with sources
```

Read it top to bottom. The keywords pick the seeds, and the graph adds their neighbors.

## Global mode

Global mode answers broad questions. It searches relationship descriptions, not community summaries.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
    participant U as User
    participant E as Query engine
    participant V as Vector store
    participant G as Graph
    participant L as LLM
    U->>E: Question, mode global
    E->>E: Extract high-level keywords
    E->>V: Search relationship vectors
    V-->>E: Top relationships
    E->>G: Load the entities on those edges
    G-->>E: Entities
    E->>L: Question and relationship context
    L-->>U: Answer with sources
```

Read it top to bottom. If the vector search finds nothing, the engine falls back to the highest-degree relationships.

Global mode can optionally expand to entities in the same index-time community, and add extractive community reports when `EDGEQUAKE_COMMUNITY_REPORTS` is on. This is not the same as Microsoft GraphRAG, which builds a hierarchy of LLM-written community reports. See [EdgeQuake vs GraphRAG](../comparisons/vs-graphrag.md).

## Hybrid mode

Hybrid mode runs local, global, and naive retrieval, then interleaves the results one by one.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
    participant U as User
    participant E as Query engine
    participant A as Local, global, naive arms
    participant L as LLM
    U->>E: Question, mode hybrid
    E->>A: Run the arms
    A-->>E: Three result lists
    E->>E: Interleave round-robin, remove duplicates
    E->>L: Question and merged context
    L-->>U: Answer with sources
```

Read it top to bottom. The arms are chosen using the question's intent, so an arm that does not fit may be skipped.

## Mix mode

Mix mode is the default. It runs the same three arms in parallel, then scores and blends them.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
    participant U as User
    participant E as Query engine
    participant A as Local, global, naive arms
    participant L as LLM
    U->>E: Question, mode mix
    par Each arm in parallel
        E->>A: Local search
        E->>A: Global search
        E->>A: Naive search
    end
    A-->>E: Scored results
    E->>E: Normalize scores, apply weights or RRF
    E->>L: Question and blended context
    L-->>U: Answer with sources
```

Read it top to bottom. Weights come from `EDGEQUAKE_MIX_LOCAL_WEIGHT`, `EDGEQUAKE_MIX_GLOBAL_WEIGHT`, and `EDGEQUAKE_MIX_NAIVE_WEIGHT`. A slow arm has a time limit, so it cannot stall the whole query.

## Bypass mode

Bypass mode skips retrieval. Use it for plain chat or to test the LLM connection.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
    participant U as User
    participant E as Query engine
    participant L as LLM
    U->>E: Question, mode bypass
    E->>L: Question and chat history
    L-->>U: Answer, no sources
```

Read it top to bottom. There is no search step and no sources are returned.

---

## Streaming

`POST /api/v1/query/stream` and `/api/v1/chat/completions/stream` follow the same stages. Preparation, retrieval, and post-processing finish first. Then the LLM answer is sent as it is produced. The streaming path can use the role-specific models described in [Tenancy and providers](./tenancy-and-providers.md#querying-the-answer-model).

## Context-only retrieval

`POST /api/v1/query/context` runs stages 1 to 3 and returns the retrieved context without calling the answer LLM. Use it when another system writes the answer.

## See also

- [Data flow](./data-flow.md): how the data got into the stores
- [Storage model](./storage-model.md)
- [Query modes deep dive](../deep-dives/query-modes.md)
- [Lineage tracking](./lineage-tracking.md): from an answer back to the source
