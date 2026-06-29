//! SPEC-024 pass 10–11 — task queue backpressure SSOT contracts.

#[test]
fn contract_task_queue_pressure_ssot_module() {
    let src = include_str!("../src/task_queue_pressure.rs");
    assert!(src.contains("EDGEQUAKE_QUEUE_PENDING_WARN"));
    assert!(src.contains("EDGEQUAKE_QUEUE_PENDING_CRITICAL"));
    assert!(src.contains("QueuePressureLevel"));
    assert!(
        src.contains("publish_queue_observability"),
        "pressure SSOT must publish Prometheus + structured logs"
    );
    assert!(
        src.contains("record_task_queue_stats"),
        "pressure SSOT must wire Prometheus gauges"
    );
}

#[test]
fn contract_health_uses_task_queue_pressure() {
    let health = include_str!("../src/handlers/health.rs");
    assert!(health.contains("assess_queue_pressure"));
    assert!(health.contains("health_degraded_by_queue"));
    assert!(
        health.contains("publish_queue_observability"),
        "/health must publish queue observability on every probe"
    );
}

#[test]
fn contract_pipeline_metrics_use_task_queue_pressure() {
    let pipeline = include_str!("../src/handlers/pipeline.rs");
    assert!(
        pipeline.contains("task_queue_pressure::assess_queue_pressure"),
        "queue-metrics must reuse pressure SSOT"
    );
    assert!(
        pipeline.contains("publish_queue_observability"),
        "queue-metrics must publish Prometheus gauges"
    );
}

#[test]
fn contract_prometheus_task_queue_gauges() {
    let metrics = include_str!("../../edgequake-observability/src/metrics.rs");
    assert!(metrics.contains("edgequake_task_queue_pending"));
    assert!(metrics.contains("edgequake_task_queue_processing"));
    assert!(metrics.contains("edgequake_task_queue_failed"));
}

#[test]
fn contract_migration_bootstrap_split_by_concern() {
    assert!(
        std::path::Path::new("src/state/migration_bootstrap/mod.rs").exists(),
        "orchestration must live in mod.rs"
    );
    assert!(
        std::path::Path::new("src/state/migration_bootstrap/helpers.rs").exists(),
        "shared migration helpers must be isolated"
    );
    for migration in [
        "m038", "m040", "m042", "m043", "m044", "m045", "m046", "m047",
    ] {
        let path = format!("src/state/migration_bootstrap/reconcile/{migration}.rs");
        assert!(
            std::path::Path::new(&path).exists(),
            "migration {migration} reconcile must be isolated"
        );
    }
}
