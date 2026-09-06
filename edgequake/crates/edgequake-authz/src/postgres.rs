//! Postgres-backed AllowSetProvider (LAW-146-18).
//!
//! SQL/set algebra for workspace|acl|owner_only; Cedar only for classified.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use cedar_policy::{Authorizer, Context, Decision, Entities, EntityUid, PolicySet, Request};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::allow_set::{AllowSet, AllowSetProvider};
use crate::cedar_schema::{compile_default_policy_set, DEFAULT_CLASSIFIED_POLICY};
use crate::context::AuthzContext;
use crate::error::{AuthzError, AuthzResult};
use crate::principal::PrincipalId;

pub struct PostgresAllowSetProvider {
    pool: PgPool,
    policy_set: Arc<PolicySet>,
    authorizer: Authorizer,
}

impl PostgresAllowSetProvider {
    pub fn new(pool: PgPool) -> AuthzResult<Self> {
        let policy_set = Arc::new(compile_default_policy_set()?);
        Ok(Self {
            pool,
            policy_set,
            authorizer: Authorizer::new(),
        })
    }

    pub fn with_policy_text(pool: PgPool, cedar_text: &str) -> AuthzResult<Self> {
        use std::str::FromStr;
        let policy_set = Arc::new(
            PolicySet::from_str(cedar_text).map_err(|e| AuthzError::Cedar(e.to_string()))?,
        );
        Ok(Self {
            pool,
            policy_set,
            authorizer: Authorizer::new(),
        })
    }

    /// Bump workspace policy_generation (LAW-146-22).
    pub async fn increment_policy_generation(&self, workspace_id: Uuid) -> AuthzResult<u64> {
        let row = sqlx::query(
            r#"
            INSERT INTO workspace_authz_state (workspace_id, policy_generation, updated_at)
            VALUES ($1, 2, NOW())
            ON CONFLICT (workspace_id) DO UPDATE
              SET policy_generation = workspace_authz_state.policy_generation + 1,
                  updated_at = NOW()
            RETURNING policy_generation
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AuthzError::Storage(e.to_string()))?;
        Ok(row.get::<i64, _>(0) as u64)
    }

    pub async fn fetch_policy_generation(&self, workspace_id: Uuid) -> AuthzResult<u64> {
        sqlx::query(
            r#"
            INSERT INTO workspace_authz_state (workspace_id, policy_generation)
            VALUES ($1, 1)
            ON CONFLICT (workspace_id) DO NOTHING
            "#,
        )
        .bind(workspace_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AuthzError::Storage(e.to_string()))?;

        let row = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT policy_generation FROM workspace_authz_state WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AuthzError::Storage(e.to_string()))?;
        Ok(row as u64)
    }

    async fn active_break_glass(
        &self,
        ctx: &AuthzContext,
    ) -> AuthzResult<Option<Option<Vec<Uuid>>>> {
        // Outer None = no session; Inner None = all non-quarantined; Some(ids) = scoped.
        let row = sqlx::query(
            r#"
            SELECT scope_doc_ids FROM break_glass_sessions
            WHERE workspace_id = $1
              AND principal_kind = $2
              AND principal_id = $3
              AND revoked_at IS NULL
              AND expires_at > NOW()
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(ctx.workspace_id)
        .bind(ctx.principal.kind_str())
        .bind(ctx.principal.id_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AuthzError::Storage(e.to_string()))?;

        Ok(row.map(|r| {
            r.try_get::<Option<Vec<Uuid>>, _>("scope_doc_ids")
                .ok()
                .flatten()
        }))
    }

    fn cedar_allows_classified(&self, ctx: &AuthzContext, classification: &str) -> bool {
        // Build minimal entities for principal + document.
        let clearance = ctx
            .subject_attrs
            .get("clearance")
            .and_then(|v| v.as_str())
            .unwrap_or("internal");
        let department = ctx
            .subject_attrs
            .get("department")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Prefer fail-closed on entity construction errors.
        let principal_uid: EntityUid = match format!("User::\"{}\"", ctx.principal.id_str()).parse()
        {
            Ok(u) => u,
            Err(_) => return false,
        };
        let resource_uid: EntityUid = match format!("Document::\"eval\"").parse() {
            Ok(u) => u,
            Err(_) => return false,
        };
        let action_uid: EntityUid = match "Action::\"document_read\"".parse() {
            Ok(u) => u,
            Err(_) => return false,
        };

        let principal_json = serde_json::json!({
            "uid": {"type": "User", "id": ctx.principal.id_str()},
            "attrs": {
                "clearance": clearance,
                "department": department,
            },
            "parents": []
        });
        let resource_json = serde_json::json!({
            "uid": {"type": "Document", "id": "eval"},
            "attrs": {
                "classification": classification,
                "share_mode": "classified",
                "project_id": "",
                "export_control": false,
                "pii": false,
            },
            "parents": []
        });
        let entities = match Entities::from_json_value(
            serde_json::json!([principal_json, resource_json]),
            None,
        ) {
            Ok(e) => e,
            Err(_) => return false,
        };

        let request = match Request::new(
            principal_uid,
            action_uid,
            resource_uid,
            Context::empty(),
            None,
        ) {
            Ok(r) => r,
            Err(_) => return false,
        };

        let response = self
            .authorizer
            .is_authorized(&request, &self.policy_set, &entities);
        // Cedar skips erroring policies (fail-open risk). SPEC-146: deny on any
        // evaluation diagnostic so classified PDP stays fail-closed.
        if response.diagnostics().errors().next().is_some() {
            return false;
        }
        response.decision() == Decision::Allow
    }
}

#[async_trait]
impl AllowSetProvider for PostgresAllowSetProvider {
    async fn current_policy_generation(&self, workspace_id: Uuid) -> AuthzResult<u64> {
        self.fetch_policy_generation(workspace_id).await
    }

    async fn bump_policy_generation(&self, workspace_id: Uuid) -> AuthzResult<u64> {
        self.increment_policy_generation(workspace_id).await
    }

    async fn documents_for(&self, ctx: &AuthzContext) -> AuthzResult<AllowSet> {
        if !ctx.abac_enabled {
            return Err(AuthzError::Disabled);
        }

        // Worker never gets a content allow-set via query path (LAW-146-13).
        if matches!(ctx.principal, PrincipalId::Worker) {
            return Ok(AllowSet::empty());
        }

        // Break-glass (LAW-146-25)
        if let Some(scope_opt) = self.active_break_glass(ctx).await? {
            match scope_opt {
                None => {
                    let rows = sqlx::query_scalar::<_, Uuid>(
                        r#"
                        SELECT id FROM documents
                        WHERE workspace_id = $1 AND security_status = 'ok'
                        "#,
                    )
                    .bind(ctx.workspace_id)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| AuthzError::Storage(e.to_string()))?;
                    return Ok(AllowSet::from_ids(rows));
                }
                Some(scope) => return Ok(AllowSet::from_ids(scope)),
            }
        }

        // Master without BG session: empty (must create BG session) — fail closed.
        if matches!(ctx.principal, PrincipalId::Master) {
            return Ok(AllowSet::empty());
        }

        let rows = sqlx::query(
            r#"
            SELECT d.id, d.share_mode, d.classification,
                   d.owner_principal_kind, d.owner_principal_id,
                   EXISTS (
                     SELECT 1 FROM document_acl a
                     WHERE a.document_id = d.id
                       AND a.principal_kind = $2
                       AND a.principal_id = $3
                       AND a.permission IN ('document:read', 'document:write')
                   ) AS acl_ok
            FROM documents d
            WHERE d.workspace_id = $1
              AND d.security_status = 'ok'
            "#,
        )
        .bind(ctx.workspace_id)
        .bind(ctx.principal.kind_str())
        .bind(ctx.principal.id_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AuthzError::Storage(e.to_string()))?;

        let mut allowed = HashSet::new();
        let p_kind = ctx.principal.kind_str();
        let p_id = ctx.principal.id_str();

        for row in rows {
            let id: Uuid = row.get("id");
            let share_mode: String = row.get("share_mode");
            let classification: String = row.get("classification");
            let owner_kind: String = row.get("owner_principal_kind");
            let owner_id: Option<String> = row.get("owner_principal_id");
            let acl_ok: bool = row.get("acl_ok");

            let include = match share_mode.as_str() {
                "workspace" => true, // capability gated at PEP
                "owner_only" => {
                    owner_kind == p_kind && owner_id.as_deref() == Some(p_id.as_str())
                }
                "acl" => {
                    acl_ok
                        || (owner_kind == p_kind && owner_id.as_deref() == Some(p_id.as_str()))
                }
                "classified" => self.cedar_allows_classified(ctx, &classification),
                _ => false,
            };
            if include {
                allowed.insert(id);
            }
        }

        let _ = DEFAULT_CLASSIFIED_POLICY; // keep linked for docs
        Ok(AllowSet {
            document_ids: allowed,
        })
    }
}
