//! Content granularity helpers (SPEC-028 / SPEC-037 DRY SSOT).

use edgequake_auth::Role;

use crate::error::{ApiError, ApiResult};
use crate::handlers::context_types::ContentGranularity;

/// Maximum snippet length for `ContentGranularity::Citation`.
pub const SNIPPET_LEN: usize = 200;

/// Truncate text according to payload tier.
pub fn truncate_for_granularity(content: &str, granularity: ContentGranularity) -> String {
    match granularity {
        ContentGranularity::Citation => content.chars().take(SNIPPET_LEN).collect(),
        ContentGranularity::Agent | ContentGranularity::Debug => content.to_string(),
    }
}

/// EC-MCP-29 / REQ-037-08: `debug` tier requires admin when a role is present.
pub fn ensure_debug_granularity_allowed(
    granularity: ContentGranularity,
    role: Option<Role>,
) -> ApiResult<()> {
    if granularity != ContentGranularity::Debug {
        return Ok(());
    }
    match role {
        Some(Role::Admin) | None => Ok(()),
        Some(_) => Err(ApiError::forbidden_reason(
            "content_granularity debug requires admin role",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citation_truncates_at_snippet_len() {
        let long = "x".repeat(500);
        let out = truncate_for_granularity(&long, ContentGranularity::Citation);
        assert_eq!(out.len(), SNIPPET_LEN);
    }

    #[test]
    fn agent_returns_full_content() {
        let long = "y".repeat(500);
        let out = truncate_for_granularity(&long, ContentGranularity::Agent);
        assert_eq!(out.len(), 500);
    }

    #[test]
    fn debug_requires_admin_when_role_user() {
        assert!(
            ensure_debug_granularity_allowed(ContentGranularity::Debug, Some(Role::User)).is_err()
        );
    }

    #[test]
    fn debug_allowed_for_admin() {
        assert!(
            ensure_debug_granularity_allowed(ContentGranularity::Debug, Some(Role::Admin)).is_ok()
        );
    }
}
