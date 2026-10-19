//! SPEC-150: HEAD schema train contracts (migrations 160–169 + fossils).
//!
//! These are compile-time / source / manifest contracts — no live Postgres
//! required. Runtime epoch proof remains `make spec150-matrix`.

use std::fs;
use std::path::PathBuf;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

fn numbered_sql_versions() -> Vec<i64> {
    let mut versions = Vec::new();
    for ent in fs::read_dir(migrations_dir()).expect("migrations dir") {
        let ent = ent.expect("dirent");
        let name = ent.file_name().to_string_lossy().into_owned();
        if let Some((num, rest)) = name.split_once('_') {
            if rest.ends_with(".sql") {
                if let Ok(v) = num.parse::<i64>() {
                    versions.push(v);
                }
            }
        }
    }
    versions.sort_unstable();
    versions
}

#[test]
fn head_train_includes_160_through_169() {
    let versions = numbered_sql_versions();
    let max = *versions.last().expect("at least one migration");
    assert_eq!(max, 212, "HEAD schema train must end at 212");
    for v in 160..=212 {
        assert!(
            versions.contains(&v),
            "missing migration {v} in edgequake/migrations/"
        );
    }
}

#[test]
fn manifest_compat_serve_max_matches_head() {
    let raw = include_str!("../../../migrations/manifest.toml");
    let max_line = raw
        .lines()
        .find(|l| l.starts_with("compat_serve_max"))
        .expect("compat_serve_max");
    let digits: String = max_line.chars().filter(|c| c.is_ascii_digit()).collect();
    assert_eq!(digits, "212");
}

#[test]
fn migration_172_is_expand_without_a_serving_floor() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v172 = manifest
        .split("version = 172")
        .nth(1)
        .expect("migration 172");
    assert!(v172.contains("phase = \"expand\""));
    assert!(
        !v172.contains("serving_floor"),
        "172 must not raise the serving floor"
    );
    let sql = include_str!("../../../migrations/172_request_path_lookup_indexes.sql");
    for index in [
        "idx_documents_metadata_track_id",
        "idx_cel_workspace_chunk_pattern",
        "idx_crl_workspace_chunk_pattern",
        "text_pattern_ops",
    ] {
        assert!(sql.contains(index), "missing {index}");
    }
    let lookup = include_str!("../src/services/task_document_sync.rs");
    assert!(lookup.contains("UNION ALL"));
    assert!(!lookup.contains("track_id = $1 OR metadata"));
    let tasks = include_str!("../../edgequake-tasks/src/postgres.rs");
    assert!(tasks.contains("p.created_at < t.created_at"));
    assert!(!tasks.contains("ROW_NUMBER()"));
}

#[test]
fn migration_173_indexes_injection_lists() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v173 = manifest
        .split("version = 173")
        .nth(1)
        .expect("migration 173");
    assert!(v173.contains("phase = \"expand\""));
    assert!(!v173.contains("serving_floor"));
    let sql = include_str!("../../../migrations/173_injection_list_index.sql");
    assert!(sql.contains("idx_documents_workspace_injection"));
    assert!(sql.contains("metadata->>'source_type'"));
    let list = include_str!("../src/services/injection_relational.rs");
    assert!(list.contains("metadata->>'source_type' = 'injection'"));
    assert!(list.contains("ORDER BY created_at DESC, id"));
}

#[test]
fn migration_174_indexes_pdf_lists() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v174 = manifest
        .split("version = 174")
        .nth(1)
        .expect("migration 174");
    assert!(v174.contains("phase = \"expand\""));
    assert!(!v174.contains("serving_floor"));
    let sql = include_str!("../../../migrations/174_pdf_workspace_created_index.sql");
    assert!(sql.contains("idx_pdf_documents_workspace_created"));
    assert!(sql.contains("(workspace_id, created_at DESC)"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/pdf_list_query.rs");
    assert!(list.contains("workspace_id = $"));
    assert!(list.contains("ORDER BY created_at DESC"));
}

#[test]
fn migration_175_indexes_task_lists() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v175 = manifest
        .split("version = 175")
        .nth(1)
        .expect("migration 175");
    assert!(v175.contains("phase = \"expand\""));
    assert!(!v175.contains("serving_floor"));
    let sql = include_str!("../../../migrations/175_task_workspace_created_index.sql");
    assert!(sql.contains("idx_tasks_tenant_workspace_created_track"));
    assert!(sql.contains("(tenant_id, workspace_id, created_at DESC, track_id DESC)"));
    let list = include_str!("../../edgequake-tasks/src/postgres.rs");
    assert!(list.contains("tenant_id = $"));
    assert!(list.contains("workspace_id = $"));
    assert!(list.contains("ORDER BY {sort_field} {sort_order}, track_id {sort_order}"));
}

#[test]
fn migration_176_indexes_conversation_sorts() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v176 = manifest
        .split("version = 176")
        .nth(1)
        .expect("migration 176");
    assert!(v176.contains("phase = \"expand\""));
    assert!(!v176.contains("serving_floor"));
    let sql = include_str!("../../../migrations/176_conversation_sort_indexes.sql");
    assert!(sql.contains("idx_conversations_tenant_user_created"));
    assert!(sql.contains("idx_conversations_tenant_user_title"));
    assert!(sql.contains("(tenant_id, user_id, created_at DESC)"));
    assert!(sql.contains("(tenant_id, user_id, title)"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("tenant_id = $1"));
    assert!(list.contains("user_id = $2"));
    assert!(list.contains("\"created_at\" => \"created_at\""));
    assert!(list.contains("\"title\" => \"title\""));
    assert!(list.contains("ORDER BY {sort_col} {dir}, conversation_id {dir}"));
    assert!(list.contains("({sort_col}, conversation_id) {cmp}"));
}

#[test]
fn migration_177_indexes_task_status_and_type_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v177 = manifest
        .split("version = 177")
        .nth(1)
        .expect("migration 177");
    assert!(v177.contains("phase = \"expand\""));
    assert!(!v177.contains("serving_floor"));
    let sql = include_str!("../../../migrations/177_task_status_type_created_indexes.sql");
    assert!(sql.contains("idx_tasks_tenant_workspace_status_created_track"));
    assert!(sql.contains("idx_tasks_tenant_workspace_type_created_track"));
    assert!(sql.contains("(tenant_id, workspace_id, status, created_at DESC, track_id DESC)"));
    assert!(sql.contains("(tenant_id, workspace_id, task_type, created_at DESC, track_id DESC)"));
    let list = include_str!("../../edgequake-tasks/src/postgres.rs");
    assert!(list.contains("status = $"));
    assert!(list.contains("task_type = $"));
    assert!(list.contains("ORDER BY {sort_field} {sort_order}, track_id {sort_order}"));
}

#[test]
fn migration_178_indexes_task_updated_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v178 = manifest
        .split("version = 178")
        .nth(1)
        .expect("migration 178");
    assert!(v178.contains("phase = \"expand\""));
    assert!(!v178.contains("serving_floor"));
    let sql = include_str!("../../../migrations/178_task_workspace_updated_index.sql");
    assert!(sql.contains("idx_tasks_tenant_workspace_updated_track"));
    assert!(sql.contains("(tenant_id, workspace_id, updated_at DESC, track_id DESC)"));
    let list = include_str!("../../edgequake-tasks/src/postgres.rs");
    assert!(list.contains("SortField::UpdatedAt => \"updated_at\""));
    assert!(list.contains("ORDER BY {sort_field} {sort_order}"));
}

#[test]
fn migration_192_indexes_scoped_drain_claims() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v192 = manifest
        .split("version = 192")
        .nth(1)
        .expect("migration 192");
    assert!(v192.contains("phase = \"expand\""));
    assert!(!v192.contains("serving_floor"));
    let sql = include_str!("../../../migrations/192_drain_workspace_claim_index.sql");
    assert!(sql.contains("idx_outbox_events_workspace_unprocessed"));
    assert!(sql.contains("(workspace_id, available_at, created_at)"));
    assert!(sql.contains("idx_compensation_quarantine_due"));
    assert!(sql.contains("idx_compensation_quarantine_workspace_next"));
    let outbox = include_str!("../../edgequake-storage/src/outbox_drain.rs");
    assert!(outbox.contains("OUTBOX_CLAIM_SCOPED"));
    assert!(outbox.contains("workspace_id = $2"));
    assert!(!outbox.contains("$2::uuid IS NULL OR workspace_id = $2"));
    let quarantine = include_str!("../../edgequake-storage/src/compensation_drain.rs");
    assert!(quarantine.contains("QUARANTINE_CLAIM_OPEN"));
    assert!(quarantine.contains("QUARANTINE_CLAIM_SCOPED"));
    assert!(!quarantine.contains("$2::uuid IS NULL OR workspace_id = $2"));
}

#[test]
fn migration_193_indexes_scoped_projection_claims() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v193 = manifest
        .split("version = 193")
        .nth(1)
        .expect("migration 193");
    assert!(v193.contains("phase = \"expand\""));
    assert!(!v193.contains("serving_floor"));
    let sql = include_str!("../../../migrations/193_projection_binding_due_index.sql");
    assert!(sql.contains("idx_projection_deliveries_binding_due"));
    assert!(sql.contains("(binding_id, next_attempt_at, event_id)"));
    let ledger = include_str!("../../edgequake-storage/src/projection/ledger.rs");
    assert!(ledger.contains("CLAIM_SQL_OPEN"));
    assert!(
        ledger.contains("idx_projection_deliveries_binding_due")
            || ledger.contains("b.provider = $4")
    );
    assert!(!ledger.contains("$4::text IS NULL OR EXISTS"));
    let connections = include_str!("../src/handlers/connections.rs");
    assert!(connections.contains("fn connection_list_sql"));
    assert!(!connections.contains("$1::uuid IS NULL OR tenant_id IS NULL OR tenant_id = $1"));
    let shell = include_str!("../../edgequake-storage/src/adapters/postgres/document_shell.rs");
    assert!(shell.contains("ORDER BY documents.id"));
    assert!(!shell.contains("$1::uuid IS NULL OR id > $1"));
    let revisions =
        include_str!("../../edgequake-storage/src/adapters/postgres/ingestion_committer.rs");
    assert!(revisions.contains("DOCUMENT_REVISION_PAGE_AFTER"));
    assert!(!revisions.contains("$3::uuid IS NULL OR logical_id > $3"));
}

#[test]
fn migration_194_indexes_runnable_task_claims() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v194 = manifest
        .split("version = 194")
        .nth(1)
        .expect("migration 194");
    assert!(v194.contains("phase = \"expand\""));
    assert!(!v194.contains("serving_floor"));
    let sql = include_str!("../../../migrations/194_task_claim_runnable_indexes.sql");
    for index in [
        "idx_tasks_pending_unheld_created",
        "idx_tasks_pending_unheld_workspace",
        "idx_tasks_processing_unleased_created",
        "idx_tasks_processing_unleased_workspace",
        "idx_tasks_processing_expired_unheld",
        "idx_tasks_processing_expired_unheld_workspace",
    ] {
        assert!(sql.contains(index), "missing {index}");
    }
    let tasks = include_str!("../../edgequake-tasks/src/postgres.rs");
    assert!(tasks.contains("FAIR_PENDING_SAMPLE_SQL"));
    assert!(tasks.contains("FAIR_STALE_SAMPLE_SQL"));
    assert!(tasks.contains("JOIN LATERAL"));
    assert!(!tasks.contains("fairness_hold_until IS NULL OR fairness_hold_until"));
    assert!(!tasks.contains("lease_expires_at IS NULL OR lease_expires_at"));
}

#[test]
fn migration_195_indexes_entity_source_ids() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v195 = manifest
        .split("version = 195")
        .nth(1)
        .expect("migration 195");
    assert!(v195.contains("phase = \"expand\""));
    assert!(!v195.contains("serving_floor"));
    let sql = include_str!("../../../migrations/195_entity_source_ids_gin.sql");
    assert!(sql.contains("idx_entities_source_ids"));
    assert!(sql.contains("USING gin (source_ids)"));
    let fleet =
        include_str!("../../edgequake-storage/src/adapters/postgres/fleet_embedding_index.rs");
    assert!(fleet.contains("entity_search_sql_in_ids"));
    assert!(fleet.contains("OFFSET 0"));
    assert!(fleet.contains("e.name = wanted.name"));
    assert!(fleet.contains("apply_entity_source"));
    assert!(fleet.contains("document_lineage_sql::entity_required"));
    assert!(!fleet.contains("e.id::text = ANY($7)"));
}

#[test]
fn migration_196_indexes_conversation_page_tiebreak() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v196 = manifest
        .split("version = 196")
        .nth(1)
        .expect("migration 196");
    assert!(v196.contains("phase = \"expand\""));
    assert!(!v196.contains("serving_floor"));
    let sql = include_str!("../../../migrations/196_conversation_page_tiebreak.sql");
    assert!(sql.contains("idx_conversations_tenant_user_updated_id"));
    assert!(sql.contains("(tenant_id, user_id, updated_at DESC, conversation_id DESC)"));
    assert!(sql.contains("idx_conversations_tenant_user_unfiled_updated_id"));
    assert!(sql.contains("WHERE folder_id IS NULL"));
    assert!(sql.contains("idx_messages_conversation_created_id"));
    assert!(sql.contains("(conversation_id, created_at, message_id)"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("ORDER BY {sort_col} {dir}, conversation_id {dir}"));
    assert!(list.contains("({sort_col}, conversation_id) {cmp}"));
    assert!(list.contains("(created_at, message_id) > ($2, $3)"));
    assert!(list.contains("ORDER BY created_at ASC, message_id ASC"));
}

#[test]
fn migration_197_indexes_pdf_page_tiebreak() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v197 = manifest
        .split("version = 197")
        .nth(1)
        .expect("migration 197");
    assert!(v197.contains("phase = \"expand\""));
    assert!(!v197.contains("serving_floor"));
    let sql = include_str!("../../../migrations/197_pdf_page_tiebreak.sql");
    assert!(sql.contains("idx_pdf_documents_workspace_created_id"));
    assert!(sql.contains("(workspace_id, created_at DESC, pdf_id DESC)"));
    assert!(sql.contains("idx_pdf_documents_workspace_status_created_id"));
    assert!(sql.contains("(workspace_id, processing_status, created_at DESC, pdf_id DESC)"));
    assert!(sql.contains("idx_pdf_documents_created_id"));
    assert!(sql.contains("idx_pdf_documents_status_created_id"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/pdf_list_query.rs");
    assert!(list.contains("created_at <= ${at}"));
    assert!(list.contains("(created_at, pdf_id) < (${at}, ${id})"));
    assert!(list.contains("ORDER BY created_at DESC, pdf_id DESC"));
    assert!(list.contains("OFFSET ${}"));
}

#[test]
fn migration_198_indexes_entity_modality() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v198 = manifest
        .split("version = 198")
        .nth(1)
        .expect("migration 198");
    assert!(v198.contains("phase = \"expand\""));
    assert!(!v198.contains("serving_floor"));
    let sql = include_str!("../../../migrations/198_entity_modality_index.sql");
    assert!(sql.contains("idx_entities_modality"));
    assert!(sql.contains("(metadata->>'modality')"));
    assert!(sql.contains("WHERE (metadata->>'modality') IS NOT NULL"));
    let search =
        include_str!("../../edgequake-storage/src/adapters/postgres/fleet_embedding_index.rs");
    assert!(search.contains("AND e.modality = ANY($6)"));
    assert!(search.contains("$6::text[] IS NULL OR e.modality = ANY($6)"));
}

#[test]
fn migration_199_covers_shared_entity_counts() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v199 = manifest
        .split("version = 199")
        .nth(1)
        .expect("migration 199");
    assert!(v199.contains("phase = \"expand\""));
    assert!(!v199.contains("serving_floor"));
    let sql = include_str!("../../../migrations/199_chunk_entity_shared_index.sql");
    assert!(sql.contains("idx_cel_entity_workspace_chunk"));
    assert!(sql.contains("(entity_name, workspace_id, chunk_id)"));
    let lineage = include_str!("../src/services/postgres_chunk_lineage.rs");
    assert!(lineage.contains("entity_name = ANY($2)"));
    assert!(lineage.contains("COUNT(DISTINCT split_part(chunk_id, '-chunk-', 1)) > 1"));
}

#[test]
fn migration_200_indexes_document_status_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v200 = manifest
        .split("version = 200")
        .nth(1)
        .expect("migration 200");
    assert!(v200.contains("phase = \"expand\""));
    assert!(!v200.contains("serving_floor"));
    let sql = include_str!("../../../migrations/200_document_status_page_index.sql");
    assert!(sql.contains("idx_documents_workspace_status_created"));
    assert!(sql.contains("(workspace_id, status, created_at DESC, id DESC)"));
    assert!(sql.contains("idx_documents_workspace_tenant_status_created"));
    assert!(sql.contains("idx_documents_workspace_null_tenant_status_created"));
    assert!(sql.contains("WHERE tenant_id IS NULL"));
    let list = include_str!("../src/document_read_model.rs");
    assert!(list.contains("status = $2"));
    assert!(list.contains("fn relational_list_filter"));
    assert!(list.contains("ORDER BY created_at DESC, id DESC"));
}

#[test]
fn migration_201_indexes_document_stage_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v201 = manifest
        .split("version = 201")
        .nth(1)
        .expect("migration 201");
    assert!(v201.contains("phase = \"expand\""));
    assert!(!v201.contains("serving_floor"));
    let sql = include_str!("../../../migrations/201_document_stage_page_index.sql");
    assert!(sql.contains("idx_documents_workspace_stage_created"));
    assert!(sql.contains("(workspace_id, (metadata->>'current_stage'), created_at DESC, id DESC)"));
    assert!(sql.contains("idx_documents_workspace_tenant_stage_created"));
    assert!(sql.contains("idx_documents_workspace_null_tenant_stage_created"));
    assert!(sql.contains("WHERE tenant_id IS NULL"));
    let list = include_str!("../src/document_read_model.rs");
    assert!(list.contains("fn chip_list_statement"));
    assert!(list.contains("current_stage ="));
    assert!(list.contains("status IS DISTINCT FROM"));
    assert!(!list.contains("(metadata->>'current_stage') ="));
}

#[test]
fn migration_205_drops_empty_ag_catalog_after_cutover() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v205 = manifest
        .split("version = 205")
        .nth(1)
        .expect("migration 205");
    assert!(v205.contains("phase = \"expand\""));
    assert!(!v205.contains("serving_floor"));
    let sql = include_str!("../../../migrations/205_drop_ag_catalog_leftovers.sql");
    assert!(sql.contains("DROP SCHEMA ag_catalog CASCADE"));
    assert!(sql.contains("age_absent IS TRUE"));
    assert!(sql.contains("refusing to drop ag_catalog"));
    assert!(sql.contains("is not empty"));
    assert!(sql.contains("extname = 'age'"));
    assert!(
        !sql.contains("Apache AGE is installed"),
        "205 must not fail the train while the extension is still installed"
    );
}

#[test]
fn migration_206_stores_leakproof_predicates() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v206 = manifest
        .split("version = 206")
        .nth(1)
        .expect("migration 206");
    assert!(v206.contains("phase = \"expand\""));
    assert!(!v206.contains("serving_floor"));
    let sql = include_str!("../../../migrations/206_rls_leakproof_predicates.sql");
    assert!(sql.contains("idx_chunks_tenant_modality"));
    assert!(sql.contains("idx_entities_tenant_modality"));
    assert!(sql.contains("idx_documents_ws_tenant_stage_col"));
    assert!(sql.contains("eq_graph_search_nodes"));
    assert!(sql.contains("SECURITY DEFINER"));
    let search =
        include_str!("../../edgequake-storage/src/adapters/postgres/graph/plain/search.rs");
    assert!(search.contains("eq_graph_search_nodes"));
    let list = include_str!("../src/document_read_model.rs");
    assert!(list.contains("current_stage ="));
}

#[test]
fn migration_207_looks_up_age_lineage_as_bypass_owner() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v207 = manifest
        .split("version = 207")
        .nth(1)
        .expect("migration 207");
    assert!(v207.contains("phase = \"expand\""));
    assert!(!v207.contains("serving_floor"));
    let sql = include_str!("../../../migrations/207_age_lineage_discovery_rls.sql");
    assert!(sql.contains("eq_age_lineage_discover_nodes"));
    assert!(sql.contains("eq_age_lineage_discover_edges_singular"));
    assert!(sql.contains("SECURITY DEFINER"));
    assert!(sql.contains("OFFSET 0"));
    assert!(sql.contains("edgequake_tenant_access"));
    let scan = include_str!("../../edgequake-storage/src/adapters/postgres/graph/scan_ops.rs");
    assert!(scan.contains("eq_age_lineage_discover_nodes"));
    assert!(scan.contains("eq_age_lineage_discover_edges"));
    assert!(scan.contains("eq_age_lineage_discover_edges_singular"));
}

#[test]
fn migration_202_indexes_document_title_search() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v202 = manifest
        .split("version = 202")
        .nth(1)
        .expect("migration 202");
    assert!(v202.contains("phase = \"expand\""));
    assert!(!v202.contains("serving_floor"));
    let sql = include_str!("../../../migrations/202_document_title_trgm.sql");
    assert!(sql.contains("idx_documents_title_trgm"));
    assert!(sql.contains("lower(title) public.gin_trgm_ops"));
    let list = include_str!("../src/document_read_model.rs");
    assert!(list.contains("fn document_title_search_statement"));
    assert!(list.contains("lower(title) LIKE"));
    assert!(list.contains("force_custom_plan"));
    assert!(list.contains("fn search_relational_document_titles"));
}

#[test]
fn migration_191_indexes_document_list_by_tenant() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v191 = manifest
        .split("version = 191")
        .nth(1)
        .expect("migration 191");
    assert!(v191.contains("phase = \"expand\""));
    assert!(!v191.contains("serving_floor"));
    let sql = include_str!("../../../migrations/191_document_tenant_created_index.sql");
    assert!(sql.contains("idx_documents_workspace_tenant_created"));
    assert!(sql.contains("(workspace_id, tenant_id, created_at DESC, id DESC)"));
    assert!(sql.contains("idx_documents_workspace_null_tenant_created"));
    assert!(sql.contains("WHERE tenant_id IS NULL"));
    let list = include_str!("../src/document_read_model.rs");
    assert!(list.contains("fn document_list_statement"));
    assert!(list.contains("tenant_id = $2"));
    assert!(list.contains("tenant_id IS NULL"));
    assert!(list.contains("ORDER BY created_at DESC, id DESC"));
    assert!(list.contains("fn document_status_statement"));
    assert!(list.contains("force_custom_plan"));
    assert!(
        !list.contains("$2::uuid IS NULL OR tenant_id IS NULL OR tenant_id = $2"),
        "list, status, and workspace delete must require the tenant predicate"
    );
}

#[test]
fn migration_190_indexes_chunk_search_by_legacy_key() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v190 = manifest
        .split("version = 190")
        .nth(1)
        .expect("migration 190");
    assert!(v190.contains("phase = \"expand\""));
    assert!(!v190.contains("serving_floor"));
    let sql = include_str!("../../../migrations/190_chunk_legacy_key_index.sql");
    assert!(sql.contains("idx_chunks_legacy_chunk_key"));
    assert!(sql.contains("(metadata->>'legacy_chunk_key') IS NOT NULL"));
    let fts = include_str!("../../edgequake-storage/src/adapters/postgres/vector/fts.rs");
    assert!(fts.contains("TYPED_CHUNKS_FTS_SQL_IN_IDS"));
    assert!(fts.contains("OFFSET 0"));
    assert!(fts.contains("id_filter.is_some()"));
}

#[test]
fn migration_189_indexes_chunk_search_by_modality() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v189 = manifest
        .split("version = 189")
        .nth(1)
        .expect("migration 189");
    assert!(v189.contains("phase = \"expand\""));
    assert!(!v189.contains("serving_floor"));
    let sql = include_str!("../../../migrations/189_chunk_modality_index.sql");
    assert!(sql.contains("idx_chunks_modality"));
    assert!(sql.contains("(metadata->>'modality') IS NOT NULL"));
    let fts = include_str!("../../edgequake-storage/src/adapters/postgres/vector/fts.rs");
    assert!(fts.contains("modalities.is_some()"));
    assert!(fts.contains("AND c.modality = ANY($6)"));
}

#[test]
fn migration_188_indexes_conversation_title_search_by_pin_and_archive() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v188 = manifest
        .split("version = 188")
        .nth(1)
        .expect("migration 188");
    assert!(v188.contains("phase = \"expand\""));
    assert!(!v188.contains("serving_floor"));
    let sql = include_str!("../../../migrations/188_conversation_flag_title_fts.sql");
    assert!(sql.contains("idx_conversations_tenant_user_flags_title_fts"));
    assert!(sql.contains("is_pinned public.bool_ops"));
    assert!(sql.contains("is_archived public.bool_ops"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("is_pinned = ${}"));
    assert!(list.contains("is_archived = ${}"));
    assert!(list.contains("to_tsvector('english', title) @@ plainto_tsquery('english'"));
}

#[test]
fn migration_187_indexes_conversation_title_search_in_a_folder() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v187 = manifest
        .split("version = 187")
        .nth(1)
        .expect("migration 187");
    assert!(v187.contains("phase = \"expand\""));
    assert!(!v187.contains("serving_floor"));
    let sql = include_str!("../../../migrations/187_conversation_folder_title_fts.sql");
    assert!(sql.contains("idx_conversations_tenant_user_folder_title_fts"));
    assert!(sql.contains("folder_id public.uuid_ops"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("folder_id = ${}"));
    assert!(list.contains("to_tsvector('english', title) @@ plainto_tsquery('english'"));
}

#[test]
fn migration_186_indexes_conversation_title_search_by_user() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v186 = manifest
        .split("version = 186")
        .nth(1)
        .expect("migration 186");
    assert!(v186.contains("phase = \"expand\""));
    assert!(!v186.contains("serving_floor"));
    let sql = include_str!("../../../migrations/186_conversation_user_title_fts.sql");
    assert!(sql.contains("btree_gin"));
    assert!(sql.contains("WITH SCHEMA public"));
    assert!(sql.contains("idx_conversations_tenant_user_title_fts"));
    assert!(sql.contains("public.uuid_ops"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("to_tsvector('english', title) @@ plainto_tsquery('english'"));
    assert!(
        list.contains("force_custom_plan"),
        "a common title must replan so the user index stops at the page"
    );
}

#[test]
fn migration_185_indexes_workspace_chunk_search() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v185 = manifest
        .split("version = 185")
        .nth(1)
        .expect("migration 185");
    assert!(v185.contains("phase = \"expand\""));
    assert!(!v185.contains("serving_floor"));
    let sql = include_str!("../../../migrations/185_chunk_workspace_fts_index.sql");
    assert!(sql.contains("idx_chunks_workspace"));
    assert!(sql.contains("ON public.chunks (workspace_id)"));
    let fts = include_str!("../../edgequake-storage/src/adapters/postgres/vector/fts.rs");
    assert!(fts.contains("TYPED_CHUNKS_FTS_SQL_IN_WORKSPACE"));
    assert!(fts.contains("c.workspace_id = $2 OR c.workspace_id IS NULL"));
    assert!(fts.contains("workspace_uuid.is_some()"));
    assert!(
        fts.contains("force_custom_plan"),
        "a common term must replan so the document join stops at the page"
    );
}

#[test]
fn migration_184_indexes_task_status_and_type_together() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v184 = manifest
        .split("version = 184")
        .nth(1)
        .expect("migration 184");
    assert!(v184.contains("phase = \"expand\""));
    assert!(!v184.contains("serving_floor"));
    let sql = include_str!("../../../migrations/184_task_status_type_pair_indexes.sql");
    assert!(sql.contains("idx_tasks_tenant_workspace_status_type_created_track"));
    assert!(sql.contains("idx_tasks_tenant_workspace_status_type_updated"));
    assert!(sql
        .contains("(tenant_id, workspace_id, status, task_type, created_at DESC, track_id DESC)"));
    assert!(sql.contains("(tenant_id, workspace_id, status, task_type, updated_at DESC)"));
    let list = include_str!("../../edgequake-tasks/src/postgres.rs");
    assert!(list.contains("status = $"));
    assert!(list.contains("task_type = $"));
    assert!(list.contains("ORDER BY {sort_field} {sort_order}, track_id {sort_order}"));
    assert!(list.contains("SortField::UpdatedAt => \"updated_at\""));
}

#[test]
fn migration_183_indexes_conversation_filter_sort_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v183 = manifest
        .split("version = 183")
        .nth(1)
        .expect("migration 183");
    assert!(v183.contains("phase = \"expand\""));
    assert!(!v183.contains("serving_floor"));
    let sql = include_str!("../../../migrations/183_conversation_filter_sort_indexes.sql");
    for index in [
        "idx_conversations_tenant_user_folder_created",
        "idx_conversations_tenant_user_folder_title",
        "idx_conversations_tenant_user_pinned_created",
        "idx_conversations_tenant_user_pinned_title",
        "idx_conversations_tenant_user_unfiled_created",
        "idx_conversations_tenant_user_unfiled_title",
        "idx_conversations_tenant_user_archived_created",
        "idx_conversations_tenant_user_archived_title",
    ] {
        assert!(sql.contains(index), "missing {index}");
    }
    assert!(sql.contains("(tenant_id, user_id, folder_id, created_at DESC)"));
    assert!(sql.contains("(tenant_id, user_id, folder_id, title)"));
    assert!(sql.contains("(tenant_id, user_id, is_pinned, created_at DESC)"));
    assert!(sql.contains("(tenant_id, user_id, is_pinned, title)"));
    assert!(sql.contains("(tenant_id, user_id, created_at DESC)"));
    assert!(sql.contains("WHERE folder_id IS NULL"));
    assert!(sql.contains("(tenant_id, user_id, is_archived, created_at DESC)"));
    assert!(sql.contains("(tenant_id, user_id, is_archived, title)"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("folder_id = $"));
    assert!(list.contains("is_pinned = $"));
    assert!(list.contains("folder_id IS NULL"));
    assert!(list.contains("is_archived = $"));
    assert!(list.contains("\"created_at\" => \"created_at\""));
    assert!(list.contains("\"title\" => \"title\""));
    assert!(list.contains("ORDER BY {sort_col} {dir}, conversation_id {dir}"));
}

#[test]
fn migration_182_indexes_task_status_and_type_updated_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v182 = manifest
        .split("version = 182")
        .nth(1)
        .expect("migration 182");
    assert!(v182.contains("phase = \"expand\""));
    assert!(!v182.contains("serving_floor"));
    let sql = include_str!("../../../migrations/182_task_status_type_updated_indexes.sql");
    assert!(sql.contains("idx_tasks_tenant_workspace_status_updated"));
    assert!(sql.contains("idx_tasks_tenant_workspace_type_updated"));
    assert!(sql.contains("(tenant_id, workspace_id, status, updated_at DESC)"));
    assert!(sql.contains("(tenant_id, workspace_id, task_type, updated_at DESC)"));
    let list = include_str!("../../edgequake-tasks/src/postgres.rs");
    assert!(list.contains("status = $"));
    assert!(list.contains("task_type = $"));
    assert!(list.contains("SortField::UpdatedAt => \"updated_at\""));
}

#[test]
fn migration_181_indexes_conversation_unfiled_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v181 = manifest
        .split("version = 181")
        .nth(1)
        .expect("migration 181");
    assert!(v181.contains("phase = \"expand\""));
    assert!(!v181.contains("serving_floor"));
    let sql = include_str!("../../../migrations/181_conversation_unfiled_updated_index.sql");
    assert!(sql.contains("idx_conversations_tenant_user_unfiled_updated"));
    assert!(sql.contains("(tenant_id, user_id, updated_at DESC)"));
    assert!(sql.contains("WHERE folder_id IS NULL"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("folder_id IS NULL"));
    assert!(list.contains("_ => \"updated_at\""));
}

#[test]
fn migration_180_indexes_conversation_pinned_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v180 = manifest
        .split("version = 180")
        .nth(1)
        .expect("migration 180");
    assert!(v180.contains("phase = \"expand\""));
    assert!(!v180.contains("serving_floor"));
    let sql = include_str!("../../../migrations/180_conversation_pinned_updated_index.sql");
    assert!(sql.contains("idx_conversations_tenant_user_pinned_updated"));
    assert!(sql.contains("(tenant_id, user_id, is_pinned, updated_at DESC)"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("is_pinned = $"));
    assert!(list.contains("_ => \"updated_at\""));
}

#[test]
fn migration_179_indexes_conversation_folder_pages() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v179 = manifest
        .split("version = 179")
        .nth(1)
        .expect("migration 179");
    assert!(v179.contains("phase = \"expand\""));
    assert!(!v179.contains("serving_floor"));
    let sql = include_str!("../../../migrations/179_conversation_folder_updated_index.sql");
    assert!(sql.contains("idx_conversations_tenant_user_folder_updated"));
    assert!(sql.contains("(tenant_id, user_id, folder_id, updated_at DESC)"));
    let list = include_str!("../../edgequake-storage/src/adapters/postgres/conversation.rs");
    assert!(list.contains("folder_id = $"));
    assert!(list.contains("_ => \"updated_at\""));
}

#[test]
fn fossils_001_and_019_are_registered() {
    let m = edgequake_migrate_manifest::load();
    let m001 = m.migration.iter().find(|e| e.version == 1).expect("m001");
    let m019 = m.migration.iter().find(|e| e.version == 19).expect("m019");
    assert!(
        !m001.fossils.is_empty(),
        "migration 001 must list the v0.11.0 fossil (issue #195)"
    );
    assert!(
        m001.fossils.iter().any(|f| !f.dev_only),
        "001 fossil must be production-accept"
    );
    assert!(
        !m019.fossils.is_empty(),
        "migration 019 must list the v0.10.6–v0.10.12 fossil"
    );
    assert!(
        m019.fossils.iter().any(|f| !f.dev_only),
        "019 fossil must be production-accept"
    );
}

#[test]
fn apply_uses_sequential_progress_path() {
    let src = include_str!("../src/state/migration_bootstrap/apply.rs");
    assert!(
        src.contains("apply_pending_sequentially"),
        "CLI must apply migrations one-by-one for operator progress"
    );
    assert!(
        src.contains("print_upgrade_path"),
        "CLI must print UPGRADE PATH before apply"
    );
    assert!(
        src.contains("record_migration_step"),
        "each step must record migration_run_step telemetry when table exists"
    );
}

#[test]
fn progress_module_release_map_covers_head() {
    // Source contract (no `postgres` feature required for this integration test).
    let src = include_str!("../src/state/migration_bootstrap/progress.rs");
    assert!(
        src.contains("167..=168 => \"v0.32.0–v0.32.2\""),
        "release_for_schema must map 167–168 to v0.32.x"
    );
    assert!(
        src.contains("169 => \"v0.33.0\""),
        "release_for_schema must map 169 to v0.33.0"
    );
    assert!(
        src.contains("170..=206 => \"v0.35.0\""),
        "release_for_schema must map 170–206 to v0.35.0"
    );
    assert!(
        src.contains("207 => \"v0.35.1\""),
        "release_for_schema must map 207 to v0.35.1"
    );
    assert!(
        src.contains("149 => \"v0.26.0–v0.26.10\""),
        "release_for_schema must map 149 to v0.26.x"
    );
    assert!(
        src.contains("150..=159 => \"v0.27.0\""),
        "release_for_schema must map 150–159 to v0.27.0"
    );
}

#[test]
fn sequential_apply_is_restartable_by_design() {
    // Idempotent re-run / interrupted-retry: each step is its own sqlx transaction
    // + ledger row. Re-running `edgequake migrate` after a crash resumes at the
    // first unapplied version (sqlx skips applied checksums).
    let src = include_str!("../src/state/migration_bootstrap/apply.rs");
    assert!(
        src.contains("one.run_direct(&mut *conn)"),
        "each step must run on one checkout whose search_path is public"
    );
    assert!(
        src.contains("SET search_path TO public"),
        "sqlx writes unqualified _sqlx_migrations; the checkout must resolve that name to public"
    );
    assert!(
        src.contains("fold_stray_sqlx_ledgers"),
        "rows already recorded outside public must be folded back before pending detection"
    );
    assert!(src.contains("locking: false"));
    assert!(
        src.contains("finish_migration_run(pool, run_id, \"error\""),
        "failed step must close migration_run as error so operators can resume"
    );
}

#[test]
fn migration_208_wraps_session_helpers_as_initplans() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v208 = manifest
        .split("version = 208")
        .nth(1)
        .and_then(|rest| rest.split("[[migration]]").next())
        .expect("migration 208");
    assert!(v208.contains("phase = \"expand\""));
    assert!(!v208.contains("serving_floor"));
    let sql = include_str!("../../../migrations/208_rls_initplan_predicates.sql");
    assert!(sql.contains("ALTER POLICY"), "rewrites policies in place");
    assert!(
        sql.contains("RAISE EXCEPTION"),
        "each rewrite must be verified against the original expression"
    );
    assert!(
        !sql.contains("DROP POLICY") && !sql.contains("CREATE POLICY"),
        "roles, commands and RESTRICTIVE must stay untouched"
    );
}

#[test]
fn migration_209_extends_graph_scope_indexes_with_the_sort_key() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v209 = manifest
        .split("version = 209")
        .nth(1)
        .expect("migration 209");
    assert!(v209.contains("phase = \"expand\""));
    assert!(!v209.contains("serving_floor"));
    let sql = include_str!("../../../migrations/209_graph_ordered_scope_indexes.sql");
    assert!(sql.contains("(namespace, tenant_id, workspace_id, node_id)"));
    assert!(sql.contains("(namespace, tenant_id, workspace_id, source_id, target_id, rel_type)"));
    // The old scope indexes are strict prefixes of the new ones.
    assert!(sql.contains("DROP INDEX IF EXISTS public.idx_eq_graph_nodes_tenant_ws"));
    assert!(sql.contains("DROP INDEX IF EXISTS public.idx_eq_graph_edges_tenant_ws"));
}

#[test]
fn migration_210_runs_typo_label_search_as_owner_with_session_scope() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v210 = manifest
        .split("version = 210")
        .nth(1)
        .expect("migration 210");
    assert!(v210.contains("phase = \"expand\""));
    assert!(!v210.contains("serving_floor"));
    let sql = include_str!("../../../migrations/210_graph_label_typo_search_rls.sql");
    assert!(sql.contains("SECURITY DEFINER"));
    assert!(sql.contains("app.current_tenant_id"));
    assert!(sql.contains("REVOKE ALL ON FUNCTION public.eq_graph_search_labels_fuzzy"));
    assert!(sql.contains("TO edgequake_tenant_access"));
    let search =
        include_str!("../../edgequake-storage/src/adapters/postgres/graph/plain/search.rs");
    assert!(
        search.contains("eq_graph_search_labels_fuzzy"),
        "the adapter must call the owner function, not run `%` as the tenant role"
    );
    assert!(!search.contains("lower(n.node_id) % $4"));
}

#[test]
fn migrations_211_212_give_relationships_their_own_chunk_keys() {
    let manifest = include_str!("../../../migrations/manifest.toml");
    let v211 = manifest
        .split("version = 211")
        .nth(1)
        .expect("migration 211");
    assert!(v211.contains("phase = \"expand\""));
    assert!(!v211.contains("serving_floor"));
    let ddl = include_str!("../../../migrations/211_relationship_source_chunk_keys.sql");
    assert!(ddl.contains("ADD COLUMN IF NOT EXISTS source_chunk_keys text[]"));
    assert!(ddl.contains("USING gin (source_chunk_keys)"));

    let v212 = manifest
        .split("version = 212")
        .nth(1)
        .expect("migration 212");
    assert!(v212.contains("phase = \"data\""));
    assert!(v212.contains("lock_class = \"dml_batch\""));
    let fill = include_str!("../../../migrations/212_relationship_source_chunk_keys_backfill.sql");
    assert!(
        fill.contains("source_chunk_keys IS NULL"),
        "backfill must be idempotent"
    );
    assert!(
        fill.contains("ANALYZE bf_edge_keys"),
        "stage and ANALYZE before joining"
    );
    assert!(
        fill.contains("ANALYZE bf_edge_agg"),
        "stage and ANALYZE before joining"
    );

    // The writer fills the column in the same statement as the row, and the
    // search reads it. A column nobody writes was the original defect.
    let sink = include_str!("../src/postgres_entity_sink.rs");
    assert!(
        sink.matches("source_chunk_keys").count() >= 6,
        "single and batch upserts must both write and union source_chunk_keys"
    );
    let search =
        include_str!("../../edgequake-storage/src/adapters/postgres/document_lineage_sql.rs");
    assert!(search.contains("r.source_chunk_keys"));
    assert!(search.contains("e.source_chunk_ids"));
}
