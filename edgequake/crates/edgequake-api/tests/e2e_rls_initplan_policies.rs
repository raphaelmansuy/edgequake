//! Migration 208 promised this guard and never shipped it; the invariant then
//! rotted until five later migrations reintroduced the bare form.
//!
//! A bare `current_tenant_id()` in a policy becomes a per-row Filter the
//! executor re-evaluates for every row a scan emits. Wrapped as
//! `(SELECT current_tenant_id())` it becomes an InitPlan, evaluated once per
//! statement. `app.current_*` only change between statements, so the two forms
//! are the same value — but only the wrapped one is O(1) instead of O(rows).
//!
//! Skips unless `DATABASE_URL` points at a database with the RLS policies.
#![cfg(feature = "postgres")]

use sqlx::{PgPool, Row};

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.is_empty())?;
    PgPool::connect(&url).await.ok()
}

/// Every policy expression must use the wrapped form.
///
/// The check is on the deparsed policy text, which is what the planner sees:
/// a call is bare when it is not immediately preceded by `SELECT`.
#[tokio::test]
async fn e2e_rls_initplan_policies() {
    let Some(pool) = pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };

    let policies: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query(
        "SELECT tablename, policyname, qual, with_check
           FROM pg_policies
          WHERE schemaname = 'public'",
    )
    .fetch_all(&pool)
    .await
    .expect("read pg_policies")
    .iter()
    .map(|row| {
        (
            row.get::<String, _>("tablename"),
            row.get::<String, _>("policyname"),
            row.get::<Option<String>, _>("qual"),
            row.get::<Option<String>, _>("with_check"),
        )
    })
    .collect();

    assert!(
        !policies.is_empty(),
        "no policies found — is this a database with migrations applied?"
    );

    let mut bare: Vec<String> = Vec::new();
    for (table, policy, qual, with_check) in &policies {
        for (clause, expr) in [("USING", qual), ("WITH CHECK", with_check)] {
            let Some(expr) = expr else { continue };
            // Strip the wrapped form, then see whether a bare call remains.
            let unwrapped = expr
                .replace("SELECT current_tenant_id() AS current_tenant_id", "")
                .replace("SELECT current_workspace_id() AS current_workspace_id", "")
                .replace("SELECT current_user_id() AS current_user_id", "")
                .replace("SELECT public.current_tenant_id() AS current_tenant_id", "")
                .replace(
                    "SELECT public.current_workspace_id() AS current_workspace_id",
                    "",
                )
                .replace("SELECT public.current_user_id() AS current_user_id", "");
            for helper in [
                "current_tenant_id()",
                "current_workspace_id()",
                "current_user_id()",
            ] {
                if unwrapped.contains(helper) {
                    bare.push(format!("{table}.{policy} {clause}: {helper}"));
                }
            }
        }
    }

    assert!(
        bare.is_empty(),
        "policy expressions must wrap session helpers as (SELECT fn()) so the \
         planner treats them as one InitPlan instead of a per-row filter:\n  {}",
        bare.join("\n  ")
    );
}

/// The wrapper must not change what the policy means.
///
/// `app.current_*` are fixed for the statement, so the wrapped value must equal
/// the bare one for the three states that could differ: unset, set, malformed.
#[tokio::test]
async fn e2e_rls_initplan_wrapping_is_semantics_preserving() {
    let Some(pool) = pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };

    // The helpers only exist once migration 001 has run.
    let present: bool =
        sqlx::query_scalar("SELECT to_regprocedure('public.current_tenant_id()') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap_or(false);
    if !present {
        eprintln!("SKIP: current_tenant_id() absent");
        return;
    }

    let mut conn = pool.acquire().await.expect("connection");
    sqlx::raw_sql("BEGIN")
        .execute(&mut *conn)
        .await
        .expect("begin");

    let cases: [(&str, &str); 3] = [
        ("unset", ""),
        ("set", "00000000-0000-0000-0000-000000000002"),
        ("malformed", "not-a-uuid"),
    ];
    for (label, value) in cases {
        sqlx::query("SELECT set_config('app.current_tenant_id', $1, true)")
            .bind(value)
            .execute(&mut *conn)
            .await
            .expect("set context");

        let row = sqlx::query(
            "SELECT current_tenant_id() AS bare,
                    (SELECT current_tenant_id()) AS wrapped",
        )
        .fetch_one(&mut *conn)
        .await
        .expect("compare");
        let bare: Option<uuid::Uuid> = row.get("bare");
        let wrapped: Option<uuid::Uuid> = row.get("wrapped");
        assert_eq!(
            bare, wrapped,
            "({label}) wrapped helper must return what the bare call returns"
        );
    }

    let _ = sqlx::raw_sql("ROLLBACK").execute(&mut *conn).await;
}
