//! One server predicate owns filtered rows, status counts and pagination.
use super::*;
use crate::handlers::documents_types::{ListDocumentsRequest, ListDocumentsResponse, StatusCounts};
use sqlx::{Postgres, QueryBuilder};

struct AccessFilter {
    rank: Option<i16>,
    tags: Vec<Uuid>,
    review: Option<String>,
}
impl AccessFilter {
    fn parse(params: &ListDocumentsRequest) -> ApiResult<Self> {
        let invalid = || {
            ApiError::unprocessable_coded("INVALID_ACCESS_FILTER", "Choose valid access filters")
        };
        let rank = params
            .classification
            .as_deref()
            .filter(|v| *v != "all")
            .map(|v| {
                edgequake_core::clearance::rank_from_name(v)
                    .map(i16::from)
                    .ok_or_else(invalid)
            })
            .transpose()?;
        let mut tags = params
            .security_tag_ids
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter(|v| !v.is_empty())
            .map(|v| Uuid::parse_str(v).map_err(|_| invalid()))
            .collect::<ApiResult<Vec<_>>>()?;
        tags.sort();
        tags.dedup();
        if tags.len() > 32 {
            return Err(invalid());
        }
        let review = params.review_state.clone();
        if review
            .as_deref()
            .is_some_and(|v| !matches!(v, "any" | "reviewed" | "needs_review"))
            || (review.as_deref() == Some("needs_review") && rank.is_some_and(|r| r != 5))
            || (review.as_deref() == Some("reviewed") && rank == Some(5))
        {
            return Err(invalid());
        }
        Ok(Self { rank, tags, review })
    }
    fn push<'a>(
        &self,
        builder: &mut QueryBuilder<'a, Postgres>,
        actor: &AccessActor,
        params: &ListDocumentsRequest,
        with_status: bool,
    ) {
        builder
            .push(" FROM public.documents WHERE workspace_id=")
            .push_bind(actor.workspace);
        if let Some(rank) = self.rank {
            builder.push(" AND classification_rank=").push_bind(rank);
        }
        if !self.tags.is_empty() {
            builder
                .push(" AND compartment_ids @> ")
                .push_bind(self.tags.clone());
        }
        match self.review.as_deref() {
            Some("needs_review") => {
                builder.push(" AND classification_rank=5");
            }
            Some("reviewed") => {
                builder.push(" AND classification_rank BETWEEN 1 AND 4");
            }
            _ => {}
        }
        if let Some(from) = &params.date_from {
            builder
                .push(" AND created_at >= ")
                .push_bind(from.clone())
                .push("::timestamptz");
        }
        if let Some(to) = &params.date_to {
            builder
                .push(" AND created_at <= ")
                .push_bind(to.clone())
                .push("::timestamptz");
        }
        if let Some(pattern) = &params.document_pattern {
            let patterns: Vec<_> = pattern
                .split(',')
                .filter(|v| !v.trim().is_empty())
                .take(16)
                .collect();
            if !patterns.is_empty() {
                builder.push(" AND (");
                for (index, part) in patterns.iter().enumerate() {
                    if index > 0 {
                        builder.push(" OR ");
                    }
                    builder.push("title ILIKE ").push_bind(format!(
                        "%{}%",
                        part.trim()
                            .replace('\\', "\\\\")
                            .replace('%', "\\%")
                            .replace('_', "\\_")
                    ));
                }
                builder.push(")");
            }
        }
        if with_status {
            if let Some(status) = params.status.as_deref().filter(|v| *v != "all") {
                match status {
                    "completed" => {
                        builder.push(" AND status IN ('completed','indexed','processed')");
                    }
                    "pending" => {
                        builder.push(" AND status IN ('pending','queued')");
                    }
                    _ => {
                        builder.push(" AND status=").push_bind(status.to_owned());
                    }
                }
            }
        }
    }
}

pub async fn list_access_filtered(
    pool: &PgPool,
    ctx: &TenantContext,
    params: &ListDocumentsRequest,
) -> ApiResult<ListDocumentsResponse> {
    let actor = AccessActor::from_context(ctx)?;
    let filter = AccessFilter::parse(params)?;
    let (mut tx, _) = scoped_transaction(pool, &actor, false).await?;
    let owned:i64=sqlx::query_scalar("SELECT count(*) FROM public.clearance_tags WHERE workspace_id=$1 AND kind='compartment' AND tag_id=ANY($2)").bind(actor.workspace).bind(&filter.tags).fetch_one(&mut *tx).await.map_err(database)?;
    if owned != filter.tags.len() as i64 {
        return Err(ApiError::unprocessable_coded(
            "INVALID_ACCESS_FILTER",
            "Choose valid access filters",
        ));
    }
    let mut count = QueryBuilder::new("SELECT count(*)");
    filter.push(&mut count, &actor, params, true);
    let total: i64 = count
        .build_query_scalar()
        .fetch_one(&mut *tx)
        .await
        .map_err(database)?;
    let mut chips = QueryBuilder::new("SELECT status,count(*)");
    filter.push(&mut chips, &actor, params, false);
    chips.push(" GROUP BY status");
    let rows: Vec<(Option<String>, i64)> = chips
        .build_query_as()
        .fetch_all(&mut *tx)
        .await
        .map_err(database)?;
    let mut status_counts = StatusCounts::default();
    for (status, n) in rows {
        let target = match status.as_deref() {
            Some("completed" | "indexed" | "processed") => &mut status_counts.completed,
            Some("pending" | "queued") => &mut status_counts.pending,
            Some("processing") => &mut status_counts.processing,
            Some("failed") => &mut status_counts.failed,
            Some("cancelled") => &mut status_counts.cancelled,
            Some("partial_failure") => &mut status_counts.partial_failure,
            _ => &mut status_counts.unknown,
        };
        *target += n as usize;
    }
    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, 100);
    let mut query=QueryBuilder::new("SELECT id::text AS id,title,status,chunk_count,entity_count,NULL::bigint AS content_length,created_at,updated_at,cost_usd,input_tokens,output_tokens,total_tokens,track_id,metadata->>'current_stage' AS current_stage,NULL::text AS content_preview,error_message");
    filter.push(&mut query, &actor, params, true);
    query
        .push(" ORDER BY created_at DESC,id LIMIT ")
        .push_bind(page_size as i64)
        .push(" OFFSET ")
        .push_bind(
            (page.saturating_sub(1).saturating_mul(page_size)).min(i64::MAX as usize) as i64,
        );
    let rows = query.build().fetch_all(&mut *tx).await.map_err(database)?;
    let mut documents = rows
        .into_iter()
        .map(crate::document_read_model::relational_summary_from_row)
        .collect::<Vec<_>>();
    let ids = documents
        .iter()
        .filter_map(|d| Uuid::parse_str(&d.id).ok())
        .collect::<Vec<_>>();
    // Existing label projection is batched, never one query per row.
    let labels = crate::services::clearance::labels_for_conn(&mut tx, &ids).await?;
    tx.commit().await.map_err(database)?;
    for doc in &mut documents {
        if let Some((classification, tags)) = labels.get(&doc.id) {
            doc.classification = classification.clone();
            doc.compartments = Some(tags.clone());
        }
    }
    let total = total as usize;
    let total_pages = total.div_ceil(page_size);
    Ok(ListDocumentsResponse {
        documents,
        total,
        page,
        page_size,
        total_pages,
        has_more: page < total_pages,
        status_counts,
        truncated: None,
    })
}
