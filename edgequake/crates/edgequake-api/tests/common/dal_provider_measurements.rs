//! Bounded HTTP fixture measurements; no production throughput claim.
use super::provider_access::http_harness::LiveServer;
use serde_json::{json, Value};
use std::time::Instant;
use uuid::Uuid;

pub async fn measure(
    server: &LiveServer,
    tenant: Uuid,
    workspace: Uuid,
    content: &str,
    node: &str,
) -> Value {
    let mut graph_ms = Vec::new();
    let mut query_ms = Vec::new();
    for i in 0..24 {
        let start = Instant::now();
        let (status, body) = server
            .get_text(&format!("/api/v1/graph/entities/{node}"), tenant, workspace)
            .await;
        assert!(status.is_success(), "graph {status}: {body}");
        assert!(
            body.contains(content),
            "graph must read the selected provider: {body}"
        );
        let graph_time = start.elapsed().as_secs_f64() * 1000.0;
        let start = Instant::now();
        let (status, body) = server
            .query_naive(tenant, workspace, "fixture", None, Some(5))
            .await;
        assert!(status.is_success(), "query {status}: {body}");
        assert!(
            body.contains(content),
            "query must retrieve the projected vector: {body}"
        );
        if i >= 3 {
            graph_ms.push(graph_time);
            query_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    fn stats(mut values: Vec<f64>) -> Value {
        values.sort_by(f64::total_cmp);
        let p95 = values[19];
        assert!(
            p95 < 2000.0,
            "fixture exceeded generous reliability latency budget"
        );
        json!({"samples": values.len(), "p50_ms": values[10], "p95_ms": p95, "max_ms": values[20]})
    }
    json!({"warmups": 3, "graph_http": stats(graph_ms), "vector_query_http": stats(query_ms)})
}
