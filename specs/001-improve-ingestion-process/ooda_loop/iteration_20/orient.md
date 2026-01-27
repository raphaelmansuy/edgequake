# Iteration 20: Orient

## Gap Analysis

### What Exists

- ✅ QueueMetrics struct in backend
- ✅ get_queue_metrics() implemented in storage backends
- ✅ TaskStorage trait updated
- ✅ Pipeline endpoints exist (/status, /cancel)

### What's Missing

- ❌ API endpoint to expose QueueMetrics
- ❌ DTO for HTTP response
- ❌ Route registration

## API Design

### Endpoint

```
GET /api/v1/pipeline/queue-metrics

Response 200:
{
  "pending_count": 12,
  "processing_count": 3,
  "active_workers": 3,
  "max_workers": 4,
  "worker_utilization": 75,
  "avg_wait_time_seconds": 45.2,
  "max_wait_time_seconds": 120.5,
  "throughput_per_minute": 2.3,
  "estimated_queue_time_seconds": 312.0,
  "rate_limited": false,
  "timestamp": "2025-01-28T10:30:00Z"
}
```

### DTO Structure

```rust
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueueMetricsResponse {
    pub pending_count: u64,
    pub processing_count: u64,
    pub active_workers: u32,
    pub max_workers: u32,
    pub worker_utilization: u8,
    pub avg_wait_time_seconds: f64,
    pub max_wait_time_seconds: f64,
    pub throughput_per_minute: f64,
    pub estimated_queue_time_seconds: f64,
    pub rate_limited: bool,
    pub timestamp: String,
}
```

## Alignment with Mission Spec

From Objective B requirements:
| Metric | QueueMetrics Field | ✅ |
|---------------------------|-----------------------------|----|
| Queue depth | pending_count | ✅ |
| Processing count | processing_count | ✅ |
| Active workers | active_workers | ✅ |
| Worker utilization | worker_utilization | ✅ |
| Avg wait time | avg_wait_time_seconds | ✅ |
| Processing rate | throughput_per_minute | ✅ |
| Estimated queue time | estimated_queue_time_seconds| ✅ |

## Implementation Effort

- Small: ~50 lines of code
- Low risk: Follows established patterns
- No breaking changes
