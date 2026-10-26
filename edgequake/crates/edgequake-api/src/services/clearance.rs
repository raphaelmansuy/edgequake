//! Document clearance for one request (SPEC-174).
//!
//! `bind_request` is the only caller that loads rules. Handlers read the
//! scope it stored. SQL writes go through this module so a handler cannot
//! update `classification_rank` on its own.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use axum::extract::Request;
use axum::response::IntoResponse;
use edgequake_audit::{AuditEventType, AuditResult};
use edgequake_core::clearance::{
    compile_subject, guc_compartment_ids, rank_name, ClearanceRule, Effect, MatchKind, RoleGrant,
    VisibilityScope, RANK_INTERNAL, RANK_UNVERIFIED,
};
use edgequake_storage::clearance_session::ClearanceSession;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::RequestAuthContext;
use crate::handlers::documents_types::CompartmentView;
use crate::middleware::TenantContext;
use crate::state::AppState;

/// Process flag. Read once. Unset means off.
pub fn break_glass_enabled() -> bool {
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(|| {
        matches!(
            std::env::var("EDGEQUAKE_CLEARANCE_BREAK_GLASS").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        )
    })
}

/// Load the caller's scope and store it on the request.
///
/// No pool means enforcement off. That is the path unit tests and a binary
/// built without Postgres take. A pool error fails the request.
pub async fn attach_clearance(
    state: &AppState,
    request: &mut Request,
    auth: &RequestAuthContext,
    ctx: &TenantContext,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        let workspace = ctx
            .workspace_id
            .as_deref()
            .and_then(|value| Uuid::parse_str(value).ok());
        let user_id = Uuid::parse_str(&auth.user_id).ok();
        let (scope, session) = if let Some(workspace_id) = workspace {
            load_scope(pool, workspace_id, user_id).await?
        } else {
            (VisibilityScope::enforcement_off(), ClearanceSession::off())
        };
        if scope.subject.break_glass && scope.subject.enforcement {
            crate::services::audit::record_compliance_event(
                state,
                ctx.tenant_id.clone().unwrap_or_default(),
                AuditEventType::Authorization,
                "clearance.break_glass",
                AuditResult::Warning,
                ctx.workspace_id.clone(),
                Some(auth.user_id.clone()),
                None,
            );
        }
        request.extensions_mut().insert(scope);
        request.extensions_mut().insert(session);
        return Ok(());
    }
    request
        .extensions_mut()
        .insert(VisibilityScope::enforcement_off());
    request.extensions_mut().insert(ClearanceSession::off());
    Ok(())
}

pub fn scope_on(request: &Request) -> VisibilityScope {
    request
        .extensions()
        .get::<VisibilityScope>()
        .cloned()
        .unwrap_or_else(VisibilityScope::enforcement_off)
}

/// Run the rest of the request with the session `bind_request` stored.
pub async fn run_scoped(
    request: Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let session = request
        .extensions()
        .get::<ClearanceSession>()
        .cloned()
        .unwrap_or_else(ClearanceSession::off);
    let path = request.uri().path().to_string();
    let fence_response = request.method() == axum::http::Method::GET
        || path.contains("/query")
        || path.contains("/chat");
    let response_session = session.clone();
    let workspace = request
        .extensions()
        .get::<TenantContext>()
        .and_then(|ctx| ctx.workspace_id_uuid());
    edgequake_storage::clearance_session::scope(session, async move {
        if let Some(document_id) = document_id_in_path(&path) {
            match must_hide(document_id).await {
                Ok(true) => {
                    return not_found(document_id).into_response();
                }
                Ok(false) => {}
                Err(error) => return error.into_response(),
            }
        }
        if let Some(pdf_id) = pdf_id_in_path(&path) {
            match must_hide_pdf(pdf_id, workspace).await {
                Ok(true) => return ApiError::NotFound("PDF not found".into()).into_response(),
                Ok(false) => {}
                Err(error) => return error.into_response(),
            }
        }
        let mut response = next.run(request).await;
        if path.contains("/clearance/")
            || path.ends_with("/access")
            || path.ends_with("/labels")
            || path.ends_with("/labels/batch")
        {
            response.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("private, no-store"),
            );
        }
        #[cfg(feature = "postgres")]
        if fence_response && response_session.enforced && !response_session.system {
            if let (Some(workspace), Some(pool)) = (
                workspace,
                crate::services::relational_sidecar_store::sidecar_pool(),
            ) {
                return fence_clearance_response(response, pool, workspace, response_session);
            }
        }
        response
    })
    .await
}

/// Revalidate immediately before each body chunk leaves the process. An epoch
/// or subject change aborts delivery, including an already-open SSE stream.
#[cfg(feature = "postgres")]
fn fence_clearance_response(
    response: axum::response::Response,
    pool: std::sync::Arc<sqlx::PgPool>,
    workspace: Uuid,
    session: ClearanceSession,
) -> axum::response::Response {
    use futures::StreamExt;
    let (mut parts, body) = response.into_parts();
    parts.headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    let stream = futures::stream::unfold(
        (body.into_data_stream(), false),
        move |(mut body, stopped)| {
            let pool = pool.clone();
            let session = session.clone();
            async move {
                if stopped {
                    return None;
                }
                let chunk = body.next().await?;
                if let Err(error) = &chunk {
                    tracing::warn!(%error,"Response body failed before access fence");
                }
                let valid = match load_scope(&pool, workspace, session.user_id).await {
                    Ok((_, current)) => {
                        let valid = current.policy_epoch == session.policy_epoch
                            && current.fingerprint == session.fingerprint;
                        if !valid {
                            tracing::debug!(
                                expected_epoch = session.policy_epoch,
                                current_epoch = current.policy_epoch,
                                "Response access snapshot changed"
                            );
                        }
                        valid
                    }
                    Err(error) => {
                        tracing::warn!(%error,"Response access check failed");
                        false
                    }
                };
                let result = if valid {
                    chunk.map_err(std::io::Error::other)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Access changed; reload before continuing",
                    ))
                };
                Some((result, (body, !valid)))
            }
        },
    );
    // HTTP compression may poll again after end-of-stream; Unfold alone panics.
    axum::response::Response::from_parts(parts, axum::body::Body::from_stream(stream.fuse()))
}

/// `/api/v1/documents/{uuid}` and its children. List and upload have no id.
fn document_id_in_path(path: &str) -> Option<&str> {
    let mut parts = path.split('/');
    while let Some(part) = parts.next() {
        if part == "documents" {
            let id = parts.next()?;
            return Uuid::parse_str(id).is_ok().then_some(id);
        }
    }
    None
}

/// Axum nested routers may strip `/api/v1` before middleware runs.
fn pdf_id_in_path(path: &str) -> Option<Uuid> {
    let (_, tail) = path.split_once("/documents/pdf/")?;
    Uuid::parse_str(tail.split('/').next()?).ok()
}

/// `true` when an enforced caller must not see this id. Missing and hidden
/// are the same answer.
pub async fn must_hide(document_id: &str) -> Result<bool, ApiError> {
    let Some(session) = ClearanceSession::current() else {
        return Ok(false);
    };
    if session.predicate_sql().is_empty() {
        return Ok(false);
    }
    let Ok(id) = Uuid::parse_str(document_id) else {
        return Ok(true);
    };
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = crate::services::relational_sidecar_store::sidecar_pool() else {
            return Ok(true);
        };
        return row_hidden(pool.as_ref(), id, &session).await;
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = id;
        Ok(false)
    }
}

/// Same not-found error a missing id already produces.
/// PDF UUIDs are identifiers, not capabilities. Guard every PDF-ID route
/// before the handler looks up metadata or streams bytes from either store.
pub async fn must_hide_pdf(pdf_id: Uuid, workspace: Option<Uuid>) -> Result<bool, ApiError> {
    let Some(session) =
        ClearanceSession::current().filter(|session| session.enforced && !session.system)
    else {
        return Ok(false);
    };
    let Some(workspace) = workspace else {
        return Ok(true);
    };
    #[cfg(feature = "postgres")]
    {
        let pool = crate::services::relational_sidecar_store::sidecar_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|_| ApiError::Internal("Could not check PDF access".into()))?;
        edgequake_storage::adapters::postgres::rls::apply_clearance_session(&mut tx, &session)
            .await?;
        let visible:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public.pdf_documents WHERE pdf_id=$1 AND workspace_id=$2 AND (public.clearance_covers(classification_rank,compartment_ids) OR (classification_rank=5 AND uploaded_by=public.current_user_id())))")
            .bind(pdf_id).bind(workspace).fetch_one(&mut *tx).await.map_err(|_|ApiError::Internal("Could not check PDF access".into()))?;
        tx.commit()
            .await
            .map_err(|_| ApiError::Internal("Could not check PDF access".into()))?;
        Ok(!visible)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (session, pdf_id, workspace);
        Ok(true)
    }
}

pub fn not_found(document_id: &str) -> ApiError {
    ApiError::NotFound(format!("Document {document_id} not found"))
}

#[cfg(feature = "postgres")]
async fn row_hidden(
    pool: &sqlx::PgPool,
    id: Uuid,
    session: &ClearanceSession,
) -> Result<bool, ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    edgequake_storage::adapters::postgres::rls::apply_clearance_session(&mut tx, session)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    let found: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM public.documents WHERE id = $1 AND (public.clearance_covers(classification_rank, compartment_ids) OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5))",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(found.is_none())
}

#[cfg(feature = "postgres")]
pub async fn load_scope(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<(VisibilityScope, ClearanceSession), ApiError> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    load_scope_conn(&mut conn, workspace_id, user_id).await
}

#[cfg(feature = "postgres")]
pub async fn load_scope_conn(
    conn: &mut sqlx::PgConnection,
    workspace_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<(VisibilityScope, ClearanceSession), ApiError> {
    let settings: Option<(bool, i64)> = sqlx::query_as(
        "SELECT enforcement, policy_epoch FROM public.workspace_clearance_settings WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let (enforcement, epoch) = settings.unwrap_or((false, 0));
    let grants = if let Some(user_id) = user_id {
        load_grants(conn, workspace_id, user_id).await?
    } else {
        Vec::new()
    };
    let mut subject = compile_subject(&grants, user_id);
    subject.enforcement = enforcement;
    subject.break_glass = break_glass_enabled();
    let vocabulary = load_vocabulary(conn, workspace_id).await?;
    let scope = VisibilityScope::new(subject, epoch);
    let session = session_from(&scope, &vocabulary);
    Ok((scope, session))
}

#[cfg(feature = "postgres")]
fn session_from(scope: &VisibilityScope, vocabulary: &[Uuid]) -> ClearanceSession {
    ClearanceSession {
        enforced: scope.subject.enforcement,
        break_glass: scope.subject.break_glass,
        system: false,
        steward: scope.subject.steward,
        rank: i16::from(scope.subject.rank),
        compartments: guc_compartment_ids(&scope.subject, vocabulary)
            .into_iter()
            .collect(),
        allow_empty: scope.subject.allow_empty_compartments,
        user_id: scope.subject.user_id,
        policy_epoch: scope.policy_epoch,
        fingerprint: scope.fingerprint.clone(),
    }
}

#[cfg(feature = "postgres")]
async fn load_vocabulary(
    conn: &mut sqlx::PgConnection,
    workspace_id: Uuid,
) -> Result<Vec<Uuid>, ApiError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT tag_id FROM public.clearance_tags WHERE workspace_id = $1 AND kind = 'compartment'",
    )
    .bind(workspace_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

#[cfg(feature = "postgres")]
#[derive(sqlx::FromRow)]
struct RuleRow {
    role_id: Uuid,
    steward: bool,
    effect: Option<String>,
    match_kind: Option<String>,
    rank: Option<i16>,
    tag_ids: Option<Vec<Uuid>>,
}

#[cfg(feature = "postgres")]
async fn load_grants(
    conn: &mut sqlx::PgConnection,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<RoleGrant>, ApiError> {
    let rows: Vec<RuleRow> = sqlx::query_as(
        "SELECT r.role_id, r.steward, ru.effect, ru.match_kind, ru.rank, ru.tag_ids
         FROM public.memberships m
         JOIN public.membership_security_roles ms ON ms.membership_id = m.membership_id
         JOIN public.security_roles r ON r.role_id = ms.role_id
         LEFT JOIN public.security_role_rules ru ON ru.role_id = r.role_id
         WHERE m.user_id = $1 AND m.workspace_id = $2 AND m.is_active
         ORDER BY r.role_id, ru.position",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let mut grouped: BTreeMap<Uuid, RoleGrant> = BTreeMap::new();
    for row in rows {
        let grant = grouped.entry(row.role_id).or_insert(RoleGrant {
            steward: row.steward,
            rules: Vec::new(),
        });
        if let Some(rule) = rule_from_row(&row) {
            grant.rules.push(rule);
        }
    }
    Ok(grouped.into_values().collect())
}

#[cfg(feature = "postgres")]
fn rule_from_row(row: &RuleRow) -> Option<ClearanceRule> {
    let effect = match row.effect.as_deref()? {
        "allow" => Effect::Allow,
        "deny" => Effect::Deny,
        _ => return None,
    };
    let kind = match row.match_kind.as_deref()? {
        "classification_at_most" => MatchKind::ClassificationAtMost {
            rank: u8::try_from(row.rank.unwrap_or(0)).unwrap_or(0),
        },
        "any_of" => MatchKind::AnyOf {
            tag_ids: row
                .tag_ids
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect(),
        },
        "all_of" => MatchKind::AllOf {
            tag_ids: row
                .tag_ids
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect(),
        },
        "untagged" => MatchKind::Untagged,
        _ => return None,
    };
    Some(ClearanceRule { effect, kind })
}

/// Labels for the documents a caller is already allowed to see.
/// Document rank, then one vocabulary tag the document carries (if any).
#[cfg(feature = "postgres")]
type LabelRow = (
    Uuid,
    Option<i16>,
    Option<Uuid>,
    Option<String>,
    Option<String>,
);

#[cfg(feature = "postgres")]
pub async fn labels_for(
    pool: &sqlx::PgPool,
    ids: &[Uuid],
) -> Result<BTreeMap<String, (Option<String>, Vec<CompartmentView>)>, ApiError> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    labels_for_conn(&mut conn, ids).await
}

#[cfg(feature = "postgres")]
pub async fn labels_for_conn(
    conn: &mut sqlx::PgConnection,
    ids: &[Uuid],
) -> Result<BTreeMap<String, (Option<String>, Vec<CompartmentView>)>, ApiError> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows: Vec<LabelRow> = sqlx::query_as(
        "SELECT d.id, d.classification_rank, t.tag_id, t.key, t.value
         FROM public.documents d
         LEFT JOIN public.document_labels dl ON dl.document_id = d.id
         LEFT JOIN public.clearance_tags t ON t.tag_id = dl.tag_id AND t.kind = 'compartment'
         WHERE d.id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let mut out: BTreeMap<String, (Option<String>, Vec<CompartmentView>)> = BTreeMap::new();
    for (id, rank, tag_id, key, value) in rows {
        let entry = out.entry(id.to_string()).or_insert_with(|| {
            (
                rank.and_then(|rank| u8::try_from(rank).ok())
                    .and_then(rank_name)
                    .map(str::to_string),
                Vec::new(),
            )
        });
        if let (Some(tag_id), Some(key), Some(value)) = (tag_id, key, value) {
            if !entry.1.iter().any(|item| item.id == tag_id.to_string()) {
                entry.1.push(CompartmentView {
                    id: tag_id.to_string(),
                    key,
                    value,
                });
            }
        }
    }
    Ok(out)
}

#[cfg(feature = "postgres")]
pub async fn membership_role(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    workspace_id: Uuid,
) -> Result<Option<String>, ApiError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role FROM public.memberships WHERE user_id = $1 AND workspace_id = $2 AND is_active LIMIT 1",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(row.map(|(role,)| role))
}

/// Seed classifications, backfill ranks, grant steward, then turn enforcement on.
#[cfg(feature = "postgres")]
pub async fn enable(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
    strict_start: bool,
) -> Result<(), ApiError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    sqlx::query(
        "INSERT INTO public.clearance_tags (tag_id, workspace_id, kind, key, value, rank)
         VALUES
           (gen_random_uuid(), $1, 'classification', 'public', 'public', 1),
           (gen_random_uuid(), $1, 'classification', 'internal', 'internal', 2),
           (gen_random_uuid(), $1, 'classification', 'confidential', 'confidential', 3),
           (gen_random_uuid(), $1, 'classification', 'restricted', 'restricted', 4),
           (gen_random_uuid(), $1, 'classification', 'unverified', 'unverified', 5)
         ON CONFLICT (workspace_id, kind, key, value) DO NOTHING",
    )
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let rank: i16 = if strict_start {
        i16::from(RANK_UNVERIFIED)
    } else {
        i16::from(RANK_INTERNAL)
    };
    sqlx::query(
        "UPDATE public.documents
         SET classification_rank = $2, compartment_ids = COALESCE(compartment_ids, '{}')
         WHERE workspace_id = $1 AND classification_rank IS NULL",
    )
    .bind(workspace_id)
    .bind(rank)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    copy_labels(&mut tx, workspace_id).await?;
    let role_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO public.security_roles (role_id, workspace_id, name, steward)
         VALUES (gen_random_uuid(), $1, 'Steward', true)
         ON CONFLICT (workspace_id, name) DO UPDATE SET steward = true
         RETURNING role_id",
    )
    .bind(workspace_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    sqlx::query(
        "INSERT INTO public.membership_security_roles (membership_id, role_id)
         SELECT m.membership_id, $2
         FROM public.memberships m
         WHERE m.user_id = $1 AND m.workspace_id = $3 AND m.is_active
         ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(role_id.0)
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    sqlx::query(
        "INSERT INTO public.workspace_clearance_settings (workspace_id, enforcement, policy_epoch)
         VALUES ($1, true, 1)
         ON CONFLICT (workspace_id) DO UPDATE
         SET enforcement = true, policy_epoch = public.workspace_clearance_settings.policy_epoch + 1",
    )
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn copy_labels(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<(), ApiError> {
    for table in [
        "chunks",
        "document_originals",
        "pdf_documents",
        "document_mm_assets",
        "document_pages",
        "tasks",
        "graph_contributions",
    ] {
        let sql = match table {
            "graph_contributions" => "UPDATE public.graph_contributions AS child
                 SET classification_rank = d.classification_rank,
                     compartment_ids = d.compartment_ids,
                     uploaded_by = d.uploaded_by
                 FROM public.documents AS d
                 WHERE child.source_document_id = d.id
                   AND d.workspace_id = $1
                   AND child.classification_rank IS NULL"
                .to_string(),
            "tasks" => "UPDATE public.tasks AS child
                 SET classification_rank = d.classification_rank,
                     compartment_ids = d.compartment_ids,
                     uploaded_by = d.uploaded_by
                 FROM public.documents AS d
                 WHERE child.document_id = d.id::text
                   AND d.workspace_id = $1
                   AND child.classification_rank IS NULL"
                .to_string(),
            _ => format!(
                "UPDATE public.{table} AS child
                 SET classification_rank = d.classification_rank,
                     compartment_ids = d.compartment_ids,
                     uploaded_by = d.uploaded_by
                 FROM public.documents AS d
                 WHERE child.document_id = d.id
                   AND d.workspace_id = $1
                   AND child.classification_rank IS NULL"
            ),
        };
        sqlx::query(&sql)
            .bind(workspace_id)
            .execute(&mut **tx)
            .await
            .map_err(|error| ApiError::Internal(format!("{table}: {error}")))?;
    }
    Ok(())
}

/// Store an omitted upload label as unverified.
#[cfg(feature = "postgres")]
pub async fn stamp_unverified(
    pool: &sqlx::PgPool,
    document_id: Uuid,
    uploaded_by: Option<Uuid>,
) -> Result<(), ApiError> {
    let session = ClearanceSession::current();
    let enforced = session.as_ref().is_some_and(|session| session.enforced);
    if !enforced {
        return Ok(());
    }
    sqlx::query(
        "UPDATE public.documents
         SET classification_rank = $2, uploaded_by = COALESCE(uploaded_by, $3),
             compartment_ids = COALESCE(compartment_ids, '{}')
         WHERE id = $1 AND classification_rank IS NULL",
    )
    .bind(document_id)
    .bind(i16::from(RANK_UNVERIFIED))
    .bind(uploaded_by)
    .execute(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(())
}

#[cfg(all(test, feature = "postgres"))]
#[path = "clearance_fence_tests.rs"]
mod fence_tests;
