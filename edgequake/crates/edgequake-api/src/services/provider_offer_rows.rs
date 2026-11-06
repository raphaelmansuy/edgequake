//! SPEC-177 persistence for provider offers. Rules live in `edgequake_core::provider_offer`.

use std::collections::BTreeSet;

use edgequake_core::provider_offer::{
    child_offer, global_offer_including_tested, parse_slug, ProviderCandidate,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::connections::env_connections;
use crate::model_catalog::active_provider_names;
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeOffer {
    pub constrained: bool,
    pub slugs: BTreeSet<String>,
}

#[cfg(feature = "postgres")]
fn pool(state: &AppState) -> ApiResult<&sqlx::PgPool> {
    state
        .optional_pg_pool()
        .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))
}

#[cfg(feature = "postgres")]
fn internal(error: sqlx::Error) -> ApiError {
    ApiError::Internal(error.to_string())
}

/// `provider_offer_revision` is `integer` (INT4). Callers compare it with the
/// `i64` sent in `If-Match`; decoding the column itself as `i64` fails the save.
#[cfg(feature = "postgres")]
pub async fn current_offer_revision<'e, E>(
    executor: E,
    workspace: bool,
    id: Uuid,
) -> Result<i64, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let sql = if workspace {
        "SELECT provider_offer_revision FROM workspaces WHERE workspace_id = $1"
    } else {
        "SELECT provider_offer_revision FROM tenants WHERE tenant_id = $1"
    };
    let current: i32 = sqlx::query_scalar(sql).bind(id).fetch_one(executor).await?;
    Ok(i64::from(current))
}

#[cfg(feature = "postgres")]
pub async fn load_workspace_offer_from_pool(
    pool: &sqlx::PgPool,
    tenant: Uuid,
    workspace: Uuid,
) -> ApiResult<ScopeOffer> {
    let parent = global_offer_from_pool(pool).await?;
    let tenant_constrained: bool =
        sqlx::query_scalar("SELECT provider_offer_constrained FROM tenants WHERE tenant_id = $1")
            .bind(tenant)
            .fetch_optional(pool)
            .await
            .map_err(internal)?
            .unwrap_or(false);
    let tenant_enabled = enabled_slugs(pool, tenant, None).await?;
    let tenant_slugs = child_offer(&parent, tenant_constrained, &tenant_enabled);
    let constrained: bool = sqlx::query_scalar(
        "SELECT provider_offer_constrained FROM workspaces WHERE workspace_id = $1",
    )
    .bind(workspace)
    .fetch_optional(pool)
    .await
    .map_err(internal)?
    .unwrap_or(false);
    let enabled = enabled_slugs(pool, tenant, Some(workspace)).await?;
    Ok(ScopeOffer {
        constrained,
        slugs: child_offer(&tenant_slugs, constrained, &enabled),
    })
}

#[cfg(feature = "postgres")]
async fn global_offer_from_pool(pool: &sqlx::PgPool) -> ApiResult<BTreeSet<String>> {
    let fleet: Vec<(String, Option<bool>)> = sqlx::query_as(
        "SELECT slug, last_test_ok FROM provider_connections WHERE tenant_id IS NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(internal)?;
    let withdrawn_rows: Vec<(String,)> =
        sqlx::query_as("SELECT provider_slug FROM deployment_provider_overrides WHERE withdrawn")
            .fetch_all(pool)
            .await
            .map_err(internal)?;
    let mut candidates: Vec<ProviderCandidate> = env_connections()
        .into_iter()
        .filter_map(|row| {
            parse_slug(&row.slug).map(|slug| ProviderCandidate {
                slug,
                from_env: true,
                last_test_ok: None,
            })
        })
        .collect();
    for (slug, last_test_ok) in fleet {
        if let Some(slug) = parse_slug(&slug) {
            candidates.push(ProviderCandidate {
                slug,
                from_env: false,
                last_test_ok,
            });
        }
    }
    // Integration tests opt in with EDGEQUAKE_ALLOW_MOCK_PROVIDER. Production
    // leaves that unset, so mock stays out of the offer and the picker.
    if crate::provider_visibility::mock_provider_allowed()
        && !candidates
            .iter()
            .any(|candidate| candidate.slug.eq_ignore_ascii_case("mock"))
    {
        candidates.push(ProviderCandidate {
            slug: "mock".into(),
            from_env: true,
            last_test_ok: None,
        });
    }
    let withdrawn: BTreeSet<String> = withdrawn_rows.into_iter().map(|(slug,)| slug).collect();
    let allow = active_provider_names().map(|names| names.into_iter().collect());
    Ok(global_offer_including_tested(
        &candidates,
        &withdrawn,
        allow.as_ref(),
    ))
}

#[cfg(feature = "postgres")]
pub async fn load_global_offer(state: &AppState) -> ApiResult<BTreeSet<String>> {
    global_offer_from_pool(pool(state)?).await
}

#[cfg(not(feature = "postgres"))]
pub async fn load_global_offer(_state: &AppState) -> ApiResult<BTreeSet<String>> {
    Err(ApiError::Internal("PostgreSQL is required".into()))
}

#[cfg(feature = "postgres")]
pub async fn load_tenant_offer(state: &AppState, tenant: Uuid) -> ApiResult<ScopeOffer> {
    let parent = load_global_offer(state).await?;
    let pool = pool(state)?;
    let constrained: bool =
        sqlx::query_scalar("SELECT provider_offer_constrained FROM tenants WHERE tenant_id = $1")
            .bind(tenant)
            .fetch_optional(pool)
            .await
            .map_err(internal)?
            .unwrap_or(false);
    let enabled = enabled_slugs(pool, tenant, None).await?;
    Ok(ScopeOffer {
        constrained,
        slugs: child_offer(&parent, constrained, &enabled),
    })
}

#[cfg(not(feature = "postgres"))]
pub async fn load_tenant_offer(_state: &AppState, _tenant: Uuid) -> ApiResult<ScopeOffer> {
    Err(ApiError::Internal("PostgreSQL is required".into()))
}

#[cfg(feature = "postgres")]
pub async fn load_workspace_offer(
    state: &AppState,
    tenant: Uuid,
    workspace: Uuid,
) -> ApiResult<ScopeOffer> {
    let parent = load_tenant_offer(state, tenant).await?.slugs;
    let pool = pool(state)?;
    let constrained: bool = sqlx::query_scalar(
        "SELECT provider_offer_constrained FROM workspaces WHERE workspace_id = $1",
    )
    .bind(workspace)
    .fetch_optional(pool)
    .await
    .map_err(internal)?
    .unwrap_or(false);
    let enabled = enabled_slugs(pool, tenant, Some(workspace)).await?;
    Ok(ScopeOffer {
        constrained,
        slugs: child_offer(&parent, constrained, &enabled),
    })
}

#[cfg(not(feature = "postgres"))]
pub async fn load_workspace_offer(
    _state: &AppState,
    _tenant: Uuid,
    _workspace: Uuid,
) -> ApiResult<ScopeOffer> {
    Err(ApiError::Internal("PostgreSQL is required".into()))
}

#[cfg(feature = "postgres")]
async fn enabled_slugs(
    pool: &sqlx::PgPool,
    tenant: Uuid,
    workspace: Option<Uuid>,
) -> ApiResult<BTreeSet<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT provider_slug FROM provider_offers \
         WHERE tenant_id = $1 AND workspace_id IS NOT DISTINCT FROM $2 AND enabled",
    )
    .bind(tenant)
    .bind(workspace)
    .fetch_all(pool)
    .await
    .map_err(internal)?;
    Ok(rows.into_iter().map(|(slug,)| slug).collect())
}

/// Replace one scope. Does not touch caches or reprocess documents.
#[cfg(feature = "postgres")]
pub async fn replace_offer(
    state: &AppState,
    tenant: Uuid,
    workspace: Option<Uuid>,
    constrained: bool,
    slugs: &[String],
    expected_revision: Option<i64>,
) -> ApiResult<()> {
    let pool = pool(state)?;
    let mut tx = pool.begin().await.map_err(internal)?;
    let id_for_revision = workspace.unwrap_or(tenant);
    if let Some(expected) = expected_revision {
        let current = current_offer_revision(&mut *tx, workspace.is_some(), id_for_revision)
            .await
            .map_err(internal)?;
        if current != expected {
            return Err(ApiError::Coded {
                code: "stale_revision",
                message: "the offer changed; reload and try again".into(),
                status: axum::http::StatusCode::CONFLICT,
            });
        }
    }
    let flag_sql = if workspace.is_some() {
        "UPDATE workspaces SET provider_offer_constrained = $2 WHERE workspace_id = $1"
    } else {
        "UPDATE tenants SET provider_offer_constrained = $2 WHERE tenant_id = $1"
    };
    let id = workspace.unwrap_or(tenant);
    sqlx::query(flag_sql)
        .bind(id)
        .bind(constrained)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    let bump_sql = if workspace.is_some() {
        "UPDATE workspaces SET provider_offer_revision = provider_offer_revision + 1 WHERE workspace_id = $1"
    } else {
        "UPDATE tenants SET provider_offer_revision = provider_offer_revision + 1 WHERE tenant_id = $1"
    };
    sqlx::query(bump_sql)
        .bind(id_for_revision)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    sqlx::query(
        "DELETE FROM provider_offers \
         WHERE tenant_id = $1 AND workspace_id IS NOT DISTINCT FROM $2",
    )
    .bind(tenant)
    .bind(workspace)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    if constrained {
        for slug in slugs {
            sqlx::query(
                "INSERT INTO provider_offers (tenant_id, workspace_id, provider_slug, enabled) \
                 VALUES ($1, $2, $3, true)",
            )
            .bind(tenant)
            .bind(workspace)
            .bind(slug)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
        }
    }
    tx.commit().await.map_err(internal)
}

#[cfg(not(feature = "postgres"))]
pub async fn replace_offer(
    _state: &AppState,
    _tenant: Uuid,
    _workspace: Option<Uuid>,
    _constrained: bool,
    _slugs: &[String],
    _expected_revision: Option<i64>,
) -> ApiResult<()> {
    Err(ApiError::Internal("PostgreSQL is required".into()))
}

/// Drop every grant for a slug. Does not delete `llm_cache` or enqueue reprocess.
#[cfg(feature = "postgres")]
pub async fn remove_offers_for_slug(pool: &sqlx::PgPool, slug: &str) -> ApiResult<()> {
    sqlx::query("DELETE FROM provider_offers WHERE provider_slug = $1")
        .bind(slug)
        .execute(pool)
        .await
        .map_err(internal)?;
    Ok(())
}

/// Replace the models confirmed for one fleet connection. Env slugs have no row.
#[cfg(feature = "postgres")]
pub async fn replace_connection_models(
    pool: &sqlx::PgPool,
    slug: &str,
    models: &[(String, bool, bool, bool, Option<i32>)],
) -> ApiResult<()> {
    let found: Option<(uuid::Uuid, Option<bool>)> = sqlx::query_as(
        "SELECT id, last_test_ok FROM provider_connections WHERE tenant_id IS NULL AND slug = $1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(internal)?;
    let Some((id, probe)) = found else {
        return Err(ApiError::NotFound(format!("provider {slug}")));
    };
    if probe != Some(true) {
        return Err(ApiError::unprocessable_coded(
            "probe_required",
            "test the provider before saving its models",
        ));
    }
    let mut tx = pool.begin().await.map_err(internal)?;
    sqlx::query("DELETE FROM connection_models WHERE connection_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    for (model, chat, vision, embedding, dimension) in models {
        sqlx::query(
            "INSERT INTO connection_models \
             (connection_id, model, supports_chat, supports_vision, supports_embedding, embedding_dimension) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(model)
        .bind(chat)
        .bind(vision)
        .bind(embedding)
        .bind(dimension)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    }
    tx.commit().await.map_err(internal)
}

#[cfg(feature = "postgres")]
pub async fn set_withdrawn(state: &AppState, slug: &str, withdrawn: bool) -> ApiResult<()> {
    let pool = pool(state)?;
    sqlx::query(
        "INSERT INTO deployment_provider_overrides (provider_slug, withdrawn) \
         VALUES ($1, $2) \
         ON CONFLICT (provider_slug) DO UPDATE \
         SET withdrawn = EXCLUDED.withdrawn, updated_at = now()",
    )
    .bind(slug)
    .bind(withdrawn)
    .execute(pool)
    .await
    .map_err(internal)?;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
pub async fn set_withdrawn(_state: &AppState, _slug: &str, _withdrawn: bool) -> ApiResult<()> {
    Err(ApiError::Internal("PostgreSQL is required".into()))
}

/// How many saved offers or workspace columns still name this slug.
pub async fn provider_assignment_count(state: &AppState, slug: &str) -> ApiResult<i64> {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = state.optional_pg_pool() else {
            return Ok(0);
        };
        let offers: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM provider_offers WHERE provider_slug = $1 AND enabled",
        )
        .bind(slug)
        .fetch_one(pool)
        .await
        .map_err(internal)?;
        let columns: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM workspaces \
             WHERE lower(metadata->>'llm_provider') = $1 \
                OR lower(metadata->>'embedding_provider') = $1 \
                OR lower(coalesce(metadata->>'vision_llm_provider', '')) = $1",
        )
        .bind(slug)
        .fetch_one(pool)
        .await
        .map_err(internal)?;
        Ok(offers + columns)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, slug);
        Ok(0)
    }
}

/// Compare `If-Match` and advance the scope revision. Absent header proceeds.
pub async fn touch_scope_revision(
    state: &AppState,
    tenant: Uuid,
    workspace: Option<Uuid>,
    expected: Option<i64>,
) -> ApiResult<()> {
    #[cfg(feature = "postgres")]
    {
        let pool = pool(state)?;
        let id = workspace.unwrap_or(tenant);
        if let Some(expected) = expected {
            let current = current_offer_revision(pool, workspace.is_some(), id)
                .await
                .map_err(internal)?;
            if current != expected {
                return Err(ApiError::Coded {
                    code: "stale_revision",
                    message: "the offer changed; reload and try again".into(),
                    status: axum::http::StatusCode::CONFLICT,
                });
            }
        }
        let bump_sql = if workspace.is_some() {
            "UPDATE workspaces SET provider_offer_revision = provider_offer_revision + 1 WHERE workspace_id = $1"
        } else {
            "UPDATE tenants SET provider_offer_revision = provider_offer_revision + 1 WHERE tenant_id = $1"
        };
        sqlx::query(bump_sql)
            .bind(id)
            .execute(pool)
            .await
            .map_err(internal)?;
        Ok(())
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant, workspace, expected);
        Ok(())
    }
}

pub fn offer_hash(slugs: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for slug in slugs {
        hasher.update(slug.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

/// Audit row with hashes only. A missing table does not fail the save.
pub async fn write_provider_audit(
    state: &AppState,
    actor_id: Option<Uuid>,
    action: &str,
    scope: &str,
    subject: &str,
    after_hash: &str,
) {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = state.optional_pg_pool() else {
            return;
        };
        if let Err(err) = sqlx::query(
            "INSERT INTO provider_config_audit \
             (actor_id, action, scope, subject, after_hash) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(actor_id)
        .bind(action)
        .bind(scope)
        .bind(subject)
        .bind(after_hash)
        .execute(pool)
        .await
        {
            tracing::warn!(error = %err, "provider audit insert skipped");
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, actor_id, action, scope, subject, after_hash);
    }
}
