//! Checksum repair authorization (LAW-MIG / SPEC-083 X-02 / SPEC-090 §8.3 / SPEC-150).
//!
//! # First principles
//!
//! 1. **Applied migration SQL is immutable.** Edit → new version. Never patch a
//!    shipped `NNN_*.sql` body to “fix” field DBs (sqlx stores SHA-384 in
//!    `_sqlx_migrations`; byte drift aborts migrate).
//! 2. **Known fossils are data.** When a historically applied body matches a
//!    fossil listed in `edgequake/migrations/manifest.toml`, `edgequake migrate`
//!    rewrites the stored checksum to the current hash **without env**.
//! 3. **Unknown mismatch fails closed.** `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` remains
//!    a scoped emergency override. `EDGEQUAKE_DEV_MODE` does **not** auto-allow
//!    unknown hashes (auth bleed).

use edgequake_migrate_manifest as manifest;

/// Env: comma-separated migration versions allowed for one-shot checksum rewrite
/// of **unknown** (non-fossil) mismatches.
pub const ALLOW_CHECKSUM_REPAIR_ENV: &str = "EDGEQUAKE_ALLOW_CHECKSUM_REPAIR";

/// Versions that have at least one production fossil in the manifest.
///
/// Prefer calling this over any hard-coded integer array — SPEC-150 SSOT is
/// `edgequake/migrations/manifest.toml`.
pub fn known_checksum_repair_versions() -> Vec<i64> {
    manifest::known_checksum_repair_versions()
}

/// Parse `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` (comma/space separated i64 versions).
pub fn parse_allow_checksum_repair_list(raw: &str) -> Vec<i64> {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|part| {
            let t = part.trim();
            if t.is_empty() {
                return None;
            }
            t.parse::<i64>().ok()
        })
        .collect()
}

/// Authorize rewriting `_sqlx_migrations.checksum` for an **unknown** mismatch.
///
/// Known production fossils are accepted separately by [`authorize_checksum_rewrite`]
/// and do not consult this function.
///
/// Order:
/// 1. `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` contains `version` → allow (scoped emergency)
/// 2. else deny
///
/// `EDGEQUAKE_DEV_MODE` deliberately does **not** authorize unknown hashes
/// (SPEC-150 WP-2).
pub fn allow_checksum_repair(version: i64) -> bool {
    match std::env::var(ALLOW_CHECKSUM_REPAIR_ENV) {
        Ok(raw) => parse_allow_checksum_repair_list(&raw).contains(&version),
        Err(_) => false,
    }
}

/// Authorize a checksum rewrite for `version` given the currently stored hex.
///
/// - Known production fossil → Ok (auto-accept, no env)
/// - Emergency allowlist → Ok
/// - Else → Err(refuse message)
pub fn authorize_checksum_rewrite(
    version: i64,
    stored_hex: &str,
    reason: &str,
) -> Result<(), String> {
    if is_known_production_fossil(version, stored_hex) {
        return Ok(());
    }
    if allow_checksum_repair(version) {
        return Ok(());
    }
    Err(refuse_silent_repair_message(version, reason))
}

/// True when `stored_hex` is a known production fossil for `version`.
pub fn is_known_production_fossil(version: i64, stored_hex: &str) -> bool {
    manifest::load().is_known_production_fossil(version, stored_hex)
}

/// True when `stored_hex` is any known fossil (including `dev_only`).
pub fn is_known_fossil(version: i64, stored_hex: &str) -> bool {
    manifest::load().is_known_fossil(version, stored_hex)
}

/// Fail-loud protocol message shared by repair modules.
pub fn refuse_silent_repair_message(version: i64, reason: &str) -> String {
    let known = manifest::load().production_fossils(version);
    let known_fmt = if known.is_empty() {
        "(none listed in manifest)".to_string()
    } else {
        known
            .iter()
            .map(|h| format!("{}…", &h[..h.len().min(24)]))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "Migration {version} checksum drift detected ({reason}). \
         Refusing silent repair without authorization. \
         Known production fossils: {known_fmt}. \
         Controlled emergency: {ALLOW_CHECKSUM_REPAIR_ENV}={version} once, then unset. \
         Spec: specs/150-reliable-migration-system/01-first-principles.md (LAW-150-4)."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_allow_list_accepts_commas_and_spaces() {
        assert_eq!(
            parse_allow_checksum_repair_list("71, 78,125"),
            vec![71, 78, 125]
        );
        assert_eq!(parse_allow_checksum_repair_list("131"), vec![131]);
        assert!(parse_allow_checksum_repair_list("").is_empty());
        assert!(parse_allow_checksum_repair_list("nope").is_empty());
    }

    #[test]
    fn known_versions_come_from_manifest_sorted_unique() {
        let vs = known_checksum_repair_versions();
        let mut sorted = vs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted, vs);
        assert!(vs.contains(&1), "001 fossil must be registered");
        assert!(vs.contains(&19), "019 fossil must be registered");
        assert!(!vs.contains(&150), "150 is dev_only");
    }

    #[test]
    fn refuse_message_names_scoped_env_and_manifest() {
        let msg = refuse_silent_repair_message(125, "SPEC-111 cast");
        assert!(msg.contains("EDGEQUAKE_ALLOW_CHECKSUM_REPAIR"));
        assert!(msg.contains("125"));
        assert!(msg.contains("LAW-150-4"));
    }

    #[test]
    fn dev_mode_does_not_authorize_unknown() {
        std::env::remove_var(ALLOW_CHECKSUM_REPAIR_ENV);
        // 999 has no fossil and no allowlist entry.
        assert!(!allow_checksum_repair(999));
    }
}
