---
title: "Rust SDK"
description: "Rust SDK for EdgeQuake."
---

# Rust SDK

> **Product: v0.23.0** · Crate **~0.4.0** (decoupled from server)

**Location:** `sdks/rust`  
**Authority:** Same headers and `/api/v1` paths as the Axum server.

## Install

In your `Cargo.toml` (crates.io or monorepo path):

```toml
edgequake-sdk = "0.4"
# edgequake-sdk = { path = "../sdks/rust" }
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
| Cancel ingestion task | `tasks().cancel(track_id)` — see [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md) |
| Stateless parse (SPEC-094) | `parse().parse(file_bytes, filename, options)` → `ParseOutcome::{Completed, Accepted}`; also `parse().backends()`, `parse().job(id)` |

## Stateless parse (SPEC-094, v0.23)

`POST /api/v1/parse` converts a PDF to Markdown without ingestion residue. Sync by default (≤ 15 pages / 20 MiB); async for larger jobs (≤ 1000 pages).

```rust
use edgequake_sdk::resources::parse::ParseOptions;

let bytes = std::fs::read("/tmp/paper.pdf")?;

// Sync parse — ParseOutcome::Completed(ParseResponse) or Accepted(202)
let outcome = client
    .parse()
    .parse(
        bytes,
        "paper.pdf",
        ParseOptions { pages: Some("1-5".into()), ..Default::default() },
    )
    .await?;

match outcome {
    edgequake_sdk::resources::parse::ParseOutcome::Completed(res) => {
        println!("{} pages, {} ms", res.page_count, res.metrics.total_ms);
        println!("{}", &res.markdown[..200.min(res.markdown.len())]);
    }
    edgequake_sdk::resources::parse::ParseOutcome::Accepted(acc) => {
        let status = client.parse().job(&acc.job_id).await?;
        if let Some(result) = status.result {
            println!("{}", &result.markdown[..200.min(result.markdown.len())]);
        }
    }
}

let backends = client.parse().backends().await?; // ParseBackendsResponse
```

Document responses also expose `display_status` / `ui_phase` (SPEC-057 P4) — prefer them over raw `status`/`stage` for progress UI. Query keyword/answer caching is **server-side only** (`EDGEQUAKE_LLM_CACHE=1` default; `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE` overrides) — no client change needed.

## Next

- [Quickstart & patterns](./quickstart.md)
- Crate `README` in `sdks/rust/README.md`
