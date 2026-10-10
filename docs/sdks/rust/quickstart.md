---
title: Rust SDK quickstart
description: Add edgequake-sdk to a Cargo project, run a health check, and ask a first query.
---

# Rust SDK quickstart

This guide adds the crate to a Cargo project, checks the server, and runs one query. You need Rust with Cargo and a running EdgeQuake server on port 8080.

## 1. Add the crate

```toml
[dependencies]
edgequake-sdk = "0.4"
tokio = { version = "1", features = ["full"] }
```

## 2. Check the server

```rust
use edgequake_sdk::ClientBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::default()
        .base_url("http://localhost:8080")
        .build()?;
    println!("{:?}", client.health().check().await?);
    Ok(())
}
```

## 3. Ask a question

Use the query snippet in the [Rust README](README.md#example). It sets `mode: Some(QueryMode::Mix)` and passes the API key if the server needs one.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TB
    Add["Add edgequake-sdk to Cargo.toml"] --> Build["ClientBuilder build()"]
    Build --> Health{"health().check() succeeds?"}
    Health -->|"yes"| Query["query().execute(QueryRequest)"]
    Health -->|"no"| Fix["Start the server and check base_url"]
    Fix --> Build
    Query --> Answer["Print answer"]
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
class Add,Build eqActor
```

The flow checks the server before the query, so a wrong base URL fails early.

Upload and task polling are covered in [Document upload](../../api-reference/document-upload-quick-reference.md).
