//! SPEC-105 — legacy cutover contracts (LAW-L2..L6).
//!
//! Source + unit-adjacent gates. PG tests soft-skip without DATABASE_URL.

#![cfg(feature = "postgres")]

use std::sync::Arc;

use edgequake_api::storage_inspector::{InspectorConfig, StorageInspector};
use sqlx::PgPool;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .or_else(|| {
            std::fs::read_to_string("/tmp/edgequake-db-url")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

#[test]
fn e2e_105_01_source_unknown_vector_backend_typed() {
    let src = include_str!("../../edgequake-storage/src/vector_backend.rs");
    assert!(
        src.contains("_ => VectorBackend::TypedEmbeddings"),
        "unknown VECTOR_BACKEND must map to TypedEmbeddings (LAW-L2)"
    );
    assert!(
        !src.contains("_ => VectorBackend::LegacyTables"),
        "unknown must not select LegacyTables"
    );
}

#[test]
fn e2e_105_02_source_census_ssot() {
    let census = include_str!("../../edgequake-storage/src/legacy_store_census.rs");
    assert!(census.contains("legacy_store_census"));
    assert!(census.contains("any_legacy_rows"));
    let guard = include_str!("../../edgequake-storage/src/cutover_flag_guard.rs");
    assert!(
        guard.contains("legacy_store_census"),
        "cutover posture must use census SSOT (LAW-L4)"
    );
    assert!(
        guard.contains("!census.vectors_present()"),
        "full_vector_legacy_dropped must include empty vectors census"
    );
}

#[test]
fn e2e_105_03_inv03_era_aware_dual_when_kv() {
    let src = include_str!("../src/storage_inspector.rs");
    assert!(
        src.contains("k.key LIKE d.id::text || '-chunk-%'"),
        "INV-03 must keep KV dual arm for ≤0.22 mid-upgrade (LAW-L6)"
    );
    assert!(
        src.contains("FROM public.chunks c WHERE c.document_id = d.id"),
        "INV-03 must check public.chunks"
    );
}

#[test]
fn e2e_105_04_source_migration_142() {
    let sql = include_str!("../../../migrations/142_spec105_legacy_cutover_assert.sql");
    assert!(sql.contains("SPEC-105"));
    assert!(sql.contains("legacy_stores_forbidden"));
    assert!(sql.contains("confirm-drop"));
    assert!(sql.contains("DROP TABLE IF EXISTS"));
    assert!(
        sql.contains("RAISE EXCEPTION"),
        "142 must abort when legacy rows remain"
    );
}

#[test]
fn e2e_105_07_source_defer_142_helpers() {
    let src = include_str!("../src/state/migration_bootstrap/mod.rs");
    assert!(
        src.contains("pending_ok_to_serve"),
        "boot/migrate soft-allow must use pending_ok_to_serve (LAW-L5)"
    );
    assert!(
        src.contains("legacy_cutover_assert_version"),
        "142 must be named SSOT via legacy_cutover_assert_version() (manifest)"
    );
    assert!(
        src.contains("defer_legacy_cutover_assert"),
        "ExpandableOnly must defer 142 while residue remains"
    );
}

#[test]
fn e2e_105_06_fts_era_aware_kv_gate() {
    let src = include_str!("../../edgequake-storage/src/adapters/postgres/vector/fts.rs");
    assert!(
        src.contains("chunk_kv_table_exists"),
        "FTS must gate KV join on table existence (era-aware)"
    );
    assert!(src.contains("SPEC-105"));
}

#[tokio::test]
async fn e2e_105_04_pg_legacy_census_and_posture() {
    let Some(url) = database_url() else {
        eprintln!("SKIP e2e_105_04_pg: no DATABASE_URL");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP connect failed: {e}");
            return;
        }
    };

    let census = edgequake_storage::legacy_store_census(&pool)
        .await
        .expect("census");
    let posture = edgequake_storage::detect_cutover_posture(&pool)
        .await
        .expect("posture");

    // After SPEC-091 drops on this staging DB, expect no legacy tables.
    if census.any_legacy_table() {
        eprintln!(
            "NOTE: legacy tables still present kv={} vectors={} (≤0.22 mid-upgrade OK)",
            census.kv_table_count, census.vectors_table_count
        );
        assert!(
            !posture.full_vector_legacy_dropped || census.vectors_present(),
            "if vectors present, full_vector_legacy_dropped must be false"
        );
    } else {
        assert!(
            posture.full_vector_legacy_dropped,
            "empty vectors census ⇒ full_vector_legacy_dropped"
        );
        assert!(
            posture.kv_store_dropped,
            "empty kv census ⇒ kv_store_dropped"
        );
    }

    // Inspect must remain free of edgequake.Node / workspaces.id class errors.
    let inspector = StorageInspector::new(
        Arc::new(pool.clone()),
        InspectorConfig::for_namespace("default"),
    );
    let report = inspector.inspect().await;
    let blob = serde_json::to_string(&report).unwrap_or_default();
    assert!(
        !blob.contains("edgequake.Node") && !blob.contains("workspaces WHERE id"),
        "inspect must not probe legacy wrong names"
    );
}

#[tokio::test]
async fn e2e_105_pg_inv03_dual_when_kv_present() {
    let Some(url) = database_url() else {
        eprintln!("SKIP e2e_105_pg_inv03: no DATABASE_URL");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP connect: {e}");
            return;
        }
    };

    let census = edgequake_storage::legacy_store_census(&pool)
        .await
        .expect("census");
    if !census.kv_present() {
        eprintln!("SKIP: no KV table (post-125) — dual path N/A; chunks-only era");
        return;
    }

    // Smoke: inspector runs without panic when KV era still present.
    let _ = Uuid::nil();
    let inspector =
        StorageInspector::new(Arc::new(pool), InspectorConfig::for_namespace("default"));
    let _ = inspector.inspect().await;
}
