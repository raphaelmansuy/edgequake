//! Cedar schema + default classified policies (LAW-146-2 / LAW-146-18).

use std::str::FromStr;

use cedar_policy::{PolicySet, Schema};

use crate::error::{AuthzError, AuthzResult};

/// Minimal Cedar schema for classified document_read.
pub const DEFAULT_CEDAR_SCHEMA: &str = r#"
entity User = {
  clearance: String,
  department: String,
};
entity Document = {
  classification: String,
  share_mode: String,
  project_id: String,
  export_control: Bool,
  pii: Bool,
};
entity Workspace;
action document_read appliesTo {
  principal: User,
  resource: Document,
};
"#;

/// Default forbid: classified requires clearance secret or top_secret.
pub const DEFAULT_CLASSIFIED_POLICY: &str = r#"
permit (
  principal,
  action == Action::"document_read",
  resource
) when {
  resource.share_mode != "classified"
};

forbid (
  principal,
  action == Action::"document_read",
  resource
) when {
  resource.share_mode == "classified" &&
  !(principal.clearance == "secret" || principal.clearance == "top_secret")
};

permit (
  principal,
  action == Action::"document_read",
  resource
) when {
  resource.share_mode == "classified" &&
  (principal.clearance == "secret" || principal.clearance == "top_secret")
};
"#;

/// Compile default schema (smoke for G-146-00).
pub fn compile_default_schema() -> AuthzResult<Schema> {
    Schema::from_cedarschema_str(DEFAULT_CEDAR_SCHEMA)
        .map(|(schema, _warnings)| schema)
        .map_err(|e| AuthzError::Cedar(e.to_string()))
}

/// Compile default policy set against schema.
pub fn compile_default_policy_set() -> AuthzResult<PolicySet> {
    let _schema = compile_default_schema()?;
    PolicySet::from_str(DEFAULT_CLASSIFIED_POLICY)
        .map_err(|e| AuthzError::Cedar(e.to_string()))
}

/// Parse Cedar policy text (PAP publish validation).
pub fn parse_cedar_policy_text(cedar_text: &str) -> AuthzResult<PolicySet> {
    PolicySet::from_str(cedar_text).map_err(|e| AuthzError::Cedar(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g146_00_cedar_schema_compiles() {
        compile_default_schema().expect("schema must compile");
        compile_default_policy_set().expect("policies must compile");
    }
}
