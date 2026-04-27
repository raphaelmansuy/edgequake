# Rust SDK — quickstart

## 1. Configure once

```rust
let client = EdgeQuakeClient::builder()
    .base_url("http://localhost:8080")
    .bearer_token(std::env::var("EDGEQUAKE_TOKEN")?)
    .tenant_id(std::env::var("EDGEQUAKE_TENANT_ID")?)
    .user_id(std::env::var("EDGEQUAKE_USER_ID")?)
    .workspace_id(std::env::var("EDGEQUAKE_WORKSPACE_ID")?)
    .build()?;
```

## 2. Health then work

```rust
let h = client.health().check().await?;
assert_eq!(h.status, "healthy");
```

## 3. Documents (lawful query)

Only send params the API defines (`ListDocumentsRequest`):

```rust
use edgequake_sdk::types::documents::DocumentListQuery;

let q = DocumentListQuery {
    page: Some(1),
    page_size: Some(20),
    date_from: None,
    date_to: None,
    document_pattern: Some("quarterly".to_string()),
};
let page = client.documents().list_with_query(&q).await?;
```

## 4. Conversations

```rust
use edgequake_sdk::types::conversations::ConversationListQuery;

let q = ConversationListQuery {
    limit: Some(50),
    filter_folder_id: Some("folder-uuid".to_string()),
    ..Default::default()
};
let convos = client.conversations().list_with_query(&q).await?;
```

## 5. Run tests (when hacking the SDK)

```bash
cd sdks/rust && cargo test
```
