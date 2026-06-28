//! SSOT for conversation share URL paths (SPEC-027 IMP-027).

/// API-relative path for a shared conversation (matches `routes.rs` mount under `/api/v1`).
pub fn share_api_path(share_id: &str) -> String {
    format!("/api/v1/shared/{share_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_api_path_matches_v1_route() {
        assert_eq!(share_api_path("abc123"), "/api/v1/shared/abc123");
    }
}
