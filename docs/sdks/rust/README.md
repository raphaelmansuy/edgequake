# Rust SDK

**Location:** `sdks/rust`  
**Authority:** Same headers and `/api/v1` paths as the Axum server.

## Install

In your `Cargo.toml` (path or crates.io as published):

```toml
edgequake-sdk = { path = "../sdks/rust" }
```

## Minimal example

```rust
use edgequake_sdk::EdgeQuakeClient;

#[tokio::main]
async fn main() -> edgequake_sdk::Result<()> {
    let client = EdgeQuakeClient::builder()
        .base_url("http://localhost:8080")
        .bearer_token("YOUR_JWT")
        .tenant_id("tenant-uuid")
        .user_id("user-uuid")
        .workspace_id("workspace-uuid")
        .build()?;

    let health = client.health().check().await?;
    println!("{}", health.status);

    let docs = client.documents().list().await?;
    println!("{} documents on this page", docs.documents.len());

    Ok(())
}
```

## High-value calls

| Goal | Method |
|------|--------|
| List documents with filters | `documents().list_with_query(&DocumentListQuery { page: Some(2), document_pattern: Some("report".into()), ..Default::default() })` |
| List conversations with API filters | `conversations().list_with_query(&ConversationListQuery { .. })` |
| Bulk delete conversations | POST body uses `conversation_ids` via SDK helpers |

## Next

- [Quickstart & patterns](./quickstart.md)
- Crate `README` in `sdks/rust/README.md`
