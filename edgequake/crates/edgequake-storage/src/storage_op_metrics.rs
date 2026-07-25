//! SPEC-060: thin storage-op duration hooks (labels: `op` only).

use std::time::Instant;

/// Records wall time for a storage op when dropped (observability feature).
pub struct TimedStorageOp {
    #[allow(dead_code)] // used when `observability` feature is enabled
    op: &'static str,
    #[allow(dead_code)] // used when `observability` feature is enabled
    start: Instant,
}

impl TimedStorageOp {
    pub fn start(op: &'static str) -> Self {
        Self {
            op,
            start: Instant::now(),
        }
    }

    /// Start a timer using a SPEC-088 data-layer Ref ID (same string as metrics `op` label).
    pub fn start_dataop(ref_id: &'static str) -> Self {
        Self::start(ref_id)
    }
}

impl Drop for TimedStorageOp {
    fn drop(&mut self) {
        #[cfg(feature = "observability")]
        edgequake_observability::record_storage_op_duration(
            self.op,
            self.start.elapsed().as_secs_f64(),
        );
    }
}
