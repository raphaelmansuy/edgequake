---
title: Rust SDK quickstart
description: Add edgequake-sdk to a Cargo project and run a first health check and query.
---

# Rust SDK quickstart

```toml
[dependencies]
edgequake-sdk = "0.4"
tokio = { version = "1", features = ["full"] }
```

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

Next: upload with `client.documents().upload(...)`, poll `client.tasks().get(...)`, then `client.query().execute(...)`. See [Rust README](README.md) and [Document upload](../../api-reference/document-upload-quick-reference.md).
