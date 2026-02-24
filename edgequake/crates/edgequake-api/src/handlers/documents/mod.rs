//! Document ingestion handlers.
//!
//! @implements FEAT0407 (Document REST API Handlers)
//! @implements FEAT0402
//!
//! # Implements
//!
//! - **UC0001**: Upload Document
//! - **UC0002**: List Documents  
//! - **UC0003**: View Document Details
//! - **UC0005**: Delete Document
//! - **FEAT0401**: Document Upload (Text)
//! - **FEAT0402**: Document Upload (File)
//! - **FEAT0001**: Document Ingestion Pipeline
//!
//! # Enforces
//!
//! - **BR0001**: Documents must be unique (SHA-256 content hash)
//! - **BR0002**: Chunk size 1200 tokens, overlap 100 tokens
//! - **BR0201**: Tenant isolation (workspace scoping)
//! - **BR0401**: Authentication required for all endpoints
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | POST | `/api/v1/documents` | [`upload_document`] | Upload text/file for ingestion |
//! | GET | `/api/v1/documents` | [`list_documents`] | List all documents |
//! | GET | `/api/v1/documents/:id` | [`get_document`] | Get document details |
//! | DELETE | `/api/v1/documents/:id` | [`delete_document`] | Delete with cascade |
//!
//! # WHY: Two Ingestion Modes
//!
//! Documents can be processed synchronously or asynchronously:
//! - **Sync**: Small documents (<10KB), immediate response with entities
//! - **Async**: Large documents, returns task_id for polling
//!
//! Async mode prevents request timeouts for large PDFs (can take 30s+ to process).
//!
//! # Module Organization (SRP)
//!
//! - `storage_helpers`: Shared storage utilities (vector storage lookup, graph cleanup)
//! - `upload`: Document upload handlers (text + file multipart)
//! - `query`: Document listing, detail, track status, directory scanning
//! - `delete`: Document deletion with cascade cleanup
//! - `recovery`: Failed document reprocessing, stuck recovery, chunk retry

// Re-export DTOs from documents_types module
pub use crate::handlers::documents_types::*;

// Sub-modules: each owns a single responsibility
mod delete;
mod query;
mod recovery;
mod storage_helpers;
mod upload;

// Re-export all public items (includes utoipa __path_* structs for OpenAPI)
pub use delete::*;
pub use query::*;
pub use recovery::*;
pub use storage_helpers::CleanupStats;
pub use upload::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_request_validation() {
        let request = UploadDocumentRequest {
            content: "Test content".to_string(),
            title: Some("Test".to_string()),
            metadata: None,
            async_processing: false,
            track_id: None,
            enable_gleaning: true,
            max_gleaning: 1,
            use_llm_summarization: true,
        };

        assert!(!request.content.is_empty());
    }

    #[test]
    fn test_upload_request_serialization() {
        let json = r#"{
            "content": "Hello world",
            "title": "Test Doc",
            "metadata": {"key": "value"}
        }"#;

        let request: UploadDocumentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Hello world");
        assert_eq!(request.title, Some("Test Doc".to_string()));
        assert!(request.metadata.is_some());
    }

    #[test]
    fn test_upload_request_minimal() {
        let json = r#"{"content": "Just content"}"#;

        let request: UploadDocumentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Just content");
        assert!(request.title.is_none());
        assert!(request.metadata.is_none());
    }

    #[test]
    fn test_upload_response_serialization() {
        let response = UploadDocumentResponse {
            document_id: "doc-123".to_string(),
            status: "processed".to_string(),
            task_id: None,
            track_id: "upload_20240101_abc12345".to_string(),
            duplicate_of: None,
            chunk_count: Some(5),
            entity_count: Some(3),
            relationship_count: Some(2),
            cost: Some(DocumentCostInfo {
                total_cost_usd: 0.0045,
                formatted_cost: "$0.004500".to_string(),
                input_tokens: 1000,
                output_tokens: 500,
                total_tokens: 1500,
                llm_model: Some("gpt-4o-mini".to_string()),
                embedding_model: Some("text-embedding-3-small".to_string()),
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-123"));
        assert!(json.contains("processed"));
        assert!(json.contains("cost"));
        assert!(json.contains("0.0045"));
    }

    #[test]
    fn test_list_documents_request_defaults() {
        let json = r#"{}"#;
        let request: ListDocumentsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 20);
    }

    #[test]
    fn test_list_documents_request_custom() {
        let json = r#"{"page": 3, "page_size": 50}"#;
        let request: ListDocumentsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 3);
        assert_eq!(request.page_size, 50);
    }

    #[test]
    fn test_document_summary_serialization() {
        let summary = DocumentSummary {
            id: "doc-456".to_string(),
            title: Some("My Document".to_string()),
            file_name: None,
            content_summary: Some("This is the first 200 chars of the document...".to_string()),
            content_length: Some(5000),
            chunk_count: 10,
            entity_count: None,
            status: Some("completed".to_string()),
            error_message: None,
            track_id: Some("upload_20240101_abc12345".to_string()),
            created_at: None,
            updated_at: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            llm_model: None,
            embedding_model: None,
            // SPEC-002 fields
            source_type: Some("markdown".to_string()),
            current_stage: Some("completed".to_string()),
            stage_progress: Some(1.0),
            stage_message: None,
            pdf_id: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("doc-456"));
        assert!(json.contains("My Document"));
    }

    #[test]
    fn test_list_documents_response_serialization() {
        let response = ListDocumentsResponse {
            documents: vec![DocumentSummary {
                id: "doc-1".to_string(),
                title: None,
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 5,
                entity_count: None,
                status: Some("completed".to_string()),
                error_message: None,
                track_id: None,
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                // SPEC-002 fields
                source_type: None,
                current_stage: Some("completed".to_string()),
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            }],
            total: 1,
            page: 1,
            page_size: 20,
            total_pages: 1,
            has_more: false,
            status_counts: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-1"));
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"total_pages\":1"));
        assert!(json.contains("\"has_more\":false"));
    }

    #[test]
    fn test_document_detail_response_serialization() {
        let response = DocumentDetailResponse {
            id: "doc-789".to_string(),
            title: Some("Test".to_string()),
            file_name: None,
            content: None,
            content_summary: None,
            content_length: None,
            content_hash: None,
            chunk_count: 5,
            entity_count: None,
            relationship_count: None,
            status: "processed".to_string(),
            error_message: None,
            source_type: None,
            mime_type: None,
            file_size: None,
            track_id: None,
            tenant_id: None,
            workspace_id: None,
            created_at: None,
            updated_at: None,
            processed_at: None,
            lineage: None,
            metadata: None,
            pdf_id: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-789"));
        assert!(json.contains("processed"));
    }

    #[test]
    fn test_delete_document_response_serialization() {
        let response = DeleteDocumentResponse {
            document_id: "doc-to-delete".to_string(),
            deleted: true,
            chunks_deleted: 7,
            entities_affected: 2,
            relationships_affected: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-to-delete"));
        assert!(json.contains("\"deleted\":true"));
        assert!(json.contains("\"chunks_deleted\":7"));
    }

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_default_page_size() {
        assert_eq!(default_page_size(), 20);
    }

    #[test]
    fn test_track_status_response_serialization() {
        let response = TrackStatusResponse {
            track_id: "upload_20240101_abc12345".to_string(),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            documents: vec![DocumentSummary {
                id: "doc-1".to_string(),
                title: Some("Test Doc".to_string()),
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 5,
                entity_count: Some(3),
                status: Some("completed".to_string()),
                error_message: None,
                track_id: Some("upload_20240101_abc12345".to_string()),
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                // SPEC-002 fields
                source_type: None,
                current_stage: Some("completed".to_string()),
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            }],
            total_count: 1,
            status_summary: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
            },
            is_complete: true,
            latest_message: Some("All documents processed successfully".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("upload_20240101_abc12345"));
        assert!(json.contains("\"is_complete\":true"));
        assert!(json.contains("\"total_count\":1"));
    }
}
