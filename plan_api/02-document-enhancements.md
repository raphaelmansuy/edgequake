# Document Management Enhancements

**Specification Version:** 1.0  
**Target Release:** EdgeQuake v1.1.0 - v1.2.0  
**Priority:** MEDIUM-HIGH  
**Status:** Planning

---

## Overview

Enhance document management capabilities to match LightRAG functionality, including file uploads, batch operations, status tracking, and directory scanning.

### Goals

1. **File Upload Support:** Accept multipart/form-data file uploads
2. **Direct Text Insert:** API endpoint for inserting text without files
3. **Batch Operations:** Insert multiple texts in single request
4. **Status Tracking:** Detailed document status (pending/processing/indexed/failed)
5. **Duplicate Detection:** Prevent re-indexing identical content
6. **Bulk Operations:** Delete all, delete failed, clear cache
7. **Directory Scanning:** Automatically index files from directory
8. **Statistics:** Document and processing statistics

---

## New Endpoints

### 1. Upload File (Multipart)

```rust
POST /api/v1/documents/upload
Content-Type: multipart/form-data
```

**Request:**

```http
POST /api/v1/documents/upload HTTP/1.1
Host: localhost:8080
Content-Type: multipart/form-data; boundary=----WebKitFormBoundary

------WebKitFormBoundary
Content-Disposition: form-data; name="file"; filename="research.pdf"
Content-Type: application/pdf

<binary PDF data>
------WebKitFormBoundary
Content-Disposition: form-data; name="metadata"

{"author": "John Doe", "tags": ["research", "ai"]}
------WebKitFormBoundary--
```

**Response (202 Accepted):**

```json
{
  "document_id": "doc-xyz789",
  "track_id": "upload-a1b2c3d4...",
  "status": "accepted",
  "message": "File uploaded successfully. Processing in background.",
  "file_info": {
    "filename": "research.pdf",
    "size_bytes": 524288,
    "content_type": "application/pdf"
  }
}
```

### 2. Insert Text Directly

```rust
POST /api/v1/documents/text
Content-Type: application/json
```

**Request:**

```json
{
  "text": "Artificial intelligence research paper content...",
  "file_source": "api_request_123",
  "title": "AI Research Summary",
  "metadata": {
    "author": "Jane Smith",
    "tags": ["ai", "ml"],
    "date": "2025-12-22"
  }
}
```

**Response (202 Accepted):**

```json
{
  "document_id": "doc-abc123",
  "track_id": "insert-def456...",
  "status": "accepted",
  "message": "Text accepted for processing."
}
```

**Response (409 Conflict - Duplicate):**

```json
{
  "status": "duplicated",
  "message": "Identical content already exists",
  "existing_document_id": "doc-xyz789",
  "existing_track_id": "insert-old123...",
  "existing_status": "indexed"
}
```

### 3. Insert Multiple Texts (Batch)

```rust
POST /api/v1/documents/texts
Content-Type: application/json
```

**Request:**

```json
{
  "texts": [
    "First document content...",
    "Second document content...",
    "Third document content..."
  ],
  "file_sources": ["batch_1_doc_1", "batch_1_doc_2", "batch_1_doc_3"],
  "metadata": {
    "batch_id": "batch_2025_12_22",
    "category": "research_papers"
  }
}
```

**Response (202 Accepted):**

```json
{
  "batch_id": "batch-xyz123",
  "status": "accepted",
  "message": "3 texts accepted for processing",
  "documents": [
    {
      "document_id": "doc-1",
      "track_id": "insert-track1...",
      "status": "pending"
    },
    {
      "document_id": "doc-2",
      "track_id": "insert-track2...",
      "status": "pending"
    },
    {
      "document_id": "doc-3",
      "track_id": "insert-track3...",
      "status": "duplicated",
      "existing_document_id": "doc-old"
    }
  ]
}
```

### 4. Get Document Status with Filtering

```rust
GET /api/v1/documents/status
```

**Query Parameters:**

- `filter`: all, indexed, failed, processing, pending (default: all)
- `search`: Search in filenames/titles
- `page`: Page number (default: 1)
- `page_size`: Items per page (default: 20)
- `sort`: created_at, updated_at, file_path (default: created_at)
- `order`: asc, desc (default: desc)

**Request:**

```http
GET /api/v1/documents/status?filter=failed&page=1&page_size=10 HTTP/1.1
```

**Response (200 OK):**

```json
{
  "documents": [
    {
      "doc_id": "doc-abc123",
      "file_path": "research_paper.pdf",
      "status": "failed",
      "track_id": "upload-xyz...",
      "created_at": "2025-12-22T10:00:00Z",
      "updated_at": "2025-12-22T10:05:00Z",
      "error_message": "LLM API timeout after 3 retries",
      "chunk_count": 0,
      "entity_count": 0,
      "relationship_count": 0
    }
  ],
  "statistics": {
    "total": 1250,
    "indexed": 1200,
    "failed": 35,
    "processing": 10,
    "pending": 5
  },
  "pagination": {
    "page": 1,
    "page_size": 10,
    "total_pages": 4,
    "total": 35
  }
}
```

### 5. Delete Document by Filename

```rust
DELETE /api/v1/documents/file/{filename}
```

**Request:**

```http
DELETE /api/v1/documents/file/research_paper.pdf HTTP/1.1
```

**Response (200 OK):**

```json
{
  "status": "success",
  "message": "Document deleted successfully",
  "deleted_document_id": "doc-abc123",
  "deleted_chunks": 42,
  "deleted_entities": 87,
  "deleted_relationships": 134
}
```

**Response (404 Not Found):**

```json
{
  "error": "document_not_found",
  "message": "No document found with filename: unknown.pdf"
}
```

### 6. Clear All Documents

```rust
DELETE /api/v1/documents/clear
```

**Request:**

```http
DELETE /api/v1/documents/clear HTTP/1.1
```

**Response (200 OK):**

```json
{
  "status": "success",
  "message": "All documents cleared successfully",
  "deleted_documents": 1250,
  "deleted_chunks": 52000,
  "deleted_entities": 104000,
  "deleted_relationships": 156000
}
```

**Response (200 OK with confirmation required):**

```http
DELETE /api/v1/documents/clear?confirm=true HTTP/1.1
```

### 7. Delete Failed Documents Only

```rust
DELETE /api/v1/documents/failed
```

**Request:**

```http
DELETE /api/v1/documents/failed HTTP/1.1
```

**Response (200 OK):**

```json
{
  "status": "success",
  "message": "Failed documents cleared",
  "deleted_documents": 35,
  "document_ids": ["doc-1", "doc-2", "..."]
}
```

### 8. Reindex Failed Documents

```rust
POST /api/v1/documents/reindex-failed
```

**Request:**

```http
POST /api/v1/documents/reindex-failed HTTP/1.1
```

**Response (202 Accepted):**

```json
{
  "status": "accepted",
  "message": "35 failed documents queued for reindexing",
  "track_ids": ["reindex-abc123...", "reindex-def456...", "..."]
}
```

### 9. Scan Directory for Documents

```rust
POST /api/v1/documents/scan
Content-Type: application/json
```

**Request:**

```json
{
  "directory_path": "/data/input",
  "recursive": true,
  "file_patterns": ["*.pdf", "*.txt", "*.docx"],
  "exclude_patterns": ["*_draft*", "*.tmp"],
  "metadata": {
    "batch_id": "scan_2025_12_22",
    "source": "data_import"
  }
}
```

**Response (202 Accepted):**

```json
{
  "status": "accepted",
  "message": "Directory scan initiated",
  "track_id": "scan-xyz789...",
  "scan_config": {
    "directory_path": "/data/input",
    "recursive": true,
    "patterns": ["*.pdf", "*.txt", "*.docx"]
  }
}
```

### 10. Get Document Statistics

```rust
GET /api/v1/documents/stats
```

**Response (200 OK):**

```json
{
  "documents": {
    "total": 1250,
    "indexed": 1200,
    "failed": 35,
    "processing": 10,
    "pending": 5
  },
  "content": {
    "total_chunks": 52000,
    "total_entities": 104000,
    "total_relationships": 156000,
    "total_size_bytes": 524288000
  },
  "storage": {
    "kv_storage_size_mb": 250.5,
    "vector_storage_size_mb": 1024.3,
    "graph_storage_size_mb": 512.7
  },
  "processing": {
    "avg_processing_time_seconds": 12.5,
    "avg_chunk_size_bytes": 2048,
    "success_rate_percent": 96.0
  }
}
```

---

## Data Models

### Document Status Table

```sql
CREATE TABLE document_status (
    -- Identity
    doc_id VARCHAR(100) PRIMARY KEY,
    file_path TEXT,
    content_hash VARCHAR(64),  -- SHA-256 of content for deduplication

    -- Status
    status VARCHAR(20) NOT NULL,  -- pending, processing, indexed, failed
    track_id VARCHAR(50),

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    indexed_at TIMESTAMPTZ,

    -- Processing info
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,

    -- Content stats
    chunk_count INTEGER DEFAULT 0,
    entity_count INTEGER DEFAULT 0,
    relationship_count INTEGER DEFAULT 0,
    size_bytes BIGINT DEFAULT 0,

    -- Metadata
    metadata JSONB,

    -- Foreign key
    FOREIGN KEY (track_id) REFERENCES tasks(track_id),

    -- Constraints
    CONSTRAINT valid_doc_status CHECK (status IN ('pending', 'processing', 'indexed', 'failed'))
);

-- Indexes
CREATE INDEX idx_doc_status ON document_status(status);
CREATE INDEX idx_doc_file_path ON document_status(file_path);
CREATE INDEX idx_doc_content_hash ON document_status(content_hash);
CREATE INDEX idx_doc_created ON document_status(created_at DESC);
CREATE INDEX idx_doc_track_id ON document_status(track_id);
```

### Rust Models

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStatus {
    pub doc_id: String,
    pub file_path: Option<String>,
    pub content_hash: String,
    pub status: DocumentStatusType,
    pub track_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub indexed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub chunk_count: i32,
    pub entity_count: i32,
    pub relationship_count: i32,
    pub size_bytes: i64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatusType {
    Pending,
    Processing,
    Indexed,
    Failed,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UploadFileRequest {
    #[serde(skip)]  // Populated from multipart field
    pub file: Vec<u8>,

    #[serde(skip)]
    pub filename: String,

    #[serde(skip)]
    pub content_type: String,

    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct InsertTextRequest {
    pub text: String,
    pub file_source: Option<String>,
    pub title: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct InsertTextsRequest {
    pub texts: Vec<String>,
    pub file_sources: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScanDirectoryRequest {
    pub directory_path: String,
    pub recursive: bool,
    pub file_patterns: Vec<String>,
    pub exclude_patterns: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}
```

---

## Implementation Details

### Content Deduplication

**LightRAG vs EdgeQuake:**

- LightRAG uses MD5: `hashlib.md5(content.encode()).hexdigest()`
- EdgeQuake uses SHA-256 (more secure, recommended)

**LightRAG Implementation:**

```python
# File: lightrag/utils.py
def compute_mdhash_id(content: str, prefix: str = "doc") -> str:
    """Compute MD5 hash for content deduplication"""
    return f"{prefix}-{hashlib.md5(content.encode()).hexdigest()}"
```

**EdgeQuake Implementation (SHA-256):**

```rust
use sha2::{Sha256, Digest};

pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub async fn check_duplicate(
    storage: &impl DocumentStatusStorage,
    content: &str,
) -> Result<Option<DocumentStatus>, Error> {
    let content_hash = compute_content_hash(content);
    storage.get_by_content_hash(&content_hash).await
}
```

### File Upload Handler

```rust
use axum::extract::Multipart;

pub async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<UploadFileResponse>)> {
    let mut file_data = Vec::new();
    let mut filename = String::new();
    let mut content_type = String::new();
    let mut metadata = None;

    while let Some(field) = multipart.next_field().await? {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                filename = field.file_name()
                    .ok_or(ApiError::BadRequest("Missing filename".to_string()))?
                    .to_string();
                content_type = field.content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                file_data = field.bytes().await?.to_vec();
            }
            "metadata" => {
                let bytes = field.bytes().await?;
                metadata = Some(serde_json::from_slice(&bytes)?);
            }
            _ => {}
        }
    }

    // Validate file
    if file_data.is_empty() {
        return Err(ApiError::BadRequest("Empty file".to_string()));
    }

    // Check file size
    if file_data.len() > state.config.max_file_size {
        return Err(ApiError::PayloadTooLarge(format!(
            "File exceeds maximum size of {} bytes",
            state.config.max_file_size
        )));
    }

    // Extract text content (PDF/DOCX parsing)
    let content = extract_text(&file_data, &content_type).await?;

    // Check for duplicates
    if let Some(existing) = check_duplicate(&state.doc_status_storage, &content).await? {
        return Ok((
            StatusCode::CONFLICT,
            Json(UploadFileResponse {
                status: "duplicated".to_string(),
                message: "File with identical content already exists".to_string(),
                existing_document_id: Some(existing.doc_id),
                existing_track_id: existing.track_id,
                ..Default::default()
            }),
        ));
    }

    // Create document record
    let document_id = Uuid::new_v4().to_string();
    let content_hash = compute_content_hash(&content);

    // Create background task
    let task_data = serde_json::json!({
        "document_id": document_id,
        "content": content,
        "file_path": filename,
        "content_type": content_type,
        "size_bytes": file_data.len(),
        "metadata": metadata,
    });

    let track_id = state.task_service
        .create_task(TaskType::Upload, task_data, None)
        .await?;

    // Create document status record
    let doc_status = DocumentStatus {
        doc_id: document_id.clone(),
        file_path: Some(filename.clone()),
        content_hash,
        status: DocumentStatusType::Pending,
        track_id: Some(track_id.clone()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        size_bytes: file_data.len() as i64,
        metadata,
        ..Default::default()
    };

    state.doc_status_storage.create(&doc_status).await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(UploadFileResponse {
            document_id,
            track_id,
            status: "accepted".to_string(),
            message: "File uploaded successfully. Processing in background.".to_string(),
            file_info: Some(FileInfo {
                filename,
                size_bytes: file_data.len(),
                content_type,
            }),
            ..Default::default()
        }),
    ))
}
```

### Directory Scanner

```rust
use walkdir::WalkDir;
use globset::{Glob, GlobSet, GlobSetBuilder};

pub struct DirectoryScanner {
    task_service: Arc<TaskService>,
    doc_status_storage: Arc<dyn DocumentStatusStorage>,
}

impl DirectoryScanner {
    pub async fn scan_directory(
        &self,
        request: ScanDirectoryRequest,
    ) -> Result<String, Error> {
        // Build glob patterns
        let mut include_builder = GlobSetBuilder::new();
        for pattern in &request.file_patterns {
            include_builder.add(Glob::new(pattern)?);
        }
        let include_set = include_builder.build()?;

        let mut exclude_builder = GlobSetBuilder::new();
        if let Some(exclude) = &request.exclude_patterns {
            for pattern in exclude {
                exclude_builder.add(Glob::new(pattern)?);
            }
        }
        let exclude_set = exclude_builder.build()?;

        // Create scan task
        let track_id = self.task_service
            .create_task(
                TaskType::Scan,
                serde_json::to_value(&request)?,
                None,
            )
            .await?;

        Ok(track_id)
    }

    async fn process_scan(&self, request: ScanDirectoryRequest) -> Result<(), Error> {
        let walker = if request.recursive {
            WalkDir::new(&request.directory_path)
        } else {
            WalkDir::new(&request.directory_path).max_depth(1)
        };

        for entry in walker {
            let entry = entry?;

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or(Error::InvalidFilename)?;

            // Check patterns
            if !include_set.is_match(filename) {
                continue;
            }

            if exclude_set.is_match(filename) {
                continue;
            }

            // Read and process file
            let content = tokio::fs::read_to_string(path).await?;

            // Check for duplicates
            if check_duplicate(&self.doc_status_storage, &content).await?.is_some() {
                tracing::info!("Skipping duplicate file: {}", filename);
                continue;
            }

            // Create insert task
            let task_data = serde_json::json!({
                "content": content,
                "file_path": path.to_string_lossy(),
                "metadata": request.metadata,
            });

            self.task_service
                .create_task(TaskType::Insert, task_data, None)
                .await?;
        }

        Ok(())
    }
}
```

---

## Testing

```rust
#[tokio::test]
async fn test_file_upload() {
    let app = test_app().await;

    let form = multipart::Form::new()
        .file("file", "test.txt", "Test content".as_bytes())
        .text("metadata", r#"{"tags": ["test"]}"#);

    let response = app
        .post("/api/v1/documents/upload")
        .multipart(form)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let body: UploadFileResponse = response.json().await;
    assert!(body.track_id.starts_with("upload-"));
}

#[tokio::test]
async fn test_duplicate_detection() {
    let app = test_app().await;

    let content = "Unique test content";

    // First insert
    let resp1 = app
        .post("/api/v1/documents/text")
        .json(&json!({"text": content}))
        .send()
        .await;
    assert_eq!(resp1.status(), StatusCode::ACCEPTED);

    // Second insert (duplicate)
    let resp2 = app
        .post("/api/v1/documents/text")
        .json(&json!({"text": content}))
        .send()
        .await;
    assert_eq!(resp2.status(), StatusCode::CONFLICT);

    let body: InsertTextResponse = resp2.json().await;
    assert_eq!(body.status, "duplicated");
}
```

---

**Status:** ✅ Specification Complete  
**Dependencies:** 01-background-tasks.md  
**Next:** Implement file upload and status tracking
