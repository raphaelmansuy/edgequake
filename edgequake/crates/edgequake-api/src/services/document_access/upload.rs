use super::*;
use crate::services::document_access_types::DocumentAccess;
use serde_json::json;

pub async fn check_duplicate_access(
    pool: &PgPool,
    ctx: &TenantContext,
    id: &str,
    access: Option<&DocumentAccess>,
) -> ApiResult<()> {
    if let Some(access) = access {
        let actor = AccessActor::from_context(ctx)?;
        let saved = read_access(
            pool,
            &actor,
            Uuid::parse_str(id).map_err(|_| ApiError::BadRequest("Invalid document".into()))?,
        )
        .await?;
        let desired = access.normalize()?;
        if saved.access.classification != desired.classification
            || saved.access.compartment_ids != desired.compartment_ids
        {
            return Err(coded(
                StatusCode::CONFLICT,
                "DOCUMENT_ACCESS_CONFLICT",
                "This document already exists with different access. Edit its access separately",
            ));
        }
    }
    Ok(())
}

pub async fn validate_upload(
    pool: &PgPool,
    ctx: &TenantContext,
    access: Option<&DocumentAccess>,
) -> ApiResult<()> {
    let actor = AccessActor::from_context(ctx)?;
    let (mut tx, scope) = scoped_transaction(pool, &actor, false).await?;
    if let Some(access) = access {
        require_writer(pool, &actor).await?;
        validate_target(&mut tx, &actor, &scope, access, actor.user).await?;
    }
    if scope.subject.enforcement && actor.user.is_none() {
        return Err(coded(
            StatusCode::FORBIDDEN,
            "ACCESS_CHANGE_FORBIDDEN",
            "Enforced uploads require an attributed uploader",
        ));
    }
    tx.commit().await.map_err(database)?;
    Ok(())
}

/// Establish the label before any task, original reference or worker can expose it.
pub async fn admit_initial_access(
    pool: &PgPool,
    ctx: &TenantContext,
    id: Uuid,
    title: &str,
    track_id: &str,
    access: Option<&DocumentAccess>,
) -> ApiResult<()> {
    let actor = AccessActor::from_context(ctx)?;
    let (mut tx, scope) = scoped_transaction(pool, &actor, true).await?;
    let label = if let Some(access) = access {
        require_writer(pool, &actor).await?;
        Some(validate_target(&mut tx, &actor, &scope, access, actor.user).await?)
    } else if scope.subject.enforcement {
        Some(
            edgequake_core::clearance::ObjectLabel::new(5, []).uploaded_by(actor.user.ok_or_else(
                || {
                    coded(
                        StatusCode::FORBIDDEN,
                        "ACCESS_CHANGE_FORBIDDEN",
                        "Enforced uploads require an uploader",
                    )
                },
            )?),
        )
    } else {
        None
    };
    let rank = label.as_ref().and_then(|v| v.rank).map(i16::from);
    let tags = label
        .as_ref()
        .map(|v| v.compartments.iter().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    let inserted=sqlx::query("INSERT INTO public.documents(id,tenant_id,workspace_id,title,content,status,track_id,classification_rank,compartment_ids,uploaded_by,label_version) VALUES($1,$2,$3,$4,'','pending',$5,$6,$7,$8,$9) ON CONFLICT(id) DO NOTHING")
        .bind(id).bind(actor.tenant).bind(actor.workspace).bind(title).bind(track_id).bind(rank).bind(&tags).bind(actor.user).bind(i64::from(label.is_some()))
        .execute(&mut *tx).await.map_err(database)?.rows_affected();
    if inserted > 0 && label.is_some() {
        sqlx::query(
            "INSERT INTO public.document_labels(document_id,tag_id) SELECT $1,unnest($2::uuid[])",
        )
        .bind(id)
        .bind(&tags)
        .execute(&mut *tx)
        .await
        .map_err(database)?;
        audit(&mut tx,&actor,id,"clearance.admission",json!({"classification":label.as_ref().and_then(|v|v.rank).and_then(edgequake_core::clearance::rank_name),"compartment_ids":tags,"policy_epoch":scope.policy_epoch,"label_version":1,"source":"upload"})).await?;
    }
    tx.commit().await.map_err(database)?;
    Ok(())
}
