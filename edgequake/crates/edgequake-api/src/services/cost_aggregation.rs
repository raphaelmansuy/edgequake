//! Document cost aggregation — SPEC-027 IMP-029 / ARCH-004 (DRY).
//!
//! Shared metadata scan + workspace isolation for cost summary and history endpoints.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, Utc};
use edgequake_storage::traits::KVStorage;

use crate::error::ApiResult;
use crate::handlers::costs_types::CostHistoryPoint;
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_scoped_document_metadata;
use crate::workspace_scope::metadata_matches_tenant_context;

/// Whether document status counts toward billing aggregates.
pub fn is_billable_document_status(status: &str) -> bool {
    status == "completed" || status == "indexed"
}

/// Parsed cost fields from a single document metadata record.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentCostRow {
    pub cost_usd: f64,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub processed_at: Option<String>,
}

/// Aggregated workspace totals for `/api/v1/costs/summary`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CostSummaryTotals {
    pub total_cost: f64,
    pub total_input_tokens: usize,
    pub total_output_tokens: usize,
    pub document_count: usize,
    pub extraction_cost: f64,
    pub embedding_cost: f64,
}

/// Load billable document cost rows scoped to tenant/workspace (legacy alias SSOT).
pub async fn load_scoped_document_cost_rows(
    kv_storage: &Arc<dyn KVStorage>,
    tenant_ctx: &TenantContext,
) -> ApiResult<Vec<DocumentCostRow>> {
    let values = load_scoped_document_metadata(kv_storage.as_ref(), tenant_ctx).await?;
    Ok(values
        .iter()
        .filter_map(|value| parse_cost_row(value, tenant_ctx))
        .collect())
}

fn parse_cost_row(
    value: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> Option<DocumentCostRow> {
    if !metadata_matches_tenant_context(value, tenant_ctx) {
        return None;
    }

    let obj = value.as_object()?;
    let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if !is_billable_document_status(status) {
        return None;
    }

    let cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let input_tokens = obj
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let output_tokens = obj
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let processed_at = obj
        .get("processed_at")
        .or_else(|| obj.get("created_at"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    Some(DocumentCostRow {
        cost_usd,
        input_tokens,
        output_tokens,
        processed_at,
    })
}

/// Aggregate summary totals from scoped document rows.
pub fn aggregate_cost_summary(rows: &[DocumentCostRow]) -> CostSummaryTotals {
    let mut totals = CostSummaryTotals::default();

    for row in rows {
        totals.document_count += 1;
        totals.total_cost += row.cost_usd;
        totals.extraction_cost += row.cost_usd * 0.9;
        totals.embedding_cost += row.cost_usd * 0.1;
        totals.total_input_tokens += row.input_tokens;
        totals.total_output_tokens += row.output_tokens;
    }

    totals
}

fn period_key_for_timestamp(dt_utc: DateTime<Utc>, granularity: &str) -> String {
    match granularity {
        "hour" => dt_utc.format("%Y-%m-%dT%H:00:00Z").to_string(),
        "week" => {
            let week_start =
                dt_utc - Duration::days(dt_utc.weekday().num_days_from_monday() as i64);
            week_start.format("%Y-%m-%dT00:00:00Z").to_string()
        }
        "month" => dt_utc.format("%Y-%m-01T00:00:00Z").to_string(),
        _ => dt_utc.format("%Y-%m-%dT00:00:00Z").to_string(),
    }
}

/// Aggregate cost history points from scoped document rows.
pub fn aggregate_cost_history(
    rows: &[DocumentCostRow],
    granularity: &str,
) -> Vec<CostHistoryPoint> {
    let mut period_data: BTreeMap<String, (f64, usize, usize)> = BTreeMap::new();

    for row in rows {
        let Some(ts) = row.processed_at.as_deref() else {
            continue;
        };
        let Ok(dt) = DateTime::parse_from_rfc3339(ts) else {
            continue;
        };
        let dt_utc = dt.with_timezone(&Utc);
        let period_key = period_key_for_timestamp(dt_utc, granularity);
        let entry = period_data.entry(period_key).or_insert((0.0, 0, 0));
        entry.0 += row.cost_usd;
        entry.1 += row.input_tokens + row.output_tokens;
        entry.2 += 1;
    }

    period_data
        .into_iter()
        .map(|(timestamp, (cost, tokens, count))| CostHistoryPoint {
            timestamp,
            total_cost: cost,
            total_tokens: tokens,
            document_count: count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::{default_tenant_uuid, default_workspace_uuid};

    fn ctx(tenant: &str, workspace: &str) -> TenantContext {
        TenantContext {
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            user_id: None,
        }
    }

    #[test]
    fn billable_statuses_only_completed_or_indexed() {
        assert!(is_billable_document_status("completed"));
        assert!(is_billable_document_status("indexed"));
        assert!(!is_billable_document_status("processing"));
        assert!(!is_billable_document_status(""));
    }

    #[test]
    fn parse_cost_row_uses_legacy_default_alias() {
        let metadata = serde_json::json!({
            "tenant_id": "default",
            "workspace_id": "default",
            "status": "completed",
            "cost_usd": 1.25,
            "input_tokens": 100,
            "output_tokens": 50,
            "processed_at": "2026-06-01T12:00:00Z",
        });
        let row = parse_cost_row(&metadata, &ctx("default", "default")).unwrap();
        assert_eq!(row.cost_usd, 1.25);
        assert_eq!(row.input_tokens, 100);
        assert_eq!(row.output_tokens, 50);
    }

    #[test]
    fn parse_cost_row_hides_other_workspace() {
        let metadata = serde_json::json!({
            "tenant_id": default_tenant_uuid().to_string(),
            "workspace_id": default_workspace_uuid().to_string(),
            "status": "completed",
            "cost_usd": 9.0,
        });
        let other = uuid::Uuid::new_v4().to_string();
        assert!(parse_cost_row(&metadata, &ctx("default", &other)).is_none());
    }

    #[test]
    fn aggregate_summary_splits_extraction_embedding() {
        let rows = vec![
            DocumentCostRow {
                cost_usd: 1.0,
                input_tokens: 10,
                output_tokens: 5,
                processed_at: None,
            },
            DocumentCostRow {
                cost_usd: 3.0,
                input_tokens: 20,
                output_tokens: 10,
                processed_at: None,
            },
        ];
        let totals = aggregate_cost_summary(&rows);
        assert_eq!(totals.document_count, 2);
        assert!((totals.total_cost - 4.0).abs() < f64::EPSILON);
        assert!((totals.extraction_cost - 3.6).abs() < f64::EPSILON);
        assert!((totals.embedding_cost - 0.4).abs() < f64::EPSILON);
        assert_eq!(totals.total_input_tokens, 30);
        assert_eq!(totals.total_output_tokens, 15);
    }

    #[test]
    fn aggregate_history_groups_by_day() {
        let rows = vec![
            DocumentCostRow {
                cost_usd: 1.0,
                input_tokens: 10,
                output_tokens: 0,
                processed_at: Some("2026-06-01T10:00:00Z".to_string()),
            },
            DocumentCostRow {
                cost_usd: 2.0,
                input_tokens: 20,
                output_tokens: 0,
                processed_at: Some("2026-06-01T18:00:00Z".to_string()),
            },
        ];
        let history = aggregate_cost_history(&rows, "day");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].timestamp, "2026-06-01T00:00:00Z");
        assert!((history[0].total_cost - 3.0).abs() < f64::EPSILON);
        assert_eq!(history[0].document_count, 2);
    }
}
