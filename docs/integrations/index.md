---
title: Integrations
description: Connect EdgeQuake to LangChain, Open WebUI, MCP clients and custom HTTP apps. Points to SDKs and the REST API.
---

# Integrations

EdgeQuake offers four front doors for other tools: an Ollama-compatible API, an MCP gateway, a Python SDK and the REST API. This section shows how to connect each one. It is for developers who already run EdgeQuake and want a chat UI, an agent, a LangChain chain or a plain HTTP client. Product pin: **v0.32.2**.

Use an [official SDK](../sdks/README.md) when one exists. Fall back to the [REST API](../api-reference/index.md) for everything else, including the v0.33.0 Connections routes.

## Choose a front door

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    UI["Open WebUI"] --> OL["Ollama API /api"]
    Agent["AI agent"] --> MCP["MCP /mcp"]
    LC["LangChain"] --> SDK["Python SDK"]
    Custom["Custom app"] --> REST["REST /api/v1"]
    OL --> EQ["EdgeQuake"]
    MCP --> EQ
    SDK --> EQ
    REST --> EQ
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class UI,MCP,SDK eqActor
class OL eqLlm
```

Read it left to right. Each tool reaches EdgeQuake through its own door. The knowledge graph and storage are the same behind every door.

## Pages

| Integration | Use it when |
|-------------|-------------|
| [Open WebUI](open-webui.md) | You want a ChatGPT-style UI on top of Graph-RAG |
| [LangChain](langchain.md) | You build Python RAG chains |
| [MCP](mcp.md) | An AI agent should search and, optionally, ingest through tools |
| [Custom clients](custom-clients.md) | No SDK fits, or you need Connections and the provider test |

Related: [SDK overview](../sdks/README.md), [API reference](../api-reference/index.md), [Getting started](../getting-started/quick-start.md).
