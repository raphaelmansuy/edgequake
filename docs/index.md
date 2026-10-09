---
title: Documentation
description: Documentation for EdgeQuake, the Graph-RAG framework built in Rust. Install it, connect a model provider, ingest documents, and query them.
template: splash
hero:
  title: EdgeQuake Documentation
  tagline: Install, run, and operate EdgeQuake. Current release is v0.32.2.
  actions:
    - text: Get Started
      link: /docs/getting-started/
      icon: right-arrow
      variant: primary
    - text: API Reference
      link: /docs/api-reference/
      variant: minimal
---

> **Released: v0.32.2** · Contract: [OpenAPI snapshot](../edgequake_webui/openapi/openapi.snapshot.json) · Ops: [Ingestion cancel and fairness](ingestion-cancel-and-fairness.md)

EdgeQuake turns your documents into a knowledge graph and answers questions from it. It is written in Rust and stores everything in PostgreSQL. These pages are for developers and operators.

## Where to start

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    A["New here"] --> B["Getting Started"]
    B --> C["Providers"]
    C --> D["Concepts"]
    D --> E["Tutorials"]
    E --> F["API Reference"]
    B --> G["Operations"]
    G --> H["Deployment and auth"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class C eqLlm
```

Follow the arrows from the top. Operators can branch from Getting Started to Operations.

## Explore the documentation

- [Getting Started](getting-started/index.md): install EdgeQuake, run your first pipeline, understand the basics.
- [Providers](providers/index.md): connect Ollama, OpenAI, Anthropic, LM Studio, oMLX and other servers.
- [Core Concepts](concepts/index.md): knowledge graphs, entity extraction, query modes, and decision extraction (preview).
- [Architecture](architecture/index.md): the crates and the storage layer.
- [Tutorials](tutorials/index.md): step-by-step guides.
- [API Reference](api-reference/index.md): guided REST overlays. The full contract is the OpenAPI snapshot.
- [Deep Dives](deep-dives/index.md): progress streaming, PDF processing, storage.
- [Operations](operations/index.md): Docker, deployment, auth, upgrades, release process.
- [SDKs](sdks/README.md): official clients. SDK versions differ from the product version.
- [Integrations](integrations/index.md): Open WebUI, LangChain, plain HTTP.
- [Comparisons](comparisons/index.md): LightRAG, GraphRAG, traditional RAG.
- [Security](security/index.md): auth defaults and hardening.
- [Troubleshooting](troubleshooting/index.md): common issues, claim and lease, cancel versus failed.
- [Changelog](../CHANGELOG.md): release history.
