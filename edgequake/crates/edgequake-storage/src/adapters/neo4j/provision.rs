//! Explicit Neo4j schema provisioning.
//!
//! Startup never calls this module implicitly; operators run it as a migration.

use edgequake_storage_contracts::{AccessError, AccessResult};
use serde_json::json;

use super::client::Neo4jClient;

const SCHEMA_STATEMENTS: &[&str] = &[
    "CREATE CONSTRAINT edgequake_entity_key_scope IF NOT EXISTS \
     FOR (n:EntityKey) REQUIRE (n.tenant_id, n.workspace_id, n.logical_id) IS UNIQUE",
    "CREATE CONSTRAINT edgequake_entity_revision_physical IF NOT EXISTS \
     FOR (n:EntityRevision) REQUIRE n.physical_id IS UNIQUE",
    "CREATE CONSTRAINT edgequake_edge_revision_physical IF NOT EXISTS \
     FOR (n:EdgeRevision) REQUIRE n.physical_id IS UNIQUE",
    "CREATE INDEX edgequake_entity_revision_scope_logical IF NOT EXISTS \
     FOR (n:EntityRevision) ON (n.tenant_id, n.workspace_id, n.logical_id)",
    "CREATE INDEX edgequake_edge_revision_scope_logical IF NOT EXISTS \
     FOR (n:EdgeRevision) ON (n.tenant_id, n.workspace_id, n.logical_id)",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neo4jProvisionReport {
    pub statements_applied: usize,
    pub bookmarks: Vec<String>,
}

impl Neo4jClient {
    pub async fn provision_schema(&self) -> AccessResult<Neo4jProvisionReport> {
        let mut transaction = self.open_transaction().await?;
        for statement in SCHEMA_STATEMENTS {
            if let Err(error) = transaction.execute(statement, json!({})).await {
                if let Err(rollback_error) = transaction.rollback().await {
                    return Err(AccessError::UnknownOutcome(format!(
                        "{error}; schema rollback was not confirmed: {rollback_error}"
                    )));
                }
                return Err(error);
            }
        }
        let bookmarks = transaction.commit().await?;
        Ok(Neo4jProvisionReport {
            statements_applied: SCHEMA_STATEMENTS.len(),
            bookmarks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_uses_scoped_application_ids() {
        let schema = SCHEMA_STATEMENTS.join("\n");
        assert!(schema.contains("EntityKey"));
        assert!(schema.contains("tenant_id, n.workspace_id, n.logical_id"));
        assert!(schema.contains("EntityRevision"));
        assert!(schema.contains("EdgeRevision"));
        assert!(schema.contains("physical_id IS UNIQUE"));
        assert!(!schema.contains("elementId"));
    }
}
