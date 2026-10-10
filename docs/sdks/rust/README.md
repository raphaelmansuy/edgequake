---
title: Rust SDK
description: Install and use the EdgeQuake Rust client crate edgequake-sdk. Async EdgeQuakeClient for documents, query, parse and more.
---

# Rust SDK

Official async Rust client. Crate: **`edgequake-sdk`** version **0.4.0** (published on crates.io).

```toml
[dependencies]
edgequake-sdk = "0.4"
tokio = { version = "1", features = ["full"] }
```

```rust
use edgequake_sdk::{ClientBuilder, types::query::QueryRequest};

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
            mode: Some("mix".into()),
            ..Default::default()
        })
        .await?;
    println!("{:?}", ans.answer);
    Ok(())
}
```

Main type: `EdgeQuakeClient` (built with `ClientBuilder`). Resources mirror the API: documents, pdf, parse, query, chat, graph, conversations, tasks, and more. The Rust `QueryRequest` still exposes a `top_k` field; prefer aligning with the server's `max_results` when you construct requests manually.

Quickstart: [quickstart.md](quickstart.md). Connections: raw HTTP ([Connections](../../api-reference/connections.md)).
