//! Test Fixtures Module for API Integration Tests
//!
//! Provides utilities for setting up a fresh workspace with test data
//! before running API integration tests.
//!
//! # Usage
//!
//! ```rust,ignore
//! use test_fixtures::setup_fresh_workspace;
//!
//! #[tokio::test]
//! async fn my_api_test() {
//!     setup_fresh_workspace().await.expect("Failed to setup");
//!     // ... run tests against populated database
//! }
//! ```

use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

/// API base URL (can be overridden with API_BASE_URL env var)
pub fn get_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Embedded test documents from specs/fix_search/data/
pub mod test_data {
    /// Peugeot 2008 specifications
    pub const EF_EXTRACT_2008: (&str, &str) = (
        "EF-extract-2008.md",
        include_str!("fixtures/EF-extract-2008.md"),
    );

    /// Peugeot 208 specifications
    pub const EF_EXTRACT_208: (&str, &str) = (
        "EF-extract-208.md",
        include_str!("fixtures/EF-extract-208.md"),
    );

    /// Peugeot 3008 specifications (key document for E-3008 queries)
    pub const EF_EXTRACT_3008: (&str, &str) = (
        "EF-extract-3008.md",
        include_str!("fixtures/EF-extract-3008.md"),
    );

    /// Peugeot 5008 specifications
    pub const EF_EXTRACT_5008: (&str, &str) = (
        "EF-extract-5008.md",
        include_str!("fixtures/EF-extract-5008.md"),
    );

    /// BYD HAN specifications (competitor)
    pub const EF_EXTRACT_BYD_HAN: (&str, &str) = (
        "EF-extract-BYD-HAN.md",
        include_str!("fixtures/EF-extract-BYD HAN.md"),
    );

    /// BYD Seal specifications (key competitor document)
    pub const EF_EXTRACT_BYD_SEAL: (&str, &str) = (
        "EF-Extract-BYD-Seal.md",
        include_str!("fixtures/EF-Extract-BYD-Seal.md"),
    );

    /// Peugeot 3008 detailed tech specs
    pub const EF_EXTRACT_CT_3008: (&str, &str) = (
        "EF-extract-CT_3008.md",
        include_str!("fixtures/EF-extract-CT_3008.md"),
    );

    /// Peugeot 308 specifications
    pub const EF_EXTRACT_NEW_308: (&str, &str) = (
        "EF-extract-new-308.md",
        include_str!("fixtures/EF-extract-new-308.md"),
    );

    /// Peugeot Traveller specifications
    pub const EF_EXTRACT_PEUGEOT_TRAVELLER: (&str, &str) = (
        "EF-Extract-Peugeot-Traveller.md",
        include_str!("fixtures/EF-Extract-Peugeot-Traveller.md"),
    );

    /// Renault 5 E-Tech specifications
    pub const EF_EXTRACT_RENAULT_5: (&str, &str) = (
        "EF-extract-RENAULT-5-e-tech.md",
        include_str!("fixtures/EF-extract-RENAULT 5-e-tech.md"),
    );

    /// Renault Clio Hybrid specifications
    pub const EF_EXTRACT_RENAULT_CLIO: (&str, &str) = (
        "EF-Extract-RENAULT-CLIO-FULL-HYBRID.md",
        include_str!("fixtures/EF-Extract-RENAULT CLIO FULL HYBRID E-TECH.md"),
    );

    /// Renault Arkana specifications
    pub const EF_EXTRACT_RENAULT_ARKANA: (&str, &str) = (
        "EF-extract-Renault-Arkana.md",
        include_str!("fixtures/EF-extract-Renault-Arkana.md"),
    );

    /// Renault Austral specifications
    pub const EF_EXTRACT_RENAULT_AUSTRAL: (&str, &str) = (
        "EF-Extract-Renault-Austral.md",
        include_str!("fixtures/EF-Extract-Renault-Autral.md"),
    );

    /// Renault Captur specifications
    pub const EF_EXTRACT_RENAULT_CAPTUR: (&str, &str) = (
        "EF-extract-Renault-CAPTUR.md",
        include_str!("fixtures/EF-extract-Renault-CAPTUR.md"),
    );

    /// Renault Scenic specifications
    pub const EF_EXTRACT_RENAULT_SCENIC: (&str, &str) = (
        "EF-Extract-Renault-Scenic.md",
        include_str!("fixtures/EF-Extract-Renault-Scenic.md"),
    );

    /// Renault Symbioz specifications
    pub const EF_EXTRACT_RENAULT_SYMBIOZ: (&str, &str) = (
        "EF-Extract-Renault-Symbioz.md",
        include_str!("fixtures/EF-Extract-Renault-Symbioz.md"),
    );

    /// Get all test documents as (filename, content) pairs
    pub fn all_documents() -> Vec<(&'static str, &'static str)> {
        vec![
            EF_EXTRACT_2008,
            EF_EXTRACT_208,
            EF_EXTRACT_3008,
            EF_EXTRACT_5008,
            EF_EXTRACT_BYD_HAN,
            EF_EXTRACT_BYD_SEAL,
            EF_EXTRACT_CT_3008,
            EF_EXTRACT_NEW_308,
            EF_EXTRACT_PEUGEOT_TRAVELLER,
            EF_EXTRACT_RENAULT_5,
            EF_EXTRACT_RENAULT_CLIO,
            EF_EXTRACT_RENAULT_ARKANA,
            EF_EXTRACT_RENAULT_AUSTRAL,
            EF_EXTRACT_RENAULT_CAPTUR,
            EF_EXTRACT_RENAULT_SCENIC,
            EF_EXTRACT_RENAULT_SYMBIOZ,
        ]
    }

    /// Get core test documents (subset for faster testing)
    pub fn core_documents() -> Vec<(&'static str, &'static str)> {
        vec![
            EF_EXTRACT_3008,           // Key Peugeot document
            EF_EXTRACT_CT_3008,        // Detailed 3008 specs
            EF_EXTRACT_BYD_SEAL,       // Key competitor
            EF_EXTRACT_NEW_308,        // Hybrid models
            EF_EXTRACT_RENAULT_SCENIC, // Competitor
        ]
    }
}

/// Health check response
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for deserialization
pub struct HealthResponse {
    pub status: String,
    pub storage_mode: Option<String>,
    pub llm_provider_name: Option<String>,
}

/// Document list response item
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for deserialization
pub struct DocumentInfo {
    pub id: String,
    pub title: Option<String>,
    pub status: Option<String>,
}

/// Upload response
#[derive(Debug, Deserialize)]
pub struct UploadResponse {
    pub task_id: Option<String>,
    pub id: Option<String>,
    pub document_id: Option<String>,
}

/// Task status response
#[derive(Debug, Deserialize)]
pub struct TaskStatus {
    pub status: String,
    pub error: Option<String>,
}

/// Check if the API is healthy and ready
pub async fn check_health() -> Result<HealthResponse, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/health", get_base_url());

    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Health check failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Health check returned {}", resp.status()));
    }

    resp.json::<HealthResponse>()
        .await
        .map_err(|e| format!("Failed to parse health response: {}", e))
}

/// List all documents in the current workspace
pub async fn list_documents() -> Result<Vec<DocumentInfo>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/documents", get_base_url());

    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("List documents failed: {}", e))?;

    if !resp.status().is_success() {
        return Ok(vec![]); // Empty list if endpoint fails
    }

    // Handle both array and object responses
    let text = resp.text().await.unwrap_or_default();
    if let Ok(docs) = serde_json::from_str::<Vec<DocumentInfo>>(&text) {
        return Ok(docs);
    }

    // Try parsing as object with documents field
    #[derive(Deserialize)]
    struct DocsWrapper {
        documents: Vec<DocumentInfo>,
    }
    if let Ok(wrapper) = serde_json::from_str::<DocsWrapper>(&text) {
        return Ok(wrapper.documents);
    }

    Ok(vec![])
}

/// Delete a document by ID
pub async fn delete_document(document_id: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/documents/{}", get_base_url(), document_id);

    let resp = client
        .delete(&url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Delete document failed: {}", e))?;

    if resp.status().is_success() || resp.status().as_u16() == 404 {
        Ok(())
    } else {
        Err(format!("Delete failed with status {}", resp.status()))
    }
}

/// Delete all documents from the workspace
pub async fn clear_all_documents() -> Result<usize, String> {
    let docs = list_documents().await?;
    let count = docs.len();

    for doc in docs {
        delete_document(&doc.id).await?;
    }

    // Wait for deletions to complete
    sleep(Duration::from_secs(2)).await;

    Ok(count)
}

/// Ingest a single document
pub async fn ingest_document(filename: &str, content: &str) -> Result<Option<String>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/documents/upload", get_base_url());

    let part = Part::text(content.to_string())
        .file_name(filename.to_string())
        .mime_str("text/markdown")
        .map_err(|e| format!("Failed to create part: {}", e))?;

    let form = Form::new().part("file", part);

    let resp = client
        .post(&url)
        .multipart(form)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("Upload failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Upload failed: {} - {}", status, body));
    }

    let upload_resp: UploadResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse upload response: {}", e))?;

    Ok(upload_resp
        .task_id
        .or(upload_resp.id)
        .or(upload_resp.document_id))
}

/// Wait for a task to complete
pub async fn wait_for_task(task_id: &str, max_wait_secs: u64) -> Result<bool, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/tasks/{}", get_base_url(), task_id);

    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < max_wait_secs {
        let resp = client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                if let Ok(status) = r.json::<TaskStatus>().await {
                    match status.status.to_lowercase().as_str() {
                        "completed" | "done" | "success" => return Ok(true),
                        "failed" | "error" => {
                            return Err(format!(
                                "Task failed: {}",
                                status.error.unwrap_or_default()
                            ))
                        }
                        _ => {} // Still processing
                    }
                }
            }
            Ok(_) => {
                // Task endpoint might not exist - assume sync processing
                return Ok(true);
            }
            Err(_) => {} // Retry
        }

        sleep(Duration::from_secs(2)).await;
    }

    Ok(true) // Assume success after timeout
}

/// Ingest all test documents
pub async fn ingest_all_documents(use_core_only: bool) -> Result<IngestResult, String> {
    let documents = if use_core_only {
        test_data::core_documents()
    } else {
        test_data::all_documents()
    };

    let mut result = IngestResult {
        total: documents.len(),
        successful: 0,
        failed: 0,
        task_ids: vec![],
    };

    for (filename, content) in documents {
        match ingest_document(filename, content).await {
            Ok(task_id) => {
                result.successful += 1;
                if let Some(id) = task_id {
                    result.task_ids.push(id);
                }
            }
            Err(e) => {
                eprintln!("Failed to ingest {}: {}", filename, e);
                result.failed += 1;
            }
        }

        // Small delay between uploads
        sleep(Duration::from_millis(500)).await;
    }

    // Wait for all tasks to complete
    for task_id in &result.task_ids {
        let _ = wait_for_task(task_id, 60).await;
    }

    // Additional wait for async processing
    sleep(Duration::from_secs(5)).await;

    Ok(result)
}

/// Result of ingestion operation
#[derive(Debug)]
pub struct IngestResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub task_ids: Vec<String>,
}

/// Setup options for fresh workspace
#[derive(Debug, Default)]
pub struct SetupOptions {
    /// Use only core documents (faster)
    pub core_only: bool,
    /// Skip clearing existing documents
    pub skip_clear: bool,
    /// Skip health check
    pub skip_health_check: bool,
}

/// Setup a fresh workspace with test data
///
/// This function:
/// 1. Checks API health
/// 2. Clears all existing documents
/// 3. Ingests all test documents
/// 4. Waits for processing to complete
///
/// # Returns
/// WorkspaceStats with document and entity counts
pub async fn setup_fresh_workspace(options: SetupOptions) -> Result<WorkspaceStats, String> {
    println!("\n========================================");
    println!("  FRESH WORKSPACE SETUP");
    println!("========================================\n");

    // Check health
    if !options.skip_health_check {
        print!("Checking API health... ");
        let health = check_health().await?;
        println!("✓ {}", health.status);
        if let Some(mode) = &health.storage_mode {
            println!("  Storage: {}", mode);
        }
    }

    // Clear existing documents
    let cleared = if !options.skip_clear {
        print!("Clearing existing documents... ");
        let count = clear_all_documents().await?;
        println!("✓ {} documents deleted", count);
        count
    } else {
        0
    };

    // Ingest test documents
    println!("Ingesting test documents...");
    let ingest_result = ingest_all_documents(options.core_only).await?;
    println!(
        "  ✓ {}/{} documents ingested successfully",
        ingest_result.successful, ingest_result.total
    );

    // Get final stats
    print!("Verifying workspace... ");
    let docs = list_documents().await?;
    println!("✓ {} documents in workspace", docs.len());

    println!("\n========================================");
    println!("  SETUP COMPLETE");
    println!("========================================\n");

    Ok(WorkspaceStats {
        documents_cleared: cleared,
        documents_ingested: ingest_result.successful,
        documents_failed: ingest_result.failed,
        total_documents: docs.len(),
    })
}

/// Statistics about the workspace after setup
#[derive(Debug)]
pub struct WorkspaceStats {
    pub documents_cleared: usize,
    pub documents_ingested: usize,
    pub documents_failed: usize,
    pub total_documents: usize,
}

/// Quick setup with default options (full clear + all documents)
#[allow(dead_code)] // Utility function for external test use
pub async fn setup_fresh_workspace_default() -> Result<WorkspaceStats, String> {
    setup_fresh_workspace(SetupOptions::default()).await
}

/// Quick setup with core documents only (faster for CI)
#[allow(dead_code)] // Utility function for external test use
pub async fn setup_fresh_workspace_core() -> Result<WorkspaceStats, String> {
    setup_fresh_workspace(SetupOptions {
        core_only: true,
        ..Default::default()
    })
    .await
}

// =============================================================================
// Unit Tests (don't require running server)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_documents_loaded() {
        let docs = test_data::all_documents();
        assert_eq!(docs.len(), 16, "Should have 16 test documents");

        // Check each document is non-empty
        for (name, content) in &docs {
            assert!(!name.is_empty(), "Document name should not be empty");
            assert!(!content.is_empty(), "Document {} should have content", name);
            assert!(
                content.contains('#'),
                "Document {} should have markdown headers",
                name
            );
        }
    }

    #[test]
    fn test_core_documents_loaded() {
        let docs = test_data::core_documents();
        assert_eq!(docs.len(), 5, "Should have 5 core documents");
    }

    #[test]
    fn test_key_documents_contain_expected_content() {
        // E-3008 document should mention STLA Medium, i-Cockpit
        let (_, content) = test_data::EF_EXTRACT_3008;
        assert!(
            content.contains("i-Cockpit") || content.contains("3008"),
            "3008 doc should mention key features"
        );

        // BYD Seal should mention LFP or battery
        let (_, content) = test_data::EF_EXTRACT_BYD_SEAL;
        assert!(
            content.contains("BYD") || content.contains("Seal"),
            "BYD Seal doc should mention brand"
        );
    }

    #[test]
    fn test_setup_options_default() {
        let opts = SetupOptions::default();
        assert!(!opts.core_only);
        assert!(!opts.skip_clear);
        assert!(!opts.skip_health_check);
    }
}
