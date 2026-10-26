use super::*;
use crate::services::document_access_types::{broadens, DocumentAccess};
use edgequake_core::clearance::{allows_relabel, rank_name};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AccessMutationItem {
    pub document_id: Uuid,
    pub label_version: i64,
    pub access: DocumentAccess,
}
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AccessMutation {
    pub items: Vec<AccessMutationItem>,
    pub reason: Option<String>,
    pub operation_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct BulkAccessIntent {
    pub operation: String,
    pub items: Vec<BulkAccessTarget>,
    pub compartment_ids: Option<Vec<Uuid>>,
    pub classification: Option<String>,
    pub expected_policy_epoch: i64,
    pub reason: Option<String>,
    pub operation_id: Uuid,
}
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct BulkAccessTarget {
    pub document_id: Uuid,
    pub label_version: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum AccessMutationRequest {
    Intent(BulkAccessIntent),
    Expanded(AccessMutation),
}

pub async fn expand_bulk(
    pool: &PgPool,
    actor: &AccessActor,
    intent: BulkAccessIntent,
) -> ApiResult<AccessMutation> {
    if intent.items.is_empty()
        || intent.items.len() > 100
        || !matches!(
            intent.operation.as_str(),
            "add_tags" | "remove_tags" | "replace_access"
        )
    {
        return Err(ApiError::bad_request_coded(
            "INVALID_ACCESS_PAYLOAD",
            "Choose a valid bulk operation and 1 to 100 documents",
        ));
    }
    let (mut tx, scope) = scoped_transaction(pool, actor, false).await?;
    let _ = scope;
    let tags = intent.compartment_ids.ok_or_else(|| {
        ApiError::bad_request_coded("INVALID_ACCESS_PAYLOAD", "Security tags are required")
    })?;
    let mut items = Vec::new();
    for item in intent.items {
        let (before, _) = load_label(&mut tx, actor, item.document_id, false).await?;
        let (classification, compartment_ids) = match intent.operation.as_str() {
            "add_tags" => (
                before
                    .rank
                    .and_then(rank_name)
                    .unwrap_or("unverified")
                    .to_owned(),
                before
                    .compartments
                    .iter()
                    .chain(tags.iter())
                    .copied()
                    .collect(),
            ),
            "remove_tags" => (
                before
                    .rank
                    .and_then(rank_name)
                    .unwrap_or("unverified")
                    .to_owned(),
                before
                    .compartments
                    .iter()
                    .filter(|id| !tags.contains(id))
                    .copied()
                    .collect(),
            ),
            _ => (
                intent.classification.clone().ok_or_else(|| {
                    ApiError::bad_request_coded(
                        "INVALID_ACCESS_PAYLOAD",
                        "Replacement sensitivity is required",
                    )
                })?,
                tags.clone(),
            ),
        };
        items.push(AccessMutationItem {
            document_id: item.document_id,
            label_version: item.label_version,
            access: DocumentAccess {
                classification,
                compartment_ids,
                expected_policy_epoch: intent.expected_policy_epoch,
            }
            .normalize()?,
        });
    }
    tx.commit().await.map_err(database)?;
    Ok(AccessMutation {
        items,
        reason: intent.reason,
        operation_id: intent.operation_id,
    })
}

pub fn operation_lock(actor: &AccessActor, operation: Uuid) -> i64 {
    let hash = Sha256::digest(format!("{}:{}:{operation}", actor.workspace, actor.actor));
    i64::from_be_bytes(hash[..8].try_into().expect("SHA256 has eight bytes"))
}

pub async fn mutate_bulk(
    pool: &PgPool,
    actor: &AccessActor,
    mut intent: BulkAccessIntent,
) -> ApiResult<serde_json::Value> {
    intent.items.sort_by_key(|item| item.document_id);
    if let Some(tags) = &mut intent.compartment_ids {
        tags.sort();
        tags.dedup();
    }
    intent.reason = intent
        .reason
        .map(|reason| reason.trim().to_owned())
        .filter(|reason| !reason.is_empty());
    let hash = format!(
        "bulk:{:x}",
        Sha256::digest(serde_json::to_vec(&intent).map_err(|e| ApiError::Internal(e.to_string()))?)
    );
    let request = expand_bulk(pool, actor, intent).await?;
    mutate_with_hash(pool, actor, request, Some(hash)).await
}

pub async fn mutate(
    pool: &PgPool,
    actor: &AccessActor,
    request: AccessMutation,
) -> ApiResult<serde_json::Value> {
    mutate_with_hash(pool, actor, request, None).await
}

async fn mutate_with_hash(
    pool: &PgPool,
    actor: &AccessActor,
    mut request: AccessMutation,
    intent_hash: Option<String>,
) -> ApiResult<serde_json::Value> {
    require_writer(pool, actor).await?;
    if request.items.is_empty() || request.items.len() > 100 {
        return Err(ApiError::bad_request_coded(
            "INVALID_ACCESS_PAYLOAD",
            "Select 1 to 100 documents",
        ));
    }
    request.items.sort_by_key(|i| i.document_id);
    for item in &mut request.items {
        item.access = item.access.normalize()?;
    }
    request.reason = request
        .reason
        .map(|reason| reason.trim().to_owned())
        .filter(|reason| !reason.is_empty());
    if request
        .items
        .windows(2)
        .any(|w| w[0].document_id == w[1].document_id)
    {
        return Err(ApiError::bad_request_coded(
            "INVALID_ACCESS_PAYLOAD",
            "Repeated document selection",
        ));
    }
    let hash = intent_hash.unwrap_or(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&request).map_err(|e| ApiError::Internal(e.to_string()))?
        )
    ));
    let (mut tx, scope) = scoped_transaction(pool, actor, true).await?;
    let reserved: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(operation_lock(actor, request.operation_id))
        .fetch_one(&mut *tx)
        .await
        .map_err(database)?;
    if !reserved {
        return Err(coded(
            StatusCode::CONFLICT,
            "OPERATION_IN_PROGRESS",
            "This access change is still in progress. Check its status before retrying",
        ));
    }
    // Look up all targets under current visibility before replaying any saved result.
    let mut before = Vec::new();
    for item in &request.items {
        before.push(load_label(&mut tx, actor, item.document_id, true).await?);
    }
    let receipt:Option<(String,serde_json::Value)>=sqlx::query_as("SELECT request_hash,outcome FROM public.document_access_operations WHERE operation_id=$1 AND workspace_id=$2 AND actor_id=$3")
        .bind(request.operation_id).bind(actor.workspace).bind(&actor.actor).fetch_optional(&mut *tx).await.map_err(database)?;
    if let Some((previous, outcome)) = receipt {
        if previous != hash {
            return Err(coded(
                StatusCode::CONFLICT,
                "IDEMPOTENCY_KEY_REUSED",
                "This operation key belongs to another change",
            ));
        }
        tx.commit().await.map_err(database)?;
        return Ok(outcome);
    }
    let mut changes = Vec::new();
    for (item, (old, version)) in request.items.iter().zip(before.iter()) {
        require_epoch(&scope, item.access.expected_policy_epoch)?;
        if item.label_version != *version {
            return Err(coded(
                StatusCode::PRECONDITION_FAILED,
                "LABEL_VERSION_CONFLICT",
                "Access changed while you were editing. Review the latest values",
            ));
        }
        let after = validate_target(&mut tx, actor, &scope, &item.access, old.uploaded_by).await?;
        if !allows_relabel(&scope.subject, old, &after).is_allow() {
            return Err(coded(
                StatusCode::FORBIDDEN,
                "STEWARD_REQUIRED",
                "A security steward must approve changes that broaden access",
            ));
        }
        if broadens(old, &after)
            && !(10..=500).contains(
                &request
                    .reason
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .count(),
            )
        {
            return Err(ApiError::unprocessable_coded(
                "REASON_REQUIRED",
                "Explain this access change in 10 to 500 characters",
            ));
        }
        changes.push(after);
    }
    let mut changed = false;
    ensure_descendant_budget(&mut tx, &request.items).await?;
    let mut outcomes = Vec::new();
    for ((item, (old, version)), after) in
        request.items.iter().zip(before.iter()).zip(changes.iter())
    {
        let different = old != after;
        changed |= different;
        if different {
            sqlx::query("DELETE FROM public.document_labels WHERE document_id=$1")
                .bind(item.document_id)
                .execute(&mut *tx)
                .await
                .map_err(database)?;
            let tags = after.compartments.iter().copied().collect::<Vec<_>>();
            sqlx::query("INSERT INTO public.document_labels(document_id,tag_id) SELECT $1,unnest($2::uuid[])").bind(item.document_id).bind(&tags).execute(&mut *tx).await.map_err(database)?;
            sqlx::query("UPDATE public.documents SET classification_rank=$2,compartment_ids=$3,label_version=label_version+1 WHERE id=$1 AND workspace_id=$4")
                .bind(item.document_id).bind(after.rank.map(i16::from)).bind(&tags).bind(actor.workspace).execute(&mut *tx).await.map_err(database)?;
            audit(&mut tx,actor,item.document_id,if broadens(old,after){"clearance.downgrade"}else{"clearance.relabel"},json!({"before":{"classification":old.rank.and_then(rank_name),"compartment_ids":old.compartments},"after":{"classification":after.rank.and_then(rank_name),"compartment_ids":tags},"reason":request.reason,"operation_id":request.operation_id,"label_version":version+1,"source":"document_access"})).await?;
        }
        outcomes.push(
            json!({"document_id":item.document_id,"label_version":version+i64::from(different)}),
        );
    }
    let epoch = if changed {
        sqlx::query_scalar::<_,i64>("UPDATE public.workspace_clearance_settings SET policy_epoch=policy_epoch+1 WHERE workspace_id=$1 RETURNING policy_epoch").bind(actor.workspace).fetch_one(&mut *tx).await.map_err(database)?
    } else {
        scope.policy_epoch
    };
    let outcome =
        json!({"items":outcomes,"policy_epoch":epoch,"operation_id":request.operation_id});
    sqlx::query("INSERT INTO public.document_access_operations(operation_id,workspace_id,actor_id,request_hash,outcome) VALUES($1,$2,$3,$4,$5)")
        .bind(request.operation_id).bind(actor.workspace).bind(&actor.actor).bind(hash).bind(&outcome).execute(&mut *tx).await.map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(outcome)
}

/// Bound the whole selection, not each document independently. Every identifier
/// and table name is fixed by this service; values remain parameterized.
async fn ensure_descendant_budget(
    tx: &mut Transaction<'_, Postgres>,
    items: &[AccessMutationItem],
) -> ApiResult<()> {
    let ids = items
        .iter()
        .map(|item| item.document_id)
        .collect::<Vec<_>>();
    let mut total = 0i64;
    for (table, column) in [
        ("chunks", "document_id"),
        ("document_originals", "document_id"),
        ("pdf_documents", "document_id"),
        ("document_mm_assets", "document_id"),
        ("document_pages", "document_id"),
        ("graph_contributions", "source_document_id"),
    ] {
        let statement = format!("SELECT count(*) FROM (SELECT 1 FROM public.{table} WHERE {column}=ANY($1) LIMIT 500001) bounded");
        total += sqlx::query_scalar::<_, i64>(&statement)
            .bind(&ids)
            .fetch_one(&mut **tx)
            .await
            .map_err(database)?;
        if total > 500_000 {
            return Err(ApiError::unprocessable_coded(
                "ACCESS_CHANGE_TOO_LARGE",
                "Select fewer documents for this access change",
            ));
        }
    }
    total += sqlx::query_scalar::<_,i64>("SELECT count(*) FROM (SELECT 1 FROM public.tasks WHERE document_id=ANY($1) LIMIT 500001) bounded").bind(ids.iter().map(ToString::to_string).collect::<Vec<_>>()).fetch_one(&mut **tx).await.map_err(database)?;
    if total > 500_000 {
        return Err(ApiError::unprocessable_coded(
            "ACCESS_CHANGE_TOO_LARGE",
            "Select fewer documents for this access change",
        ));
    }
    Ok(())
}
