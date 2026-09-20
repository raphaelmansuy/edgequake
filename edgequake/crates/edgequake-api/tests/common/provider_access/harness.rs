//! Strict connection helpers for provider-access certification.
//!
//! Unlike the legacy test helper, this module never reads
//! `/tmp/edgequake-db-url` and never derives a database from a developer URL.

use std::env;

pub const ENABLE_ENV: &str = "EDGEQUAKE_PROVIDER_ACCESS_E2E";
pub const REQUIRE_ENV: &str = "EDGEQUAKE_REQUIRE_PROVIDER_ACCESS";

pub fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub fn certification_required() -> bool {
    env_flag(REQUIRE_ENV)
}

/// Return the explicit runner URL, or a machine-readable skip reason.
pub fn certification_database_url() -> Result<Option<String>, String> {
    let enabled = env_flag(ENABLE_ENV);
    let database_url = env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty());

    if !enabled || database_url.is_none() {
        let missing = match (enabled, database_url.is_some()) {
            (false, false) => "EDGEQUAKE_PROVIDER_ACCESS_E2E,DATABASE_URL",
            (false, true) => "EDGEQUAKE_PROVIDER_ACCESS_E2E",
            (true, false) => "DATABASE_URL",
            (true, true) => unreachable!(),
        };
        if certification_required() {
            return Err(format!(
                "provider_access_required_configuration_missing:{missing}"
            ));
        }
        return Ok(None);
    }

    let url = database_url.expect("checked above");
    if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
        return Err("provider_access_database_url_must_be_postgres".to_string());
    }
    Ok(Some(url))
}

/// Build the scratch database URL used by the shell runner.
pub fn database_url_for_run(base_url: &str, run_id: &str) -> Result<String, String> {
    let database = scratch_database_name(run_id)?;
    let (head, query) = base_url
        .split_once('?')
        .map_or((base_url, None), |(head, query)| (head, Some(query)));
    let slash = head
        .rfind('/')
        .ok_or_else(|| "provider_access_database_url_missing_path".to_string())?;
    let rewritten = format!("{}/{}", &head[..slash], database);
    Ok(match query {
        Some(query) => format!("{rewritten}?{query}"),
        None => rewritten,
    })
}

pub fn scratch_database_name(run_id: &str) -> Result<String, String> {
    if run_id.is_empty()
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("provider_access_invalid_run_id".to_string());
    }
    Ok(format!("eq_pa_{}", run_id.replace('-', "_")))
}
