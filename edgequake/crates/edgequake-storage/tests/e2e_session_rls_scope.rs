//! Session GUCs installed at pool checkout must satisfy message RLS.
//!
//! `messages_access` checks the conversation through `current_tenant_id()` and
//! `current_user_id()`. `conversations` also has restrictive
//! `tenant_access_guard`, so the subquery sees nothing when the session is
//! unset. A checkout under [`RlsScope`] passes. A checkout with no scope, and
//! a checkout for a different user, still fail. The fixture role is not
//! superuser and does not bypass row security.
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::adapters::postgres::rls::{RlsScope, RLS_SCOPE};
use edgequake_storage::adapters::postgres::with_session_hygiene_labeled;
use sqlx::postgres::PgConnectOptions;
use sqlx::PgPool;
use uuid::Uuid;

use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};

const WRITER_ROLE: &str = "eq_session_rls_writer";
const WRITER_PASSWORD: &str = "session_rls_writer_pw";

#[tokio::test]
async fn session_scope_lets_message_insert_pass_and_clears_on_release() {
    let Some(config) = require_or_skip_postgres("session_rls") else {
        return;
    };
    let admin = contract_pg_pool(&config).await;
    let guard: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_policies \
         WHERE schemaname = 'public' AND tablename = 'messages' \
           AND policyname = 'messages_access'",
    )
    .fetch_one(&admin)
    .await
    .expect("policy lookup");
    assert_eq!(guard, 1, "messages.messages_access must be installed");

    provision_writer(&admin).await;
    let pool = writer_pool(&config).await;
    let bypass: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&pool)
    .await
    .expect("writer role flags");
    assert!(!bypass, "fixture role must be subject to RLS");

    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let user = Uuid::new_v4();
    let other_user = Uuid::new_v4();
    let conversation = Uuid::new_v4();
    seed_scope(&admin, tenant, workspace, user).await;
    seed_conversation(&admin, conversation, tenant, workspace, user).await;

    let scope = RlsScope {
        tenant_id: tenant,
        workspace_id: Some(workspace),
        user_id: Some(user),
    };

    let installed = RLS_SCOPE
        .scope(scope.clone(), async {
            let mut conn = pool.acquire().await.expect("acquire in scope");
            let first: String =
                sqlx::query_scalar("SELECT current_setting('app.current_tenant_id', true)")
                    .fetch_one(&mut *conn)
                    .await
                    .expect("first guc");
            let second: String =
                sqlx::query_scalar("SELECT current_setting('app.current_user_id', true)")
                    .fetch_one(&mut *conn)
                    .await
                    .expect("second guc");
            (first, second)
        })
        .await;
    assert_eq!(installed.0, tenant.to_string());
    assert_eq!(installed.1, user.to_string());

    let cleared: String =
        sqlx::query_scalar("SELECT COALESCE(current_setting('app.current_tenant_id', true), '')")
            .fetch_one(&pool)
            .await
            .expect("cleared guc");
    assert!(
        cleared.is_empty(),
        "released connection must not keep the previous tenant, got {cleared}"
    );

    let bare = insert_message(&pool, conversation).await;
    let bare_msg = bare
        .expect_err("unset session must fail message RLS")
        .to_string();
    assert!(
        bare_msg.contains("row-level security"),
        "bare insert should be an RLS failure, got: {bare_msg}"
    );

    let saved = RLS_SCOPE
        .scope(scope, async { insert_message(&pool, conversation).await })
        .await
        .expect("scoped insert");
    assert_eq!(saved, 1);

    let foreign = RlsScope {
        tenant_id: tenant,
        workspace_id: Some(workspace),
        user_id: Some(other_user),
    };
    let denied = RLS_SCOPE
        .scope(foreign, async { insert_message(&pool, conversation).await })
        .await;
    let denied_msg = denied.expect_err("other user must fail").to_string();
    assert!(
        denied_msg.contains("row-level security"),
        "foreign user should be an RLS failure, got: {denied_msg}"
    );

    sqlx::query("DELETE FROM public.messages WHERE conversation_id = $1")
        .bind(conversation)
        .execute(&admin)
        .await
        .expect("cleanup messages");
    sqlx::query("DELETE FROM public.conversations WHERE conversation_id = $1")
        .bind(conversation)
        .execute(&admin)
        .await
        .expect("cleanup conversation");
    sqlx::query("DELETE FROM public.users WHERE user_id = $1")
        .bind(user)
        .execute(&admin)
        .await
        .expect("cleanup user");
    sqlx::query("DELETE FROM public.workspaces WHERE workspace_id = $1")
        .bind(workspace)
        .execute(&admin)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM public.tenants WHERE tenant_id = $1")
        .bind(tenant)
        .execute(&admin)
        .await
        .expect("cleanup tenant");
}

async fn insert_message(pool: &PgPool, conversation: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO public.messages (conversation_id, role, content, is_error) \
         VALUES ($1, 'user', 'hello', false)",
    )
    .bind(conversation)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

async fn provision_writer(admin: &PgPool) {
    sqlx::query(&format!(
        "DO $$ BEGIN \
           IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{WRITER_ROLE}') THEN \
             CREATE ROLE {WRITER_ROLE} LOGIN PASSWORD '{WRITER_PASSWORD}' \
               NOSUPERUSER NOBYPASSRLS NOINHERIT; \
           END IF; \
         END $$"
    ))
    .execute(admin)
    .await
    .expect("create writer role");
    sqlx::query(&format!(
        "ALTER ROLE {WRITER_ROLE} WITH LOGIN PASSWORD '{WRITER_PASSWORD}' \
         NOSUPERUSER NOBYPASSRLS NOINHERIT"
    ))
    .execute(admin)
    .await
    .expect("pin writer role");
    sqlx::query(&format!(
        "ALTER ROLE {WRITER_ROLE} SET role TO 'edgequake_tenant_access'"
    ))
    .execute(admin)
    .await
    .expect("default writer role");
    let db: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(admin)
        .await
        .expect("database name");
    let grants = [
        format!("GRANT CONNECT ON DATABASE \"{db}\" TO {WRITER_ROLE}"),
        format!("GRANT USAGE ON SCHEMA public TO {WRITER_ROLE}"),
        format!("GRANT edgequake_tenant_access TO {WRITER_ROLE}"),
        format!("GRANT SELECT, INSERT ON public.messages TO {WRITER_ROLE}"),
        format!("GRANT SELECT ON public.conversations TO {WRITER_ROLE}"),
    ];
    for stmt in grants {
        sqlx::query(&stmt)
            .execute(admin)
            .await
            .unwrap_or_else(|error| panic!("grant failed ({stmt}): {error}"));
    }
}

async fn writer_pool(config: &edgequake_storage::PostgresConfig) -> PgPool {
    let options = PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .database(&config.database)
        .username(WRITER_ROLE)
        .password(WRITER_PASSWORD);
    with_session_hygiene_labeled(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .min_connections(0),
        "edgequake:default",
    )
    .connect_with(options)
    .await
    .expect("writer pool")
}

async fn seed_scope(pool: &PgPool, tenant_id: Uuid, workspace_id: Uuid, user_id: Uuid) {
    let slug = format!("session-rls-{}", tenant_id.as_simple());
    sqlx::query(
        "INSERT INTO public.tenants (tenant_id, name, slug) VALUES ($1, $2, $3) \
         ON CONFLICT (tenant_id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind("session rls")
    .bind(slug)
    .execute(pool)
    .await
    .expect("seed tenant");
    let ws = format!("session-rls-ws-{}", workspace_id.as_simple());
    sqlx::query(
        "INSERT INTO public.workspaces (workspace_id, tenant_id, name, slug) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (workspace_id) DO NOTHING",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind("session rls ws")
    .bind(ws)
    .execute(pool)
    .await
    .expect("seed workspace");
    sqlx::query(
        "INSERT INTO public.users (user_id, tenant_id, email, username) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(format!("session-rls-{user_id}@example.com"))
    .bind(format!("session-rls-{}", user_id.as_simple()))
    .execute(pool)
    .await
    .expect("seed user");
}

async fn seed_conversation(
    pool: &PgPool,
    conversation_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    user_id: Uuid,
) {
    sqlx::query(
        "INSERT INTO public.conversations ( \
            conversation_id, tenant_id, workspace_id, user_id, title, mode, meta \
         ) VALUES ($1, $2, $3, $4, 'session rls', 'hybrid', '{}')",
    )
    .bind(conversation_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed conversation");
}
