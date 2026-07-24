//! Document DTO types for the documents API handlers.
//!
//! This module contains all request/response data transfer objects used by
//! the document management endpoints. Extracted from documents.rs for modularity.

mod defaults;
mod detail;
mod listing;
mod mutation;
mod recovery;
mod tracking;
mod upload;

pub use defaults::*;
pub use detail::*;
pub use listing::*;
pub use mutation::*;
pub use recovery::*;
pub use tracking::*;
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
            chunk_strategy: None,
            chunk_options: None,
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
            warning_message: None,
            track_id: Some("upload_20240101_abc12345".to_string()),
            created_at: None,
            updated_at: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            llm_model: None,
            embedding_model: None,
            source_type: Some("markdown".to_string()),
            current_stage: Some("completed".to_string()),
            stage_progress: Some(1.0),
            stage_message: None,
            pdf_id: None,
            display_status: None,
            ui_phase: None,
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
                warning_message: None,
                track_id: None,
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                source_type: None,
                current_stage: Some("completed".to_string()),
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
                display_status: None,
                ui_phase: None,
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
                unknown: 0,
            },
            truncated: None,
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
            warning_message: None,
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
            multimodal_summary: None,
            multimodal_items: None,
            display_status: Some("completed".to_string()),
            ui_phase: Some("terminal".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-789"));
        assert!(json.contains("processed"));
        assert!(json.contains("display_status"));
        assert!(json.contains("ui_phase"));
    }

    #[test]
    fn test_delete_document_response_serialization() {
        let response = DeleteDocumentResponse {
            document_id: "doc-to-delete".to_string(),
            deleted: true,
            accepted: false,
            track_id: None,
            chunks_deleted: 7,
            entities_affected: 2,
            relationships_affected: 1,
            embeddings_deleted: 0,
            partial_failure: false,
            partial_failure_reason: None,
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
                warning_message: None,
                track_id: Some("upload_20240101_abc12345".to_string()),
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                source_type: None,
                current_stage: Some("completed".to_string()),
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
                display_status: None,
                ui_phase: None,
            }],
            total_count: 1,
            status_summary: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
                unknown: 0,
            },
            is_complete: true,
            expected_count: None,
            registered_count: 1,
            latest_message: Some("All documents processed successfully".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("upload_20240101_abc12345"));
        assert!(json.contains("\"is_complete\":true"));
        assert!(json.contains("\"total_count\":1"));
    }
}
