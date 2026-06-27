//! Task delivery mode from environment (SPEC-026 P-12).

/// How tasks are delivered to workers after Postgres persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskDeliveryMode {
    /// In-process channel only (default monolith).
    #[default]
    Local,
    /// Notify external bridge AND local channel (migration / hybrid).
    Bridged,
    /// Notify only — workers hydrate from Postgres SSOT.
    NotifyOnly,
}

/// Parse `EDGEQUAKE_TASK_DELIVERY` env var.
pub fn parse_delivery_mode(raw: &str) -> TaskDeliveryMode {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bridged" | "bridge" | "dual" => TaskDeliveryMode::Bridged,
        "notify_only" | "notify-only" | "external" => TaskDeliveryMode::NotifyOnly,
        _ => TaskDeliveryMode::Local,
    }
}

/// Read delivery mode from environment.
pub fn delivery_mode_from_env() -> TaskDeliveryMode {
    std::env::var("EDGEQUAKE_TASK_DELIVERY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(|v| parse_delivery_mode(&v))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_mode_from_env_parses() {
        assert_eq!(parse_delivery_mode("local"), TaskDeliveryMode::Local);
        assert_eq!(parse_delivery_mode("bridged"), TaskDeliveryMode::Bridged);
        assert_eq!(
            parse_delivery_mode("notify_only"),
            TaskDeliveryMode::NotifyOnly
        );
    }
}
