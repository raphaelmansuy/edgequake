//! SPEC-112 LAW-112-3 — shared-DB connection slot budget.
//!
//! `need = total_pool_max × instances` must fit under
//! `max_connections − superuser_reserved − reserved_connections − tools_headroom`.

use sqlx::PgPool;

/// Default headroom for DBeaver / psql / autovacuum beyond superuser reserved.
pub const DEFAULT_TOOLS_HEADROOM: u32 = 10;

/// How to react when the fleet budget is exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetMode {
    /// Log a warning and continue (default — upgrade-safe).
    Warn,
    /// Refuse boot / return error.
    Fail,
}

impl BudgetMode {
    /// `EDGEQUAKE_DB_POOL_BUDGET_MODE=warn|fail` (default warn).
    pub fn from_env() -> Self {
        match std::env::var("EDGEQUAKE_DB_POOL_BUDGET_MODE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "fail" | "error" | "strict" => Self::Fail,
            _ => Self::Warn,
        }
    }
}

/// Pure budget evaluation result (no I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolBudgetReport {
    pub total_pool_max: u32,
    pub instances: u32,
    pub need: u32,
    pub pg_max: u32,
    pub reserved: u32,
    pub tools_headroom: u32,
    pub limit: u32,
    pub ok: bool,
    pub mode: BudgetMode,
}

impl PoolBudgetReport {
    pub fn operator_action(&self) -> Option<String> {
        if self.ok {
            return None;
        }
        Some(format!(
            "Pool budget exceeded: need={} ({} instances × {} pool max) > limit={} \
             (max_connections={} − reserved={} − headroom={}). \
             Lower EDGEQUAKE_DB_POOL_SIZE_* or instances, or raise shared PG capacity via PgBouncer — \
             do not treat max_connections=400 as the product fix (SPEC-112 LAW-112-6).",
            self.need,
            self.instances,
            self.total_pool_max,
            self.limit,
            self.pg_max,
            self.reserved,
            self.tools_headroom
        ))
    }
}

/// `EDGEQUAKE_DB_POOL_INSTANCE_COUNT` (default 1; clamp 1..=256).
pub fn pool_instance_count_from_env() -> u32 {
    std::env::var("EDGEQUAKE_DB_POOL_INSTANCE_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .clamp(1, 256)
}

/// Evaluate fleet slot budget (LAW-112-3).
pub fn evaluate_pool_budget(
    total_pool_max: u32,
    instances: u32,
    pg_max: u32,
    reserved: u32,
    tools_headroom: u32,
    mode: BudgetMode,
) -> PoolBudgetReport {
    let instances = instances.max(1);
    let need = total_pool_max.saturating_mul(instances);
    let reserve_total = reserved.saturating_add(tools_headroom);
    let limit = pg_max.saturating_sub(reserve_total);
    let ok = need <= limit;
    PoolBudgetReport {
        total_pool_max,
        instances,
        need,
        pg_max,
        reserved,
        tools_headroom,
        limit,
        ok,
        mode,
    }
}

/// Read slot capacity in one round trip, including PG 16+ reserved connections.
/// Missing `reserved_connections` on older servers contributes zero.
pub async fn check_pool_budget(
    pool: &PgPool,
    total_pool_max: u32,
) -> Result<PoolBudgetReport, sqlx::Error> {
    let (pg_max, reserved): (i32, i32) = sqlx::query_as(
        "SELECT current_setting('max_connections')::integer, \
                current_setting('superuser_reserved_connections')::integer + \
                COALESCE(current_setting('reserved_connections', true)::integer, 0)",
    )
    .fetch_one(pool)
    .await?;
    let report = evaluate_pool_budget(
        total_pool_max,
        pool_instance_count_from_env(),
        pg_max.max(0) as u32,
        reserved.max(0) as u32,
        DEFAULT_TOOLS_HEADROOM,
        BudgetMode::from_env(),
    );
    Ok(report)
}

/// Apply mode: Ok(report) always on Warn; Err when Fail && !ok.
pub fn enforce_pool_budget(report: &PoolBudgetReport) -> Result<(), String> {
    if report.ok {
        return Ok(());
    }
    let msg = report
        .operator_action()
        .unwrap_or_else(|| "pool budget exceeded".to_string());
    match report.mode {
        BudgetMode::Warn => {
            tracing::warn!(
                need = report.need,
                limit = report.limit,
                instances = report.instances,
                total_pool_max = report.total_pool_max,
                pg_max = report.pg_max,
                "SPEC-112: {msg}"
            );
            Ok(())
        }
        BudgetMode::Fail => Err(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_ok_under_limit() {
        let r = evaluate_pool_budget(34, 1, 100, 3, 10, BudgetMode::Warn);
        assert!(r.ok);
        assert_eq!(r.need, 34);
        assert_eq!(r.limit, 87);
    }

    #[test]
    fn budget_fail_with_overlap_instances() {
        // 4×34 = 136 > limit 87
        let r = evaluate_pool_budget(34, 4, 100, 3, 10, BudgetMode::Fail);
        assert!(!r.ok);
        assert_eq!(r.need, 136);
        assert!(enforce_pool_budget(&r).is_err());
    }

    #[test]
    fn budget_warn_allows_boot() {
        let r = evaluate_pool_budget(34, 4, 100, 3, 10, BudgetMode::Warn);
        assert!(!r.ok);
        assert!(enforce_pool_budget(&r).is_ok());
    }

    #[test]
    fn saturating_limit_when_reserve_exceeds_max() {
        let r = evaluate_pool_budget(10, 1, 5, 3, 10, BudgetMode::Fail);
        assert_eq!(r.limit, 0);
        assert!(!r.ok);
    }

    #[test]
    fn reserved_slots_change_boundary_admission() {
        assert!(evaluate_pool_budget(87, 1, 100, 3, 10, BudgetMode::Fail).ok);
        let report = evaluate_pool_budget(87, 1, 100, 8, 10, BudgetMode::Fail);
        assert_eq!(report.limit, 82);
        assert!(!report.ok);
        assert!(enforce_pool_budget(&report).is_err());
    }
}
