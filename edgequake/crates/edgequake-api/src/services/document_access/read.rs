use super::*;
use crate::services::document_access_types::{broadens, DocumentAccess};
use edgequake_core::clearance::{allows, allows_relabel, rank_name, ClearanceAction, ObjectLabel};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct SecurityTag {
    pub id: Uuid,
    pub key: String,
    pub value: String,
}
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccessCapabilities {
    pub can_edit: bool,
    pub can_downgrade: bool,
    pub can_manage_vocabulary: bool,
    pub can_check_member: bool,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct AssignmentOptions {
    pub enforcement: bool,
    pub policy_epoch: i64,
    pub break_glass: bool,
    pub can_upload: bool,
    pub capabilities: AccessCapabilities,
    pub classifications: Vec<String>,
    pub tags: Vec<SecurityTag>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentAccessView {
    pub document_id: Uuid,
    pub access: DocumentAccess,
    pub label_version: i64,
    pub policy_epoch: i64,
    pub enforcement: bool,
    pub capabilities: AccessCapabilities,
    pub tags: Vec<SecurityTag>,
    pub explanation: serde_json::Value,
}

pub async fn capabilities(
    conn: &mut sqlx::PgConnection,
    actor: &AccessActor,
    scope: &VisibilityScope,
) -> ApiResult<AccessCapabilities> {
    let role:Option<String>=sqlx::query_scalar("SELECT role FROM public.memberships WHERE user_id=$1 AND workspace_id=$2 AND is_active LIMIT 1").bind(actor.user).bind(actor.workspace).fetch_optional(&mut *conn).await.map_err(database)?;
    Ok(AccessCapabilities {
        can_edit: matches!(
            role.as_deref(),
            Some("owner" | "admin" | "member" | "editor")
        ),
        can_downgrade: matches!(role.as_deref(), Some("owner" | "admin" | "member"))
            && scope.subject.steward
            && scope.subject.enforcement,
        can_manage_vocabulary: matches!(role.as_deref(), Some("owner" | "admin")),
        can_check_member: scope.subject.steward,
    })
}

pub async fn assignment_options(
    pool: &PgPool,
    actor: &AccessActor,
    query: &str,
    cursor: Option<&str>,
) -> ApiResult<AssignmentOptions> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use sha2::{Digest, Sha256};
    let (mut tx, scope) = scoped_transaction(pool, actor, false).await?;
    let caps = capabilities(&mut tx, actor, &scope).await?;
    let binding = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}:{}:{}:{}",
            actor.workspace, actor.actor, query, scope.fingerprint
        ))
    );
    let offset = if let Some(cursor) = cursor {
        let decoded = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|_| ApiError::BadRequest("Invalid cursor".into()))?;
        let value: serde_json::Value = serde_json::from_slice(&decoded)
            .map_err(|_| ApiError::BadRequest("Invalid cursor".into()))?;
        if value["epoch"].as_i64() != Some(scope.policy_epoch) {
            return Err(coded(
                StatusCode::CONFLICT,
                "POLICY_CHANGED",
                "Access rules changed. Restart tag search",
            ));
        }
        if value["binding"].as_str() != Some(&binding) {
            return Err(ApiError::BadRequest("Invalid cursor".into()));
        }
        value["offset"]
            .as_i64()
            .filter(|n| *n >= 0)
            .ok_or_else(|| ApiError::BadRequest("Invalid cursor".into()))?
    } else {
        0
    };
    let tags: Vec<SecurityTag> = sqlx::query_as("SELECT tag_id AS id,key,value FROM public.clearance_tags WHERE workspace_id=$1 AND kind='compartment' AND value ILIKE $2 AND ($3 OR tag_id=ANY($4)) AND NOT(tag_id=ANY($5)) ORDER BY lower(value),tag_id OFFSET $6 LIMIT 51")
        .bind(actor.workspace).bind(format!("%{}%",query.chars().take(128).collect::<String>()))
        .bind(scope.subject.holds_all_compartments).bind(edgequake_core::clearance::guc_compartment_ids(&scope.subject,&[]).into_iter().collect::<Vec<_>>())
        .bind(scope.subject.denied_compartments.iter().copied().collect::<Vec<_>>())
        .bind(offset).fetch_all(&mut *tx).await.map_err(database)?;
    // Steward denies must still filter individually; the SQL all-compartment branch
    // is narrowed here by the same domain evaluator used for commit.
    let mut tags: Vec<_> = tags
        .into_iter()
        .filter(|tag| {
            let rank = scope.subject.rank.clamp(1, 4);
            allows(
                &scope.subject,
                &ObjectLabel::new(rank, [tag.id]),
                ClearanceAction::Write,
            )
            .is_allow()
        })
        .collect();
    let next_cursor = (tags.len() > 50).then(|| {
        URL_SAFE_NO_PAD.encode(
            serde_json::json!({"offset":offset+50,"epoch":scope.policy_epoch,"binding":binding})
                .to_string(),
        )
    });
    tags.truncate(50);
    let classifications = (1..=4)
        .filter(|rank| *rank <= scope.subject.rank)
        .filter_map(rank_name)
        .map(str::to_owned)
        .collect();
    tx.commit().await.map_err(database)?;
    Ok(AssignmentOptions {
        enforcement: scope.subject.enforcement,
        policy_epoch: scope.policy_epoch,
        break_glass: scope.subject.break_glass,
        can_upload: caps.can_edit,
        capabilities: caps,
        classifications,
        tags,
        next_cursor,
    })
}

pub async fn load_label(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AccessActor,
    id: Uuid,
    lock: bool,
) -> ApiResult<(ObjectLabel, i64)> {
    let sql = if lock {
        "SELECT classification_rank,COALESCE(compartment_ids,'{}'),uploaded_by,label_version FROM public.documents WHERE id=$1 AND workspace_id=$2 FOR UPDATE"
    } else {
        "SELECT classification_rank,COALESCE(compartment_ids,'{}'),uploaded_by,label_version FROM public.documents WHERE id=$1 AND workspace_id=$2"
    };
    type LabelRow = (Option<i16>, Vec<Uuid>, Option<Uuid>, i64);
    let row: Option<LabelRow> = sqlx::query_as(sql)
        .bind(id)
        .bind(actor.workspace)
        .fetch_optional(&mut **tx)
        .await
        .map_err(database)?;
    let (rank, tags, uploader, version) =
        row.ok_or_else(|| crate::services::clearance::not_found(&id.to_string()))?;
    Ok((
        ObjectLabel {
            rank: rank.and_then(|r| u8::try_from(r).ok()),
            compartments: tags.into_iter().collect(),
            uploaded_by: uploader,
        },
        version,
    ))
}

pub async fn read_access(
    pool: &PgPool,
    actor: &AccessActor,
    id: Uuid,
) -> ApiResult<DocumentAccessView> {
    let (mut tx, scope) = scoped_transaction(pool, actor, false).await?;
    let (label, version) = load_label(&mut tx, actor, id, false).await?;
    if !allows(&scope.subject, &label, ClearanceAction::Read).is_allow() {
        return Err(crate::services::clearance::not_found(&id.to_string()));
    }
    let tags=sqlx::query_as("SELECT tag_id AS id,key,value FROM public.clearance_tags WHERE workspace_id=$1 AND tag_id=ANY($2) ORDER BY lower(value),tag_id")
        .bind(actor.workspace).bind(label.compartments.iter().copied().collect::<Vec<_>>()).fetch_all(&mut *tx).await.map_err(database)?;
    let caps = capabilities(&mut tx, actor, &scope).await?;
    let explanation = serde_json::json!({"kind":if label.rank == Some(5) {"review"} else {"rank_and_all_tags"},"classification":label.rank.and_then(rank_name),"required_tags":tags,"steward_exception":true,"caller_can_read":true});
    tx.commit().await.map_err(database)?;
    Ok(DocumentAccessView {
        document_id: id,
        access: DocumentAccess {
            classification: label
                .rank
                .and_then(rank_name)
                .unwrap_or("unverified")
                .into(),
            compartment_ids: label.compartments.into_iter().collect(),
            expected_policy_epoch: scope.policy_epoch,
        },
        label_version: version,
        policy_epoch: scope.policy_epoch,
        enforcement: scope.subject.enforcement,
        capabilities: caps,
        tags,
        explanation,
    })
}

pub async fn validate_target(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AccessActor,
    scope: &VisibilityScope,
    access: &DocumentAccess,
    uploader: Option<Uuid>,
) -> ApiResult<ObjectLabel> {
    require_epoch(scope, access.expected_policy_epoch)?;
    let normalized = access.normalize()?;
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM public.clearance_tags WHERE workspace_id=$1 AND kind='compartment' AND tag_id=ANY($2)")
        .bind(actor.workspace).bind(&normalized.compartment_ids).fetch_one(&mut **tx).await.map_err(database)?;
    if count != normalized.compartment_ids.len() as i64 {
        return Err(ApiError::unprocessable_coded(
            "UNASSIGNABLE_TAG",
            "One or more security tags are unavailable",
        ));
    }
    let label = normalized.label(uploader)?;
    if label.rank == Some(5)
        && normalized.compartment_ids.iter().any(|tag| {
            !allows(
                &scope.subject,
                &ObjectLabel::new(scope.subject.rank.clamp(1, 4), [*tag]),
                ClearanceAction::Write,
            )
            .is_allow()
        })
    {
        return Err(coded(
            StatusCode::FORBIDDEN,
            "ACCESS_CHANGE_FORBIDDEN",
            "That label is outside your access profile",
        ));
    }
    if !allows(&scope.subject, &label, ClearanceAction::Write).is_allow() {
        return Err(coded(
            StatusCode::FORBIDDEN,
            "ACCESS_CHANGE_FORBIDDEN",
            "That label is outside your access profile",
        ));
    }
    Ok(label)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AccessPreviewRequest {
    pub mode: String,
    pub items: Vec<PreviewItem>,
    pub expected_policy_epoch: i64,
    pub check_member_id: Option<Uuid>,
}
#[derive(Debug, Deserialize, ToSchema)]
pub struct PreviewItem {
    pub document_id: Option<Uuid>,
    pub access: DocumentAccess,
    pub label_version: Option<i64>,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct PreviewResult {
    pub allowed: bool,
    pub requires_reason: bool,
    pub change_kind: String,
    pub reason: Option<String>,
    pub caller_can_read: bool,
    pub normalized_access: Option<DocumentAccess>,
    pub member_can_read: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccessPreviewResponse {
    pub allowed: bool,
    pub policy_epoch: i64,
    pub items: Vec<PreviewResult>,
}

pub async fn preview(
    pool: &PgPool,
    actor: &AccessActor,
    request: AccessPreviewRequest,
) -> ApiResult<serde_json::Value> {
    if request.check_member_id.is_none() {
        require_writer(pool, actor).await?;
    }
    if request.items.is_empty()
        || request.items.len() > 100
        || !matches!(request.mode.as_str(), "create" | "edit" | "bulk")
    {
        return Err(ApiError::bad_request_coded(
            "INVALID_ACCESS_PAYLOAD",
            "Invalid preview selection",
        ));
    }
    let (mut tx, scope) = scoped_transaction(pool, actor, false).await?;
    require_epoch(&scope, request.expected_policy_epoch)?;
    let member = if let Some(user) = request.check_member_id {
        if !scope.subject.steward {
            return Err(coded(
                StatusCode::FORBIDDEN,
                "ACCESS_CHANGE_FORBIDDEN",
                "Only security stewards can check another member",
            ));
        }
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public.memberships WHERE workspace_id=$1 AND user_id=$2 AND is_active)").bind(actor.workspace).bind(user).fetch_one(&mut *tx).await.map_err(database)?;
        if !active {
            return Err(ApiError::NotFound("Member not found".into()));
        }
        Some(
            crate::services::clearance::load_scope_conn(&mut tx, actor.workspace, Some(user))
                .await?
                .0,
        )
    } else {
        None
    };
    let mut results = Vec::new();
    for item in request.items {
        let before = if let Some(id) = item.document_id {
            Some(load_label(&mut tx, actor, id, false).await?)
        } else {
            None
        };
        if request.mode != "create" && before.is_none() {
            return Err(ApiError::bad_request_coded(
                "INVALID_ACCESS_PAYLOAD",
                "Document is required",
            ));
        }
        if let Some((_, version)) = &before {
            if item.label_version != Some(*version) {
                return Err(coded(
                    StatusCode::PRECONDITION_FAILED,
                    "LABEL_VERSION_CONFLICT",
                    "Document access changed",
                ));
            }
        }
        let target = validate_target(
            &mut tx,
            actor,
            &scope,
            &item.access,
            before.as_ref().and_then(|b| b.0.uploaded_by).or(actor.user),
        )
        .await;
        let result = match target {
            Ok(after) => {
                let relaxing = before.as_ref().is_some_and(|b| broadens(&b.0, &after));
                let allowed = before
                    .as_ref()
                    .is_none_or(|b| allows_relabel(&scope.subject, &b.0, &after).is_allow());
                PreviewResult {
                    allowed,
                    requires_reason: relaxing,
                    change_kind: if relaxing {
                        "broaden"
                    } else if before.as_ref().is_some_and(|b| b.0 == after) {
                        "unchanged"
                    } else if before.is_some() {
                        "strengthen"
                    } else {
                        "initial"
                    }
                    .into(),
                    reason: (!allowed).then(|| {
                        "A security steward must approve changes that broaden access".into()
                    }),
                    caller_can_read: allows(&scope.subject, &after, ClearanceAction::Read)
                        .is_allow(),
                    normalized_access: Some(item.access.normalize()?),
                    member_can_read: member.as_ref().map(|member| {
                        allows(&member.subject, &after, ClearanceAction::Read).is_allow()
                    }),
                }
            }
            Err(error) => PreviewResult {
                allowed: false,
                requires_reason: false,
                change_kind: "blocked".into(),
                reason: Some(error.to_string()),
                caller_can_read: false,
                normalized_access: None,
                member_can_read: None,
            },
        };
        results.push(result);
    }
    tx.commit().await.map_err(database)?;
    Ok(
        serde_json::json!({"allowed":results.iter().all(|r|r.allowed),"items":results,"policy_epoch":scope.policy_epoch}),
    )
}
