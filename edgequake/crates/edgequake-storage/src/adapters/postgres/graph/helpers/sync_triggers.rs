//! Schema-qualified AGE synchronization triggers, safe from identifier truncation.
use super::super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};

impl PostgresAGEGraphStorage {
    pub(super) async fn ensure_sync_triggers(&self, conn: &mut sqlx::PgConnection) -> Result<()> {
        let g = &self.graph_name;
        // PostgreSQL identifiers are limited to 63 bytes. Keep the short function
        // names inside the graph schema; concatenating the schema into a function
        // name can alias the node and edge functions after truncation.
        let statements = [
            format!(
                r#"CREATE OR REPLACE FUNCTION {g}.eq_sync_node_id() RETURNS trigger AS $$
                BEGIN
                    NEW.eq_node_id := ag_catalog.agtype_to_json(NEW.properties)->>'node_id';
                    RETURN NEW;
                END;
                $$ LANGUAGE plpgsql"#
            ),
            format!(
                r#"CREATE OR REPLACE FUNCTION {g}.eq_sync_edge_ids() RETURNS trigger AS $$
                BEGIN
                    NEW.eq_source_id := COALESCE(NULLIF(TRIM(NEW.eq_source_id), ''),
                        ag_catalog.agtype_to_json(NEW.properties)->>'source_id');
                    NEW.eq_target_id := COALESCE(NULLIF(TRIM(NEW.eq_target_id), ''),
                        ag_catalog.agtype_to_json(NEW.properties)->>'target_id');
                    NEW.eq_rel_type := UPPER(COALESCE(NULLIF(TRIM(NEW.eq_rel_type), ''),
                        NULLIF(TRIM(ag_catalog.agtype_to_json(NEW.properties)->>'relation_type'), ''), 'RELATED_TO'));
                    RETURN NEW;
                END;
                $$ LANGUAGE plpgsql"#
            ),
            format!(
                r#"CREATE OR REPLACE TRIGGER trg_eq_sync_node_id
                BEFORE INSERT OR UPDATE OF properties ON {g}."Node"
                FOR EACH ROW EXECUTE FUNCTION {g}.eq_sync_node_id()"#
            ),
            format!(
                r#"CREATE OR REPLACE TRIGGER trg_eq_sync_edge_ids
                BEFORE INSERT OR UPDATE OF properties ON {g}."EDGE"
                FOR EACH ROW EXECUTE FUNCTION {g}.eq_sync_edge_ids()"#
            ),
        ];
        for sql in statements {
            sqlx::query(&sql).execute(&mut *conn).await.map_err(|e| {
                StorageError::Database(format!("AGE synchronization trigger setup failed: {e}"))
            })?;
        }
        Ok(())
    }
}
