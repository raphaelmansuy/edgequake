---
title: Rust SDK
description: Install and use the EdgeQuake Rust client crate edgequake-sdk 0.4.0 from crates.io. Async EdgeQuakeClient for documents, query, parse and more.
---

# Rust SDK

The Rust client is an async crate built on Tokio. It is the only SDK whose latest source version (**0.4.0**) matches its published crates.io release. Use `EdgeQuakeClient` for documents, query, parse and the other resources, and `ClientBuilder` to configure the base URL and API key.

## Install

```toml
[dependencies]
edgequake-sdk = "0.4"
tokio = { version = "1", features = ["full"] }
```

## Example

This snippet compiles against `edgequake-sdk` 0.4.0.

```rust
use edgequake_sdk::types::query::{QueryMode, QueryRequest};
use edgequake_sdk::ClientBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::default()
        .base_url("http://localhost:8080")
        .api_key("eq-...")
        .build()?;

    let health = client.health().check().await?;
    println!("{health:?}");

    let ans = client
        .query()
        .execute(&QueryRequest {
            query: "What is in my documents?".into(),
            mode: Some(QueryMode::Mix),
            ..Default::default()
        })
        .await?;
    println!("{:?}", ans.answer);
    Ok(())
}
```

`mode` is a `QueryMode` enum, so a plain string such as `"mix".into()` does not compile.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    Builder["ClientBuilder"] -->|"build()"| Client["EdgeQuakeClient"]
    Client --> Docs["documents()"]
    Client --> Query["query()"]
    Client --> Tasks["tasks()"]
    Docs --> API["REST /api/v1"]
    Query --> API
    Tasks --> API
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
class Builder,Client eqActor
```

`ClientBuilder` produces one `EdgeQuakeClient`, and each resource method sends a request to the same REST API.

## Request fields

| Field | Type | Notes |
|-------|------|-------|
| `query` | `String` | Required question text |
| `mode` | `Option<QueryMode>` | Use `QueryMode::Mix` to match the server default |
| `top_k` | field on `QueryRequest` | There is no `max_results` field in this crate |

## Resources

Resources mirror the REST API: `documents`, `pdf`, `parse`, `query`, `chat`, `graph`, `conversations`, `tasks`, and more. Each one is a method on `EdgeQuakeClient`.

## Limits

- Connections and `POST /api/v1/providers/test` are not wrapped. Use raw HTTP ([Connections](../../api-reference/connections.md)).
- `display_status` and `ui_phase` are not modelled.

Next steps: [Rust quickstart](quickstart.md). Overview of all clients: [SDK overview](../README.md).
