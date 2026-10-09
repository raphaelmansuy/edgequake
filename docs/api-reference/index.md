---
title: API Reference
description: Map of the EdgeQuake REST API for v0.32.x and the v0.33.0 additions. Start here to find the right page for documents, queries, graph, tasks, connections and more.
---

This section describes the EdgeQuake HTTP API. It is for developers who call EdgeQuake from their own code, or who write an SDK. Product pin: **v0.32.2**. Pages also cover the **v0.33.0** additions (provider Connections, `POST /api/v1/providers/test`, honest `/health`). Those are marked "v0.33.0" on each page.

**Source of truth.** The running server publishes its own contract at `/api-docs/openapi.json` and a Try-it-out UI at `/swagger-ui/` (default `http://localhost:8080`). A committed copy of the v0.32.2 contract is in [`openapi.snapshot.json`](../../edgequake_webui/openapi/openapi.snapshot.json). It does not yet include the v0.33.0 routes. Where this documentation and the server disagree, trust the server.

## Pick a page

| I want to... | Read |
|--------------|------|
| Learn auth, headers, errors, pagination, rate limits | [REST API](rest-api.md#conventions) |
| Upload text, files or PDFs and follow progress | [Document upload](document-upload-quick-reference.md) |
| Ask questions, stream answers, chat | [REST API: Query and Chat](rest-api.md#query) |
| Read or edit the knowledge graph | [REST API: Graph](rest-api.md#graph) |
| Manage tasks, queue metrics, costs, tenants, workspaces | [Extended API](extended-api.md) |
| Trace an answer back to chunks and documents | [Lineage endpoints](lineage-endpoints.md) |
| Save provider endpoints and test them (v0.33.0) | [Connections and provider test](connections.md) |
| Use EdgeQuake from an AI agent | [MCP integration](../../mcp/README.md) |
| Use a typed client library | [SDKs](../sdks/README.md) |

## How the API is organised

All resource routes live under `/api/v1`. Health probes, WebSockets, the MCP gateway and OAuth are served from the root. The Ollama emulation lives under `/api`.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    C["Client"] --> H["Health: /health /ready /live"]
    C --> V1["REST: /api/v1"]
    C --> V2["Jobs: /api/v2"]
    C --> WS["WebSocket: /ws"]
    C --> MCP["MCP gateway: /mcp"]
    C --> OL["Ollama emulation: /api"]
    V1 --> D["Documents and PDFs"]
    V1 --> Q["Query and Chat"]
    V1 --> G["Graph and Lineage"]
    V1 --> T["Tasks and Pipeline"]
    V1 --> A["Tenants and Workspaces"]
    V1 --> P["Models, Settings, Connections"]
    V1 --> U["Auth, Users, API keys"]
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class C,MCP eqActor
class OL,P eqLlm
```

Read the diagram from left to right: a client picks a front door, and `/api/v1` fans out into the resource areas listed in the table above.

## What a typical integration does

A typical client authenticates, picks a workspace, uploads a document, waits for the background task to finish, then queries.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
    participant App as Your app
    participant API as EdgeQuake API
    participant Q as Task queue
    App->>API: POST /api/v1/auth/login
    API-->>App: access_token
    App->>API: POST /api/v1/documents/upload
    API-->>App: 202 with task_id and document_id
    API->>Q: queue the work
    loop until a terminal status
        App->>API: GET /api/v1/tasks/{task_id}
        API-->>App: status pending, processing, indexed
    end
    App->>API: POST /api/v1/query
    API-->>App: answer and sources
```

Read it top to bottom. Uploads return at once; the server does the heavy work in the background, so you poll the task (or listen on a WebSocket) before you query.

## Pages in this section

- [REST API](rest-api.md): conventions, health, documents, parse, query, chat, graph, conversations, knowledge injection, models and settings.
- [Document upload](document-upload-quick-reference.md): which upload endpoint to use, with copy-paste examples.
- [Extended API](extended-api.md): tasks, progress streams, pipeline, costs, tenants, workspaces, Ollama emulation.
- [Lineage endpoints](lineage-endpoints.md): provenance for documents, chunks and entities.
- [Connections and provider test](connections.md): saved provider endpoints, probes, per-role model routing (v0.33.0).

Related: [Ingestion cancel and fairness](../ingestion-cancel-and-fairness.md), [Decision extraction](../concepts/decision-extraction.md), [Environment variables](../operations/env-reference.md).
