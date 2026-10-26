//! User-facing task pages and counts share a single clearance-scoped snapshot.
use super::*;
use edgequake_tasks::{Pagination, SortField, SortOrder, TaskFilter, TaskList, TaskStatistics};
use sqlx::{Postgres, QueryBuilder, Row};

fn predicate<'a>(query: &mut QueryBuilder<'a, Postgres>, actor: &AccessActor, filter: &TaskFilter) {
    query
        .push(" FROM public.tasks WHERE tenant_id=")
        .push_bind(actor.tenant)
        .push(" AND workspace_id=")
        .push_bind(actor.workspace);
    if let Some(status) = &filter.status {
        query.push(" AND status=").push_bind(status.to_string());
    }
    if let Some(kind) = &filter.task_type {
        query.push(" AND task_type=").push_bind(kind.to_string());
    }
}

pub async fn list_scoped_tasks(
    pool: &PgPool,
    actor: &AccessActor,
    filter: &TaskFilter,
    pagination: &Pagination,
) -> ApiResult<(TaskList, TaskStatistics)> {
    if filter.tenant_id != Some(actor.tenant) || filter.workspace_id != Some(actor.workspace) {
        return Err(ApiError::forbidden_reason(
            "Task filters must match the authorized workspace",
        ));
    }
    let (mut tx, _) = scoped_transaction(pool, actor, false).await?;
    let mut counts=QueryBuilder::new("SELECT count(*) AS total,count(*) FILTER(WHERE status='pending') AS pending,count(*) FILTER(WHERE status='processing') AS processing,count(*) FILTER(WHERE status='indexed') AS indexed,count(*) FILTER(WHERE status='failed') AS failed,count(*) FILTER(WHERE status='cancelled') AS cancelled");
    predicate(&mut counts, actor, filter);
    let row = counts.build().fetch_one(&mut *tx).await.map_err(database)?;
    let count = |name: &str| row.get::<i64, _>(name).max(0) as u64;
    let stats = TaskStatistics {
        pending: count("pending"),
        processing: count("processing"),
        indexed: count("indexed"),
        failed: count("failed"),
        cancelled: count("cancelled"),
        total: count("total"),
    };
    let page = pagination.page.max(1);
    let page_size = pagination.page_size.clamp(1, 100);
    let mut query = QueryBuilder::new("SELECT *");
    predicate(&mut query, actor, filter);
    let keyset = pagination.sort_by == SortField::CreatedAt && pagination.has_keyset_cursor();
    if keyset {
        if let (Some(at), Some(id)) = (
            pagination.after_created_at,
            pagination.after_track_id.as_ref(),
        ) {
            query
                .push(" AND (created_at,track_id) ")
                .push(if pagination.order == SortOrder::Asc {
                    ">"
                } else {
                    "<"
                })
                .push(" (")
                .push_bind(at)
                .push(",")
                .push_bind(id.clone())
                .push(")");
        }
    }
    let field = if pagination.sort_by == SortField::CreatedAt {
        "created_at"
    } else {
        "updated_at"
    };
    let order = if pagination.order == SortOrder::Asc {
        " ASC"
    } else {
        " DESC"
    };
    query
        .push(" ORDER BY ")
        .push(field)
        .push(order)
        .push(",track_id")
        .push(order)
        .push(" LIMIT ")
        .push_bind(i64::from(page_size));
    if !keyset {
        query
            .push(" OFFSET ")
            .push_bind(i64::from(page - 1) * i64::from(page_size));
    }
    let rows = query.build().fetch_all(&mut *tx).await.map_err(database)?;
    let tasks = rows
        .iter()
        .map(edgequake_tasks::postgres::task_from_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ApiError::Internal("Task metadata could not be read".into()))?;
    tx.commit().await.map_err(database)?;
    let list = TaskList {
        tasks,
        total: stats.total,
        page,
        page_size,
        total_pages: stats.total.div_ceil(u64::from(page_size)) as u32,
    };
    Ok((list, stats))
}
