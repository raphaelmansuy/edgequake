//! Source-level inventory plus server Parse/Describe verification (no Bind/Execute).
//! Run strict: EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 cargo test -p edgequake-storage
//! --features postgres --test postgres_query_catalog -- --nocapture.
//! EQ_QUERY_AUDIT_OUTPUT writes all sites, including unresolved dynamic SQL.

#![cfg(feature = "postgres")]

#[path = "support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;
#[path = "support/query_catalog/mod.rs"]
mod query_catalog;

use sqlx::{Column, Executor, Statement, TypeInfo};
use std::time::Instant;

#[test]
fn inventory_excludes_comments_strings_and_test_code() {
    let sites = query_catalog::scan_source(
        "fixture.rs",
        r##"
        const SQL: &str = "SELECT $1::text";
        fn real() {
            // sqlx::query("invalid comment");
            let text = r#"sqlx::query("invalid string")"#;
            let sql = SQL;
            sqlx::query_scalar::<_, String>(&sql);
            sqlx::query(&format!("SELECT * FROM {table} WHERE id=$1"));
        }
        #[cfg(test)] mod tests { fn test() { sqlx::query("invalid test"); } }
    "##,
    );
    assert_eq!(sites.len(), 2);
    assert_eq!(sites[0].sql.as_deref(), Some("SELECT $1::text"));
    assert!(sites[1].sql.is_none());
    assert!(sites[1].template.as_deref().unwrap().contains("{table}"));
}

#[test]
fn inventory_does_not_resolve_mutable_or_shadowed_sql_as_static() {
    let sites = query_catalog::scan_source(
        "fixture.rs",
        r#"
        const sql: &str = "SELECT 'constant'";
        fn real() {
            let mut sql = "SELECT 1".to_string();
            sql.push_str(" WHERE false");
            sqlx::query(&sql);
            if a { let other = "SELECT 1"; sqlx::query(other); }
            else { let other = "SELECT 2"; sqlx::query(other); }
        }
    "#,
    );
    assert_eq!(sites.len(), 3);
    assert!(sites.iter().all(|s| s.sql.is_none()));
}

#[test]
fn inventory_counts_direct_binds_without_counting_methods_in_arguments() {
    let sites = query_catalog::scan_source(
        "fixture.rs",
        r#"
        async fn real() {
            sqlx::query("SELECT $1::text, $2::text").bind(a).bind(b).fetch_one(pool).await;
            sqlx::query("SELECT 1").execute(pool).await;
            let q = sqlx::query("SELECT $1::text");
            q.bind(a).execute(pool).await;
        }
    "#,
    );
    assert_eq!(sites.len(), 3);
    assert_eq!(sites[0].direct_bind_count, Some(2));
    assert_eq!(sites[1].direct_bind_count, Some(0));
    assert_eq!(
        sites[2].direct_bind_count, None,
        "variable builders require runtime evidence"
    );
}

#[tokio::test]
async fn postgres_prepares_source_derived_static_queries() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let sites = query_catalog::inventory(&root);
    assert!(sites.len() > 235, "source catalog unexpectedly shrank");
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let mut conn = pool.acquire().await.expect("audit connection");
    sqlx::query("LOAD 'age'")
        .execute(&mut *conn)
        .await
        .expect("AGE");
    sqlx::query("SET search_path = ag_catalog, public")
        .execute(&mut *conn)
        .await
        .expect("AGE path");
    sqlx::query("SET statement_timeout = '5s'")
        .execute(&mut *conn)
        .await
        .expect("prepare timeout");
    // Retired legacy ANN branches create this registry before using it. A
    // session-local fixture checks their SQL without resurrecting legacy data.
    sqlx::query("CREATE TEMP TABLE eq_hot_ann_workspaces (table_prefix text, workspace_id text, created_at timestamptz DEFAULT now(), PRIMARY KEY(table_prefix, workspace_id))")
        .execute(&mut *conn).await.expect("legacy ANN schema fixture");
    let mut results = Vec::new();
    let mut failures = Vec::new();
    let mut prepared = 0;
    for site in &sites {
        let start = Instant::now();
        let result = if let Some(sql) = &site.sql {
            match (&mut *conn).prepare(sql).await {
                Ok(statement) => {
                    prepared += 1;
                    let parameters = statement
                        .parameters()
                        .and_then(|p| p.left())
                        .expect("PostgreSQL parameter types");
                    let bind_count_ok =
                        site.direct_bind_count.is_none_or(|n| n == parameters.len());
                    if !bind_count_ok {
                        failures.push(format!(
                            "{}:{} {}: {} binds supplied for {} parameters",
                            site.file,
                            site.line,
                            site.function,
                            site.direct_bind_count.unwrap(),
                            parameters.len()
                        ));
                    }
                    serde_json::json!({"status": "prepared", "columns": statement.columns().iter()
                        .map(|c| serde_json::json!({"name": c.name(), "type": c.type_info().name()})).collect::<Vec<_>>(),
                        "parameter_types": parameters.iter().map(|p| p.name()).collect::<Vec<_>>(),
                        "direct_bind_count_verified": site.direct_bind_count.map(|_| bind_count_ok)})
                }
                Err(e) => {
                    failures.push(format!(
                        "{}:{} {}: {e}",
                        site.file, site.line, site.function
                    ));
                    serde_json::json!({"status": "prepare_failed", "error": e.to_string()})
                }
            }
        } else {
            serde_json::json!({"status": "dynamic_requires_execution_test"})
        };
        results.push(serde_json::json!({"site": site, "verification": result,
            "prepare_elapsed_ms": start.elapsed().as_secs_f64() * 1000.0,
            "execution_verified": false, "performance_measured": false}));
    }
    if let Ok(path) = std::env::var("EQ_QUERY_AUDIT_OUTPUT") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&results).expect("serialize catalog"),
        )
        .expect("write catalog");
    }
    eprintln!(
        "QUERY_CATALOG sites={} prepared={prepared} failed={} dynamic={}",
        sites.len(),
        failures.len(),
        sites.iter().filter(|s| s.sql.is_none()).count()
    );
    assert!(
        failures.is_empty(),
        "static SQL preparation failures:\n{}",
        failures.join("\n")
    );
}
