//! Task queue backpressure assessment (SPEC-024 pass 10 / 008 system engineering).
//!
//! SSOT for operator-facing queue pressure labels in `/health` and queue-metrics.

/// Queue depth pressure level for dashboards and alerting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuePressureLevel {
    Normal,
    Elevated,
    Critical,
}

impl QueuePressureLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Elevated => "elevated",
            Self::Critical => "critical",
        }
    }
}

/// Snapshot of queue depth vs configured thresholds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuePressureSnapshot {
    pub level: QueuePressureLevel,
    pub pending_warn_threshold: u64,
    pub pending_critical_threshold: u64,
    pub operator_action: Option<String>,
}

/// Pending count above which operators should investigate backlog (`EDGEQUAKE_QUEUE_PENDING_WARN`).
pub fn queue_pending_warn_threshold() -> u64 {
    std::env::var("EDGEQUAKE_QUEUE_PENDING_WARN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
}

/// Pending count indicating sustained overload (`EDGEQUAKE_QUEUE_PENDING_CRITICAL`).
pub fn queue_pending_critical_threshold() -> u64 {
    std::env::var("EDGEQUAKE_QUEUE_PENDING_CRITICAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| queue_pending_warn_threshold().saturating_mul(5).max(500))
}

/// Assess queue pressure from pending task depth.
pub fn assess_queue_pressure(pending: u64) -> QueuePressureSnapshot {
    let warn = queue_pending_warn_threshold();
    let critical = queue_pending_critical_threshold().max(warn.saturating_add(1));

    let (level, operator_action) = if pending >= critical {
        (
            QueuePressureLevel::Critical,
            Some(format!(
                "Task queue backlog critical ({pending} pending >= {critical}). \
                 Scale workers (EDGEQUAKE_WORKER_COUNT) or reduce ingest rate; \
                 monitor /api/v1/pipeline/queue-metrics"
            )),
        )
    } else if pending >= warn {
        (
            QueuePressureLevel::Elevated,
            Some(format!(
                "Task queue backlog elevated ({pending} pending >= {warn}). \
                 Watch queue-metrics and worker utilization"
            )),
        )
    } else {
        (QueuePressureLevel::Normal, None)
    };

    QueuePressureSnapshot {
        level,
        pending_warn_threshold: warn,
        pending_critical_threshold: critical,
        operator_action,
    }
}

/// True when `/health` should report `degraded` due to queue overload.
pub fn health_degraded_by_queue(pending: u64) -> bool {
    assess_queue_pressure(pending).level == QueuePressureLevel::Critical
}

/// Emit structured log when backlog warrants operator attention (idempotent per request).
pub fn log_queue_pressure(
    snapshot: &QueuePressureSnapshot,
    pending: u64,
    processing: u64,
    failed: u64,
) {
    match snapshot.level {
        QueuePressureLevel::Normal => {}
        QueuePressureLevel::Elevated => {
            tracing::warn!(
                target: "edgequake.task_queue",
                pending,
                processing,
                failed,
                pressure = snapshot.level.as_str(),
                warn_threshold = snapshot.pending_warn_threshold,
                critical_threshold = snapshot.pending_critical_threshold,
                operator_action = snapshot.operator_action.as_deref(),
                "Task queue backlog elevated"
            );
        }
        QueuePressureLevel::Critical => {
            tracing::error!(
                target: "edgequake.task_queue",
                pending,
                processing,
                failed,
                pressure = snapshot.level.as_str(),
                warn_threshold = snapshot.pending_warn_threshold,
                critical_threshold = snapshot.pending_critical_threshold,
                operator_action = snapshot.operator_action.as_deref(),
                "Task queue backlog critical — /health degraded"
            );
        }
    }
}

/// Record Prometheus gauges + optional pressure log (DRY hook for health/metrics handlers).
pub fn publish_queue_observability(
    pending: u64,
    processing: u64,
    failed: u64,
    snapshot: &QueuePressureSnapshot,
) {
    edgequake_observability::record_task_queue_stats(pending, processing, failed);
    log_queue_pressure(snapshot, pending, processing, failed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_env<F: FnOnce()>(key: &str, value: Option<&str>, f: F) {
        let prev = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        f();
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn assess_pressure_from_env_thresholds() {
        with_env("EDGEQUAKE_QUEUE_PENDING_WARN", Some("10"), || {
            with_env("EDGEQUAKE_QUEUE_PENDING_CRITICAL", Some("20"), || {
                assert_eq!(assess_queue_pressure(5).level, QueuePressureLevel::Normal);
                assert_eq!(
                    assess_queue_pressure(10).level,
                    QueuePressureLevel::Elevated
                );
                assert_eq!(
                    assess_queue_pressure(20).level,
                    QueuePressureLevel::Critical
                );
                assert!(health_degraded_by_queue(20));
                assert!(!health_degraded_by_queue(10));
            });
        });
    }
}
