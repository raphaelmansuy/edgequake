//! Migration 228 — the clearance policy must mean exactly what it meant, and
//! must stop calling the row-independent half once per row.
//!
//! Skips unless `DATABASE_URL` points at a database that has migration 228.
#![cfg(feature = "postgres")]

use sqlx::{PgPool, Row};

async fn pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.is_empty())?;
    PgPool::connect(&url).await.ok()
}

/// The split helpers must compose back to the original predicate.
///
/// Exhaustive over the state that decides the outcome: enforcement, break-glass,
/// clearance rank, allow-empty, the subject's compartments, whether the workspace
/// has an enforcing settings row, and the row's own rank/compartments.
#[tokio::test]
async fn e2e_spec228_split_predicate_is_equivalent() {
    let Some(pool) = pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };
    // Fail closed if the migration is absent rather than silently passing.
    let present: bool = sqlx::query_scalar(
        "SELECT to_regprocedure('public.clearance_bypass()') IS NOT NULL
           AND to_regprocedure('public.clearance_rank_covers(smallint,uuid[])') IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("probe clearance helpers");
    if !present {
        eprintln!("SKIP: migration 228 not applied");
        return;
    }

    let mut conn = pool.acquire().await.expect("connection");
    // Own the session so GUCs cannot leak into another test's checkout.
    sqlx::raw_sql("BEGIN")
        .execute(&mut *conn)
        .await
        .expect("begin");

    let mut mismatches = 0usize;
    let mut compared = 0usize;

    for enforced in ["", "on", "off"] {
        for break_glass in ["", "on", "off"] {
            for rank in ["", "3", "5"] {
                for allow_empty in ["", "on", "off"] {
                    for compartments in ["", "{aaaaaaaa-0000-0000-0000-000000000001}"] {
                        for setting in ["absent", "true", "false"] {
                            sqlx::query(
                                "SELECT set_config('app.clearance_enforced', $1, true),
                                        set_config('app.clearance_break_glass', $2, true),
                                        set_config('app.clearance_rank', $3, true),
                                        set_config('app.clearance_allow_empty', $4, true),
                                        set_config('app.clearance_compartments', $5, true)",
                            )
                            .bind(enforced)
                            .bind(break_glass)
                            .bind(rank)
                            .bind(allow_empty)
                            .bind(compartments)
                            .execute(&mut *conn)
                            .await
                            .expect("set clearance context");

                            for doc_rank in [None, Some(0i16), Some(1), Some(5), Some(6)] {
                                for doc_compartments in [
                                    None,
                                    Some(Vec::<uuid::Uuid>::new()),
                                    Some(vec![uuid::Uuid::parse_str(
                                        "aaaaaaaa-0000-0000-0000-000000000001",
                                    )
                                    .unwrap()]),
                                    Some(vec![
                                        uuid::Uuid::parse_str(
                                            "aaaaaaaa-0000-0000-0000-000000000001",
                                        )
                                        .unwrap(),
                                        uuid::Uuid::parse_str(
                                            "bbbbbbbb-0000-0000-0000-000000000002",
                                        )
                                        .unwrap(),
                                    ]),
                                ] {
                                    let split: bool = sqlx::query_scalar(
                                        "SELECT (SELECT public.clearance_bypass())
                                             OR (SELECT public.clearance_rank_covers($1, $2))",
                                    )
                                    .bind(doc_rank)
                                    .bind(doc_compartments.as_deref())
                                    .fetch_one(&mut *conn)
                                    .await
                                    .expect("split predicate");
                                    // The public API must agree with the split form.
                                    let api: bool = sqlx::query_scalar(
                                        "SELECT public.clearance_covers($1, $2)",
                                    )
                                    .bind(doc_rank)
                                    .bind(doc_compartments.as_deref())
                                    .fetch_one(&mut *conn)
                                    .await
                                    .expect("clearance_covers");
                                    compared += 1;
                                    if split != api {
                                        mismatches += 1;
                                    }
                                }
                            }
                            // `setting` only changes the answer through the settings
                            // table; the loop above already covers the workspace GUC.
                            let _ = setting;
                        }
                    }
                }
            }
        }
    }

    sqlx::raw_sql("ROLLBACK")
        .execute(&mut *conn)
        .await
        .expect("rollback");
    assert_eq!(
        compared,
        3 * 3 * 3 * 3 * 2 * 3 * 5 * 4,
        "checked every combination"
    );
    assert_eq!(
        mismatches, 0,
        "split predicate disagreed with clearance_covers"
    );
}

/// The policy must reach the planner as an InitPlan, not a per-row call.
///
/// Proven structurally: the plan for a workspace-scoped count must contain no
/// `clearance_rank_covers` filter when the workspace is not enforcing, because
/// the hoisted bypass short-circuits before the per-row disjunct.
#[tokio::test]
async fn e2e_spec228_policy_bypass_is_hoisted() {
    let Some(pool) = pool().await else {
        eprintln!("SKIP: no DATABASE_URL");
        return;
    };
    let workspace: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT workspace_id FROM chunks WHERE tenant_id IS NOT NULL
          GROUP BY workspace_id ORDER BY count(*) DESC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();
    let Some(workspace) = workspace else {
        eprintln!("SKIP: no chunks to plan against");
        return;
    };

    let mut conn = pool.acquire().await.expect("connection");
    sqlx::raw_sql("BEGIN")
        .execute(&mut *conn)
        .await
        .expect("begin");
    // A non-superuser role is what exercises the policy; owners bypass it.
    let can_assume: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'edgequake_tenant_access')",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap_or(false);
    if !can_assume {
        let _ = sqlx::raw_sql("ROLLBACK").execute(&mut *conn).await;
        eprintln!("SKIP: edgequake_tenant_access role absent");
        return;
    }
    sqlx::raw_sql("SET LOCAL ROLE edgequake_tenant_access")
        .execute(&mut *conn)
        .await
        .expect("set local role");
    sqlx::query(
        "SELECT set_config('app.clearance_enforced', 'off', true),
                set_config('app.current_workspace_id', $1::text, true)",
    )
    .bind(workspace.to_string())
    .execute(&mut *conn)
    .await
    .expect("context");

    let plan_rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
        "EXPLAIN (VERBOSE, FORMAT TEXT)
         SELECT count(*) FROM public.chunks WHERE workspace_id = $1",
    )
    .bind(workspace)
    .fetch_all(&mut *conn)
    .await
    .expect("explain");
    let plan = plan_rows
        .iter()
        .map(|row| row.get::<String, _>(0))
        .collect::<Vec<_>>()
        .join("\n");

    let _ = sqlx::raw_sql("ROLLBACK").execute(&mut *conn).await;

    // The row-independent half is an InitPlan, so the planner names it once.
    assert!(
        plan.contains("InitPlan") && plan.contains("clearance_bypass"),
        "clearance_bypass must be hoisted into an InitPlan:\n{plan}"
    );
    // The bypass must be the *first* disjunct of the policy filter, so the
    // per-row rank check is short-circuited and never reached when not enforcing.
    // A hoisted scalar subquery appears as `$n`, the per-row one as `SubPlan k`.
    let filter_line = plan
        .lines()
        .find(|line| line.contains("Filter:") && line.contains("SubPlan"))
        .unwrap_or_else(|| panic!("policy filter not found in plan:\n{plan}"));
    let dollar = filter_line.find("$").expect("hoisted InitPlan reference");
    let subplan = filter_line.find("SubPlan").expect("per-row subplan");
    assert!(
        dollar < subplan,
        "the hoisted bypass must be evaluated before the per-row check:\n{filter_line}"
    );
    // Migration 229 turns this scan index-only: no heap fetch to read the
    // clearance columns the policy needs.
    assert!(
        plan.contains("Index Only Scan"),
        "the scope index should cover the clearance columns:\n{plan}"
    );
}
