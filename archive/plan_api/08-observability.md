# Observability & Monitoring

**Version:** 1.0  
**Target Release:** EdgeQuake v2.0.0  
**Priority:** HIGH (Production)  
**Status:** Planning

---

## Overview

Implement comprehensive observability through metrics, tracing, and structured logging for production deployments.

### Goals

1. **Metrics:** Prometheus-compatible metrics export
2. **Tracing:** OpenTelemetry distributed tracing
3. **Logging:** Structured JSON logging with correlation IDs
4. **Dashboards:** Pre-built Grafana dashboards
5. **Alerting:** Key metrics for alerting rules

---

## Architecture

```
┌─────────────────┐
│  EdgeQuake API  │
│   (Tracing)     │
└────────┬────────┘
         │
         ├──▶ OpenTelemetry ──▶ Jaeger/Tempo
         ├──▶ Prometheus ◀──── /metrics
         └──▶ Loki ◀────────── JSON logs
                               
         Visualize in Grafana Dashboard
```

---

## 1. Prometheus Metrics

### Key Metrics

```rust
use prometheus::{IntCounter, IntGauge, Histogram, HistogramVec};

// HTTP Metrics
http_requests_total: Counter
http_request_duration_seconds: Histogram
http_requests_in_flight: Gauge

// Document Metrics
documents_uploaded_total: Counter
documents_indexed_total: Counter
documents_failed_total: Counter
documents_processing_duration_seconds: Histogram

// Query Metrics
queries_total: Counter
query_duration_seconds: Histogram
query_sources_retrieved: Histogram

// Task Metrics
tasks_created_total: Counter
tasks_completed_total: Counter
tasks_failed_total: Counter
task_queue_size: Gauge
task_duration_seconds: Histogram

// Storage Metrics
storage_operations_total: Counter (by operation: read/write/delete)
storage_operation_duration_seconds: Histogram
storage_size_bytes: Gauge (by type: kv/vector/graph)

// LLM Metrics
llm_requests_total: Counter
llm_tokens_used_total: Counter (by type: prompt/completion)
llm_request_duration_seconds: Histogram
llm_errors_total: Counter
```

### Metrics Endpoint

```http
GET /metrics

# HELP edgequake_http_requests_total Total HTTP requests
# TYPE edgequake_http_requests_total counter
edgequake_http_requests_total{method="GET",path="/api/v1/documents",status="200"} 1250

# HELP edgequake_query_duration_seconds Query execution duration
# TYPE edgequake_query_duration_seconds histogram
edgequake_query_duration_seconds_bucket{le="0.1"} 450
edgequake_query_duration_seconds_bucket{le="0.5"} 890
edgequake_query_duration_seconds_bucket{le="1.0"} 1200
edgequake_query_duration_seconds_sum 1250.5
edgequake_query_duration_seconds_count 1250
```

---

## 2. OpenTelemetry Tracing

### Trace Spans

```rust
use tracing::{info, instrument};
use opentelemetry::trace::{Tracer, Span};

#[instrument(
    skip(state, request),
    fields(
        document_id = %request.document_id,
        content_length = request.content.len()
    )
)]
pub async fn upload_document(
    State(state): State<AppState>,
    Json(request): Json<UploadDocumentRequest>,
) -> ApiResult<Json<UploadDocumentResponse>> {
    info!("Starting document upload");
    
    let span = tracing::Span::current();
    span.record("document_id", &request.document_id);
    
    // Processing...
    
    Ok(response)
}
```

### Trace Context Propagation

```http
# Incoming request with trace headers
GET /api/v1/documents HTTP/1.1
traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
tracestate: key1=value1,key2=value2

# EdgeQuake propagates to downstream services (LLM, storage)
```

---

## 3. Structured Logging

### Log Format

```json
{
  "timestamp": "2025-12-22T19:30:00.123Z",
  "level": "INFO",
  "target": "edgequake_api::handlers::documents",
  "message": "Document uploaded successfully",
  "span": {
    "name": "upload_document",
    "trace_id": "0af7651916cd43dd8448eb211c80319c",
    "span_id": "b7ad6b7169203331"
  },
  "fields": {
    "document_id": "doc-xyz789",
    "track_id": "upload-abc123",
    "content_length": 524288,
    "user_id": "user-123",
    "tenant_id": "tenant-456"
  }
}
```

### Log Levels

- **ERROR:** System errors, failures
- **WARN:** Degraded performance, non-fatal issues
- **INFO:** Key events (document uploaded, query executed)
- **DEBUG:** Detailed debugging info
- **TRACE:** Very verbose (disabled in production)

---

## 4. Grafana Dashboards

### Pre-built Dashboards

1. **System Overview**
   - HTTP request rate
   - Error rate
   - Request latency (p50, p95, p99)
   - Active connections
   
2. **Document Processing**
   - Upload rate
   - Processing duration
   - Success/failure ratio
   - Queue depth
   
3. **Query Performance**
   - Query rate
   - Query latency by mode
   - Sources retrieved
   - LLM token usage
   
4. **Storage Health**
   - Storage size trends
   - Operation latency
   - Error rates
   
5. **LLM Usage**
   - Request rate
   - Token consumption
   - Cost estimation
   - Error rate

---

## 5. Alerting Rules

### Critical Alerts

```yaml
groups:
  - name: edgequake_critical
    interval: 1m
    rules:
      - alert: HighErrorRate
        expr: rate(edgequake_http_requests_total{status=~"5.."}[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          
      - alert: HighQueryLatency
        expr: histogram_quantile(0.95, rate(edgequake_query_duration_seconds_bucket[5m])) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Query latency p95 > 5s"
          
      - alert: TaskQueueBacklog
        expr: edgequake_task_queue_size > 1000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Task queue has > 1000 pending tasks"
```

---

## Configuration

```toml
[observability]
# Metrics
metrics_enabled = true
prometheus_port = 9090

# Tracing
tracing_enabled = true
otel_exporter_otlp_endpoint = "http://localhost:4317"
otel_service_name = "edgequake"
trace_sample_rate = 0.1  # Sample 10% of traces

# Logging
log_level = "info"
log_format = "json"  # or "pretty"
log_file = "/var/log/edgequake/app.log"
```

---

## Implementation

```rust
// crates/edgequake-telemetry/src/lib.rs

use opentelemetry::sdk::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;

pub fn init_telemetry(config: &TelemetryConfig) -> Result<(), Error> {
    // Initialize OpenTelemetry
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(&config.service_name)
        .with_endpoint(&config.otlp_endpoint)
        .install_simple()?;
    
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    
    // Initialize tracing subscriber
    let subscriber = tracing_subscriber::registry()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer().json())
        .with(tracing_subscriber::EnvFilter::new(&config.log_level));
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    // Initialize Prometheus metrics
    prometheus::default_registry().register(Box::new(HTTP_REQUESTS_TOTAL.clone()))?;
    
    Ok(())
}
```

---

**Status:** ✅ Specification Complete  
**Dependencies:** OpenTelemetry, Prometheus  
**Next:** Implement telemetry crate and dashboards
