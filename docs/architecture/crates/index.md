---
title: Crate Reference
description: The 15 Rust crates in the EdgeQuake workspace, what each one does, and how they depend on each other.
---

This page lists every crate in the EdgeQuake workspace and the one job each crate has. Read it when you need to find where code lives. For the dependency graphs and per-crate notes, see the [detailed reference](./README.md).

The workspace lives in `edgequake/crates/`. A root package, `edgequake`, builds the server binary.

| Crate | Job |
| ----- | --- |
| `edgequake-api` | Axum HTTP server, WebSocket progress, OpenAPI, MCP, background task processor |
| `edgequake-core` | Orchestration: the `EdgeQuake` facade, workspace and tenant services, LLM role resolution |
| `edgequake-pipeline` | Ingestion: chunking, extraction, embedding, graph merge, lineage |
| `edgequake-query` | Query engine: six query modes, keyword extraction, context building |
| `edgequake-storage` | Storage traits and the PostgreSQL adapters (KV, pgvector, Apache AGE) |
| `edgequake-storage-contracts` | Driver-free types shared by storage providers (scopes, ids, projections) |
| `edgequake-tasks` | Task queue, worker pool, lease, cancel, tenant fairness |
| `edgequake-pdf` | PDF to Markdown (vision LLM or EdgeParse), page assets |
| `edgequake-auth` | JWT, API keys, OIDC settings, roles, tenant types |
| `edgequake-audit` | Audit events and PostgreSQL audit log |
| `edgequake-rate-limiter` | Token-bucket rate limiting per tenant |
| `edgequake-observability` | Tracing, Prometheus metrics, OpenTelemetry, Langfuse |
| `edgequake-secrets` | AES-256-GCM encryption for stored API keys **(v0.33.0)** |
| `edgequake-migrate-manifest` | Parsed `manifest.toml` that describes migrations (SPEC-150) |
| `edgequake-fake-llm` | Test-only fake LLM server for hermetic proofs **(v0.33.0)** |

Two crates come from crates.io, not from this repository:

- `edgequake-llm` provides the LLM and embedding provider traits and clients.
- `edgequake-pdf2md` converts PDF pages to Markdown. `edgequake-pdf` wraps it.

There is no `edgequake-graph` crate. Graph code lives in `edgequake-storage` and `edgequake-query`.

## Which crate owns a change

Read the diagram top to bottom. Pick the question that matches your change, then open the crate it points to.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    change["What are you changing?"] --> q1["New HTTP route or handler"]
    change --> q2["Chunking, extraction, or merge"]
    change --> q3["Retrieval or answer mode"]
    change --> q4["Table, vector, or graph access"]
    change --> q5["Upload queue, worker, or cancel"]
    change --> q6["PDF to Markdown conversion"]
    change --> q7["Login, API key, or role"]
    q1 --> c1["edgequake-api: routes.rs and handlers/"]
    q2 --> c2["edgequake-pipeline"]
    q3 --> c3["edgequake-query"]
    q4 --> c4["edgequake-storage"]
    q5 --> c5["edgequake-tasks"]
    q6 --> c6["edgequake-pdf"]
    q7 --> c7["edgequake-auth"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class q4,q5,c4 eqStore
```
