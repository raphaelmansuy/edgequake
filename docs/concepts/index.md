---
title: Core Concepts
description: The ideas behind EdgeQuake's Graph-RAG approach - knowledge graphs, entity extraction, query modes, and decision extraction.
---

> **Released: v0.32.2** · Contract: [OpenAPI snapshot](../../edgequake_webui/openapi/openapi.snapshot.json) · Ops: [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md)

# Core Concepts

These pages explain the ideas that EdgeQuake is built on. They are for anyone who wants to understand why the system behaves the way it does before they tune it.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    A["Graph-RAG"] --> B["Entity extraction"]
    B --> C["Knowledge graph"]
    C --> D["Hybrid retrieval"]
    B --> E["Decision extraction"]
    E --> C
```

Read the chart left to right. Entity extraction builds the knowledge graph. Hybrid retrieval reads it. Decision extraction is an optional second way to build the graph.

## Read in this order

1. [Graph-RAG](graph-rag.md): why a graph helps retrieval-augmented generation.
2. [Entity extraction](entity-extraction.md): how text becomes entities and relationships.
3. [Knowledge graph](knowledge-graph.md): how the graph is stored and isolated per tenant.
4. [Hybrid retrieval](hybrid-retrieval.md): how a question is answered with vectors and graph together.
5. [Decision extraction](decision-extraction.md): preview mode that answers closed questions with a small local model (SPEC-160).
6. [Feature tour](feature-tour.md): one page that lists what the product can do.

## Operational concepts

- [Pipeline progress](../deep-dives/pipeline-progress.md): task IDs, WebSocket and SSE progress, `display_status` and `ui_phase`.
- [PDF processing](../deep-dives/pdf-processing.md): vision convert, multimodal assets, the convert-then-ingest split, and the `cancelled` status.
- [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md): cancel behaviour, tenant fairness, cooperative abort.
