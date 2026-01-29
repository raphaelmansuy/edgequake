//! End-to-end performance tests for document upload under progressive load.
//!
//! # Purpose
//!
//! This test suite evaluates system performance characteristics under increasing load:
//! - Baseline single-upload latency
//! - Concurrent upload throughput
//! - System behavior under stress
//! - Recovery after high load
//!
//! # Test Phases
//!
//! 1. **Warmup**: Single upload baseline (1 doc)
//! 2. **Light Load**: Low concurrency (5 docs, 2 concurrent)
//! 3. **Medium Load**: Moderate concurrency (10 docs, 5 concurrent)
//! 4. **Heavy Load**: High concurrency (20 docs, 10 concurrent)
//! 5. **Stress Load**: Maximum concurrency (50 docs, 25 concurrent)
//! 6. **Recovery**: Return to light load to verify system recovery
//!
//! # Implements
//!
//! - **FEAT0401**: Document Upload (Text)
//! - **FEAT0402**: Document Upload (File)
//! - **UC0001**: Upload Document
//!
//! # WHY: Progressive Load Testing
//!
//! Starting with low load and incrementally increasing helps identify:
//! - Linear scaling characteristics
//! - Performance degradation patterns
//! - System breaking points
//! - Resource bottlenecks
//! - Recovery capabilities
//!
//! This approach is superior to immediate high-load testing because it provides
//! a complete performance profile rather than just pass/fail at max load.

use axum::{body::Body, http::Request};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::Value;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tower::ServiceExt;

// ============================================================================
// Test Configuration & Metrics
// ============================================================================

/// Performance metrics for a test phase
#[derive(Debug, Clone)]
struct PhaseMetrics {
    phase_name: String,
    total_uploads: usize,
    success_count: usize,
    failure_count: usize,
    min_duration_ms: u64,
    max_duration_ms: u64,
    avg_duration_ms: f64,
    p50_duration_ms: u64,
    p90_duration_ms: u64,
    p95_duration_ms: u64,
    p99_duration_ms: u64,
    throughput_per_sec: f64,
    error_rate_percent: f64,
    total_duration_ms: u64,
}

impl PhaseMetrics {
    /// Create metrics from upload results
    fn from_results(phase_name: &str, results: &[UploadResult], total_duration: Duration) -> Self {
        let success_results: Vec<_> = results.iter().filter(|r| r.success).collect();
        let mut durations: Vec<u64> = success_results.iter().map(|r| r.duration_ms).collect();
        durations.sort_unstable();

        let success_count = success_results.len();
        let failure_count = results.len() - success_count;

        let avg_duration_ms = if !durations.is_empty() {
            durations.iter().sum::<u64>() as f64 / durations.len() as f64
        } else {
            0.0
        };

        let throughput_per_sec = if total_duration.as_secs_f64() > 0.0 {
            success_count as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        let error_rate_percent = if !results.is_empty() {
            (failure_count as f64 / results.len() as f64) * 100.0
        } else {
            0.0
        };

        Self {
            phase_name: phase_name.to_string(),
            total_uploads: results.len(),
            success_count,
            failure_count,
            min_duration_ms: durations.first().copied().unwrap_or(0),
            max_duration_ms: durations.last().copied().unwrap_or(0),
            avg_duration_ms,
            p50_duration_ms: percentile(&durations, 50),
            p90_duration_ms: percentile(&durations, 90),
            p95_duration_ms: percentile(&durations, 95),
            p99_duration_ms: percentile(&durations, 99),
            throughput_per_sec,
            error_rate_percent,
            total_duration_ms: total_duration.as_millis() as u64,
        }
    }

    /// Print formatted metrics
    fn print(&self) {
        println!("\n{}", "=".repeat(80));
        println!("📊 {}", self.phase_name);
        println!("{}", "=".repeat(80));
        println!(
            "Total: {:4} | Success: {:4} | Failed: {:4} | Error Rate: {:6.2}%",
            self.total_uploads, self.success_count, self.failure_count, self.error_rate_percent
        );
        println!(
            "Throughput: {:.2} docs/sec | Total Duration: {}ms",
            self.throughput_per_sec, self.total_duration_ms
        );
        println!("{}", "-".repeat(80));
        println!("Response Times (ms):");
        println!(
            "  Min: {:5} | Max: {:5} | Avg: {:7.0}",
            self.min_duration_ms, self.max_duration_ms, self.avg_duration_ms
        );
        println!(
            "  P50: {:5} | P90: {:5} | P95: {:5} | P99: {:5}",
            self.p50_duration_ms, self.p90_duration_ms, self.p95_duration_ms, self.p99_duration_ms
        );
        println!("{}", "=".repeat(80));
    }
}

/// Result of a single upload
#[derive(Debug, Clone)]
struct UploadResult {
    success: bool,
    duration_ms: u64,
    #[allow(dead_code)]
    document_id: Option<String>,
    #[allow(dead_code)]
    status_code: u16,
    #[allow(dead_code)]
    error: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate percentile from sorted array
fn percentile(sorted: &[u64], p: u8) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len() as f64 * p as f64 / 100.0).ceil() as usize).saturating_sub(1);
    sorted[index.min(sorted.len() - 1)]
}

/// Generate test document content
fn generate_test_content(size: ContentSize) -> String {
    let base = r#"
# Performance Test Document

## Introduction
This document is generated for performance testing of the EdgeQuake system.
It contains structured content designed to test entity extraction and 
relationship discovery under load.

## Key Entities
- **System**: EdgeQuake RAG Framework
- **Technology**: Rust, Axum, PostgreSQL
- **Capability**: Knowledge Graph Construction
- **Process**: Entity Extraction, Relationship Mapping

## Technical Details
The EdgeQuake system processes documents through a pipeline that:
1. Chunks content into manageable segments
2. Extracts entities using LLM-based analysis
3. Discovers relationships between entities
4. Stores results in graph and vector databases

### Performance Characteristics
- Concurrent Processing: Multiple documents simultaneously
- Async Operations: Non-blocking task execution  
- Resource Management: Efficient CPU and memory utilization
- Scalability: Linear scaling with controlled degradation

## Use Cases
Performance testing helps identify:
- Throughput capacity under various loads
- Response time distributions
- System breaking points
- Recovery characteristics after stress

## Conclusion
Progressive load testing provides comprehensive performance profiles.
"#;

    match size {
        ContentSize::Small => base[..500.min(base.len())].to_string(),
        ContentSize::Medium => base.repeat(3),
        ContentSize::Large => base.repeat(10),
    }
}

#[derive(Debug, Clone, Copy)]
enum ContentSize {
    Small,
    Medium,
    Large,
}

/// Create test server instance
fn create_test_server() -> Server {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    Server::new(config, AppState::test_state())
}

/// Upload a document and measure performance
async fn upload_document(app: axum::Router, content: String, title: String) -> UploadResult {
    let start = Instant::now();

    let body = serde_json::json!({
        "content": content,
        "title": title,
        "async_processing": true,
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/documents")
        .header("content-type", "application/json")
        .header("x-workspace-id", "perf-test-workspace")
        .body(Body::from(body.to_string()))
        .unwrap();

    match app.oneshot(request).await {
        Ok(response) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let status = response.status();

            let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap_or_default();

            let json_result: Result<Value, _> = serde_json::from_slice(&bytes);

            let (document_id, error) = match json_result {
                Ok(json) => (
                    json.get("document_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    if !status.is_success() {
                        Some(format!("Status {}: {:?}", status, json))
                    } else {
                        None
                    },
                ),
                Err(e) => (None, Some(format!("Parse error: {}", e))),
            };

            UploadResult {
                success: status.is_success(),
                duration_ms,
                document_id,
                status_code: status.as_u16(),
                error,
            }
        }
        Err(e) => UploadResult {
            success: false,
            duration_ms: start.elapsed().as_millis() as u64,
            document_id: None,
            status_code: 0,
            error: Some(format!("Request error: {}", e)),
        },
    }
}

/// Execute concurrent uploads with controlled concurrency
async fn execute_concurrent_uploads(
    count: usize,
    concurrency: usize,
    content_size: ContentSize,
) -> Vec<UploadResult> {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let results = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let completed = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];

    for i in 0..count {
        let sem = semaphore.clone();
        let res = results.clone();
        let comp = completed.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            let content = generate_test_content(content_size);
            let title = format!("perf-test-{}-{}", chrono::Utc::now().timestamp_millis(), i);

            let app = create_test_server().build_router();
            let result = upload_document(app, content, title).await;

            res.lock().await.push(result);
            let done = comp.fetch_add(1, Ordering::SeqCst) + 1;

            if done % 10 == 0 || done == count {
                println!("  Progress: {}/{} uploads completed", done, count);
            }
        });

        handles.push(handle);
    }

    // Wait for all uploads to complete
    for handle in handles {
        let _ = handle.await;
    }

    Arc::try_unwrap(results).unwrap().into_inner()
}

// ============================================================================
// Test Suite
// ============================================================================

#[tokio::test]
#[ignore] // Run with: cargo test --test e2e_upload_performance -- --ignored --nocapture
async fn test_progressive_load_performance() {
    println!("\n{}", "=".repeat(80));
    println!("🚀 UPLOAD PERFORMANCE - PROGRESSIVE LOAD TESTING");
    println!("{}", "=".repeat(80));
    println!("Start Time: {}", chrono::Utc::now().to_rfc3339());
    println!("{}", "=".repeat(80));

    let mut all_metrics = Vec::new();

    // Phase 0: Warmup - Single Upload Baseline
    println!("\n🔥 Phase 0: Warmup - Establishing Baseline");
    let start = Instant::now();
    let results = execute_concurrent_uploads(1, 1, ContentSize::Small).await;
    let metrics = PhaseMetrics::from_results("Phase 0: Warmup", &results, start.elapsed());
    metrics.print();
    all_metrics.push(metrics.clone());

    assert_eq!(metrics.success_count, 1, "Warmup should succeed");
    assert!(
        metrics.avg_duration_ms < 5000.0,
        "Baseline latency too high: {}ms",
        metrics.avg_duration_ms
    );

    // Phase 1: Light Load - 5 Concurrent Uploads
    println!("\n⚡ Phase 1: Light Load");
    let start = Instant::now();
    let results = execute_concurrent_uploads(10, 2, ContentSize::Small).await;
    let metrics = PhaseMetrics::from_results("Phase 1: Light Load", &results, start.elapsed());
    metrics.print();
    all_metrics.push(metrics.clone());

    assert!(
        metrics.success_count > 0,
        "No successful uploads in light load"
    );
    assert!(
        metrics.error_rate_percent < 10.0,
        "Error rate too high: {:.1}%",
        metrics.error_rate_percent
    );

    // Phase 2: Medium Load - 10 Concurrent Uploads
    println!("\n🚀 Phase 2: Medium Load");
    let start = Instant::now();
    let results = execute_concurrent_uploads(20, 5, ContentSize::Medium).await;
    let metrics = PhaseMetrics::from_results("Phase 2: Medium Load", &results, start.elapsed());
    metrics.print();
    all_metrics.push(metrics.clone());

    assert!(
        metrics.success_count > 0,
        "No successful uploads in medium load"
    );
    assert!(
        metrics.error_rate_percent < 20.0,
        "Error rate too high: {:.1}%",
        metrics.error_rate_percent
    );

    // Phase 3: Heavy Load - 20 Concurrent Uploads
    println!("\n💪 Phase 3: Heavy Load");
    let start = Instant::now();
    let results = execute_concurrent_uploads(40, 10, ContentSize::Medium).await;
    let metrics = PhaseMetrics::from_results("Phase 3: Heavy Load", &results, start.elapsed());
    metrics.print();
    all_metrics.push(metrics.clone());

    assert!(
        metrics.success_count > 0,
        "No successful uploads in heavy load"
    );
    assert!(
        metrics.throughput_per_sec > 0.5,
        "Throughput too low: {:.2} docs/sec",
        metrics.throughput_per_sec
    );

    // Phase 4: Stress Load - 50 Concurrent Uploads
    println!("\n🔴 Phase 4: Stress Load");
    let start = Instant::now();
    let results = execute_concurrent_uploads(100, 25, ContentSize::Large).await;
    let metrics = PhaseMetrics::from_results("Phase 4: Stress Load", &results, start.elapsed());
    metrics.print();
    all_metrics.push(metrics.clone());

    assert!(
        metrics.success_count > 0,
        "System completely failed under stress"
    );
    assert!(
        metrics.error_rate_percent < 50.0,
        "Error rate too high: {:.1}%",
        metrics.error_rate_percent
    );

    // Phase 5: Recovery - Return to Light Load
    println!("\n🔄 Phase 5: Recovery Test");
    println!("Waiting 5 seconds for system recovery...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    let start = Instant::now();
    let results = execute_concurrent_uploads(10, 2, ContentSize::Small).await;
    let metrics = PhaseMetrics::from_results("Phase 5: Recovery", &results, start.elapsed());
    metrics.print();
    all_metrics.push(metrics.clone());

    // Compare with Phase 1
    let phase1 = &all_metrics[1]; // Phase 1 metrics
    let degradation =
        ((metrics.avg_duration_ms - phase1.avg_duration_ms) / phase1.avg_duration_ms) * 100.0;

    println!("\n📊 Recovery Analysis:");
    println!(
        "  Phase 1 Avg: {:.0}ms | Phase 5 Avg: {:.0}ms | Change: {:+.1}%",
        phase1.avg_duration_ms, metrics.avg_duration_ms, degradation
    );

    assert!(
        degradation.abs() < 50.0,
        "System did not recover properly: {:+.1}% degradation",
        degradation
    );

    // Summary Report
    println!("\n{}", "=".repeat(80));
    println!("📈 COMPARATIVE ANALYSIS");
    println!("{}", "=".repeat(80));
    println!("Phase                    | Uploads | Avg (ms) | P95 (ms) | Throughput/s | Error %");
    println!("{}", "-".repeat(80));

    for m in &all_metrics {
        println!(
            "{:<24} | {:>7} | {:>8.0} | {:>8} | {:>12.2} | {:>7.2}",
            m.phase_name,
            m.total_uploads,
            m.avg_duration_ms,
            m.p95_duration_ms,
            m.throughput_per_sec,
            m.error_rate_percent
        );
    }

    println!("{}", "=".repeat(80));
    println!("End Time: {}", chrono::Utc::now().to_rfc3339());
    println!("{}", "=".repeat(80));
}

#[tokio::test]
#[ignore]
async fn test_sustained_load() {
    println!("\n⏱️  Sustained Load Test - 2 Minutes");
    println!("{}", "=".repeat(80));

    let duration = Duration::from_secs(120); // 2 minutes
    let target_rate = 5; // 5 uploads per minute
    let interval = Duration::from_millis(60_000 / target_rate);

    let start = Instant::now();
    let mut results = Vec::new();
    let mut count = 0;

    while start.elapsed() < duration {
        let content = generate_test_content(ContentSize::Small);
        let title = format!(
            "sustained-{}-{}",
            chrono::Utc::now().timestamp_millis(),
            count
        );

        let app = create_test_server().build_router();
        let result = upload_document(app, content, title).await;
        results.push(result);

        count += 1;
        if count % 5 == 0 {
            println!(
                "  {}s elapsed - {} uploads completed",
                start.elapsed().as_secs(),
                count
            );
        }

        // Wait for next interval
        tokio::time::sleep(interval).await;
    }

    let metrics = PhaseMetrics::from_results("Sustained Load", &results, start.elapsed());
    metrics.print();

    assert!(count >= 10, "Too few uploads completed: {}", count);
    assert!(
        metrics.error_rate_percent < 10.0,
        "Error rate too high: {:.1}%",
        metrics.error_rate_percent
    );
}
