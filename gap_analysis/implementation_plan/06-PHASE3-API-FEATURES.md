# Phase 3: API Feature Enhancement

**Document ID:** 06-PHASE3-API-FEATURES  
**Priority:** 🟡 P2 MEDIUM  
**Effort:** 5 person-days  
**Duration:** Weeks 7-9  
**Dependencies:** [01](./01-PHASE1-QUERY-ENGINE.md), [02](./02-PHASE1-MULTI-TENANCY.md)  
**Blocks:** None

---

## 📋 Overview

This document provides implementation guidance for API feature enhancements including document scanning, improved graph endpoints, and tenant management APIs.

### Gaps Addressed

| Gap ID      | Feature                | Severity | Status         | Effort |
| ----------- | ---------------------- | -------- | -------------- | ------ |
| **GAP-014** | Document Scan API      | 🟡 P2    | 🔲 Not started | 2 days |
| **GAP-036** | Graph Labels (Popular) | 🟡 P2    | 🔲 Not started | 1 day  |
| **GAP-039** | Reprocess Failed       | 🟡 P2    | 🔲 Not started | 2 days |

### Cross-References

- **Source Analysis:** [../gap-analysis.md](../gap-analysis.md#feature-f-076-document-scan)
- **Master Plan:** [00-INDEX.md](./00-INDEX.md#phase-3-expansion-weeks-7-9)
- **Multi-Tenancy:** [02-PHASE1-MULTI-TENANCY.md](./02-PHASE1-MULTI-TENANCY.md)

---

## 🎯 Document Scan API

### 1.1 Objective

Implement a document scanning API that indexes all documents from a specified directory.

### 1.2 Source Reference

**Location:** `lightrag/api/routers/document_routes.py` - scan endpoint

### 1.3 Implementation Tasks

#### Task 1.3.1: Create Scan Handler

**File:** `edgequake/crates/edgequake-api/src/handlers/documents.rs`

```rust
// ADD to documents.rs handlers

use std::path::PathBuf;
use tokio::fs;

/// Request for scanning a directory
#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    /// Directory path to scan
    pub path: String,
    /// File extensions to include (e.g., ["txt", "md", "pdf"])
    pub extensions: Option<Vec<String>>,
    /// Whether to scan subdirectories
    pub recursive: Option<bool>,
    /// Maximum number of files to process
    pub max_files: Option<usize>,
}

/// Response for scan operation
#[derive(Debug, Serialize)]
pub struct ScanResponse {
    pub task_id: String,
    pub files_found: usize,
    pub status: String,
}

/// Scan a directory for documents and queue them for processing
#[utoipa::path(
    post,
    path = "/api/v1/documents/scan",
    request_body = ScanRequest,
    responses(
        (status = 200, description = "Scan started", body = ScanResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Access denied to path"),
    )
)]
pub async fn scan_directory(
    State(state): State<AppState>,
    Json(request): Json<ScanRequest>,
) -> Result<Json<ScanResponse>, ApiError> {
    // Security: Validate path is within allowed directories
    let path = PathBuf::from(&request.path);
    validate_scan_path(&path, &state.config)?;

    let extensions = request.extensions.unwrap_or_else(|| {
        vec!["txt".to_string(), "md".to_string(), "pdf".to_string()]
    });
    let recursive = request.recursive.unwrap_or(true);
    let max_files = request.max_files.unwrap_or(1000);

    // Discover files
    let files = discover_files(&path, &extensions, recursive, max_files).await?;
    let files_count = files.len();

    // Create batch task
    let task_id = uuid::Uuid::new_v4().to_string();

    // Queue files for processing
    for file_path in files {
        let content = fs::read_to_string(&file_path).await
            .map_err(|e| ApiError::Internal(format!("Failed to read file: {}", e)))?;

        let filename = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Queue document for ingestion
        state.document_queue.send(DocumentTask {
            task_id: task_id.clone(),
            filename,
            content,
            metadata: serde_json::json!({
                "source_path": file_path.to_string_lossy(),
                "scan_task_id": task_id,
            }),
        }).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    Ok(Json(ScanResponse {
        task_id,
        files_found: files_count,
        status: "processing".to_string(),
    }))
}

/// Discover files matching criteria
async fn discover_files(
    path: &PathBuf,
    extensions: &[String],
    recursive: bool,
    max_files: usize,
) -> Result<Vec<PathBuf>, ApiError> {
    let mut files = Vec::new();
    let mut dirs_to_scan = vec![path.clone()];

    while let Some(dir) = dirs_to_scan.pop() {
        if files.len() >= max_files {
            break;
        }

        let mut entries = fs::read_dir(&dir).await
            .map_err(|e| ApiError::Internal(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        {
            let path = entry.path();
            let file_type = entry.file_type().await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            if file_type.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                        files.push(path);
                        if files.len() >= max_files {
                            break;
                        }
                    }
                }
            } else if file_type.is_dir() && recursive {
                dirs_to_scan.push(path);
            }
        }
    }

    Ok(files)
}

/// Validate scan path is allowed
fn validate_scan_path(path: &PathBuf, config: &AppConfig) -> Result<(), ApiError> {
    // Security: Ensure path is absolute and within allowed roots
    if !path.is_absolute() {
        return Err(ApiError::BadRequest("Path must be absolute".to_string()));
    }

    // Check against allowed scan roots
    let allowed = config.allowed_scan_paths.iter().any(|allowed| {
        path.starts_with(allowed)
    });

    if !allowed {
        return Err(ApiError::Forbidden("Path not in allowed scan roots".to_string()));
    }

    Ok(())
}
```

---

## 🎯 Graph Labels with Popular Entities

### 2.1 Objective

Enhance graph labels endpoint to return popular/most-connected entities.

### 2.2 Implementation Tasks

#### Task 2.2.1: Enhance Graph Labels Endpoint

**File:** `edgequake/crates/edgequake-api/src/handlers/graph.rs`

```rust
// ADD to graph.rs handlers

/// Response for graph labels with popular entities
#[derive(Debug, Serialize)]
pub struct GraphLabelsResponse {
    pub entity_types: Vec<LabelCount>,
    pub relationship_types: Vec<LabelCount>,
    pub popular_entities: Vec<PopularEntity>,
    pub stats: GraphStats,
}

#[derive(Debug, Serialize)]
pub struct LabelCount {
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct PopularEntity {
    pub name: String,
    pub entity_type: String,
    pub connection_count: usize,
    pub description: Option<String>,
}

/// Get graph labels and statistics including popular entities
#[utoipa::path(
    get,
    path = "/api/v1/graph/labels",
    params(
        ("top_k" = Option<usize>, Query, description = "Number of popular entities to return")
    ),
    responses(
        (status = 200, description = "Graph labels", body = GraphLabelsResponse),
    )
)]
pub async fn get_graph_labels(
    State(state): State<AppState>,
    Query(params): Query<GraphLabelsParams>,
) -> Result<Json<GraphLabelsResponse>, ApiError> {
    let top_k = params.top_k.unwrap_or(10);
    let graph = state.graph_storage.read().await;

    // Get all nodes and count by type
    let nodes = graph.get_all_nodes().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for node in &nodes {
        let entity_type = node.properties.get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");
        *type_counts.entry(entity_type.to_string()).or_default() += 1;
    }

    let entity_types: Vec<LabelCount> = type_counts
        .into_iter()
        .map(|(label, count)| LabelCount { label, count })
        .collect();

    // Get edges and count connections per node
    let edges = graph.get_all_edges().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut connection_counts: HashMap<String, usize> = HashMap::new();
    for edge in &edges {
        *connection_counts.entry(edge.source.clone()).or_default() += 1;
        *connection_counts.entry(edge.target.clone()).or_default() += 1;
    }

    // Find most connected entities
    let mut popular: Vec<_> = connection_counts.into_iter().collect();
    popular.sort_by(|a, b| b.1.cmp(&a.1));

    let popular_entities: Vec<PopularEntity> = popular
        .into_iter()
        .take(top_k)
        .filter_map(|(name, count)| {
            nodes.iter()
                .find(|n| n.id == name)
                .map(|node| PopularEntity {
                    name: name.clone(),
                    entity_type: node.properties.get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    connection_count: count,
                    description: node.properties.get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.chars().take(200).collect()),
                })
        })
        .collect();

    // Count relationship types (simplified - all RELATED for now)
    let relationship_types = vec![LabelCount {
        label: "RELATED".to_string(),
        count: edges.len(),
    }];

    Ok(Json(GraphLabelsResponse {
        entity_types,
        relationship_types,
        popular_entities,
        stats: GraphStats {
            node_count: nodes.len(),
            edge_count: edges.len(),
        },
    }))
}

#[derive(Debug, Deserialize)]
pub struct GraphLabelsParams {
    pub top_k: Option<usize>,
}
```

---

## 🎯 Reprocess Failed Documents

### 3.1 Objective

Implement endpoint to retry processing of failed documents.

### 3.2 Implementation Tasks

#### Task 3.2.1: Add Reprocess Endpoint

**File:** `edgequake/crates/edgequake-api/src/handlers/documents.rs`

```rust
// ADD to documents.rs handlers

/// Request to reprocess failed documents
#[derive(Debug, Deserialize)]
pub struct ReprocessRequest {
    /// Specific document IDs to reprocess (if empty, all failed)
    pub document_ids: Option<Vec<String>>,
    /// Maximum number to reprocess
    pub limit: Option<usize>,
}

/// Response for reprocess operation
#[derive(Debug, Serialize)]
pub struct ReprocessResponse {
    pub task_id: String,
    pub documents_queued: usize,
    pub status: String,
}

/// Reprocess failed documents
#[utoipa::path(
    post,
    path = "/api/v1/documents/reprocess",
    request_body = ReprocessRequest,
    responses(
        (status = 200, description = "Reprocess started", body = ReprocessResponse),
    )
)]
pub async fn reprocess_failed(
    State(state): State<AppState>,
    Json(request): Json<ReprocessRequest>,
) -> Result<Json<ReprocessResponse>, ApiError> {
    let limit = request.limit.unwrap_or(100);

    // Get failed documents from status storage
    let failed_docs = match &request.document_ids {
        Some(ids) => {
            // Fetch specific documents
            let mut docs = Vec::new();
            for id in ids {
                if let Some(status) = state.doc_status_storage.get(id).await
                    .map_err(|e| ApiError::Internal(e.to_string()))?
                {
                    if status.status == DocumentStatus::Failed {
                        docs.push((id.clone(), status));
                    }
                }
            }
            docs
        }
        None => {
            // Get all failed documents
            state.doc_status_storage
                .list_by_status(DocumentStatus::Failed, limit)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
        }
    };

    let docs_count = failed_docs.len();
    let task_id = uuid::Uuid::new_v4().to_string();

    // Queue documents for reprocessing
    for (doc_id, status) in failed_docs {
        // Reset status
        state.doc_status_storage.update_status(
            &doc_id,
            DocumentStatus::Pending,
            None,
        ).await.map_err(|e| ApiError::Internal(e.to_string()))?;

        // Queue for processing
        if let Some(content) = status.content {
            state.document_queue.send(DocumentTask {
                task_id: task_id.clone(),
                filename: status.filename.unwrap_or_else(|| doc_id.clone()),
                content,
                metadata: serde_json::json!({
                    "reprocess_task_id": task_id,
                    "original_doc_id": doc_id,
                    "retry_count": status.retry_count.unwrap_or(0) + 1,
                }),
            }).await.map_err(|e| ApiError::Internal(e.to_string()))?;
        }
    }

    Ok(Json(ReprocessResponse {
        task_id,
        documents_queued: docs_count,
        status: "processing".to_string(),
    }))
}
```

---

## 🔗 Route Registration

**File:** `edgequake/crates/edgequake-api/src/routes.rs`

```rust
// ADD new routes

pub fn document_routes() -> Router<AppState> {
    Router::new()
        // Existing routes...
        .route("/documents", post(handlers::documents::upload))
        .route("/documents", get(handlers::documents::list))
        // NEW routes
        .route("/documents/scan", post(handlers::documents::scan_directory))
        .route("/documents/reprocess", post(handlers::documents::reprocess_failed))
}

pub fn graph_routes() -> Router<AppState> {
    Router::new()
        // Enhanced route
        .route("/graph/labels", get(handlers::graph::get_graph_labels))
        // Existing routes...
        .route("/graph", get(handlers::graph::get_graph))
}
```

---

## 📊 Testing Requirements

### Unit Tests

```bash
cargo test --package edgequake-api --lib handlers::documents
cargo test --package edgequake-api --lib handlers::graph
```

### Integration Tests

```bash
cargo test --package edgequake-api --test api_integration
```

---

## 🔗 Cross-References

| Topic         | Document                                                   | Section       |
| ------------- | ---------------------------------------------------------- | ------------- |
| Gap Details   | [../gap-analysis.md](../gap-analysis.md)                   | F-076, F-077  |
| Multi-Tenancy | [02-PHASE1-MULTI-TENANCY.md](./02-PHASE1-MULTI-TENANCY.md) | Tenant Routes |
| Testing Plan  | [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md)     | API Tests     |
| Master Index  | [00-INDEX.md](./00-INDEX.md)                               | Phase 3       |

---

## ✅ Completion Criteria

| Criterion                 | Target                 | Validation       |
| ------------------------- | ---------------------- | ---------------- |
| Scan API works            | Directory indexed      | Integration test |
| Popular entities returned | Sorted by connections  | Unit test        |
| Reprocess works           | Failed docs retried    | Integration test |
| Security validated        | Path traversal blocked | Security test    |

---

_Document Version: 1.0_  
_Last Updated: 2024-12-24_  
_Owner: EdgeQuake API Team_
