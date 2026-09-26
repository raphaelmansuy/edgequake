//! SPEC-150 migration manifest — single source of truth for phases, fossils,
//! irreversible drops, and the serve-compat window.
//!
//! The TOML at `edgequake/migrations/manifest.toml` is compiled in via
//! `include_str!`. Callers must not maintain parallel integer allowlists.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;
use thiserror::Error;

/// Raw embedded manifest text (validated on first `load()`).
pub const MANIFEST_TOML: &str = include_str!("../../../migrations/manifest.toml");

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("manifest validation: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationPhase {
    Expand,
    Data,
    Contract,
}

impl MigrationPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Expand => "expand",
            Self::Data => "data",
            Self::Contract => "contract",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Fossil {
    /// Full SHA-384 hex (96 chars) of a historically applied body.
    pub sha384: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub equivalence: String,
    /// When true, never auto-accept in production migrate (dev scratch only).
    #[serde(default)]
    pub dev_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationEntry {
    pub version: i64,
    pub file: String,
    pub phase: MigrationPhase,
    #[serde(default)]
    pub no_transaction: bool,
    #[serde(default = "default_lock_class")]
    pub lock_class: String,
    #[serde(default)]
    pub confirm_drop: bool,
    /// Minimum binary schema this migration raises for N-1 rolling serve.
    #[serde(default)]
    pub serving_floor: Option<i64>,
    #[serde(default)]
    pub fossils: Vec<Fossil>,
}

fn default_lock_class() -> String {
    "ddl_access_exclusive".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub compat_serve_min: i64,
    pub compat_serve_max: i64,
    pub irreversible_drop: Vec<i64>,
    pub legacy_cutover_assert: i64,
    pub migration: Vec<MigrationEntry>,
}

impl Manifest {
    /// Parse and validate `MANIFEST_TOML`.
    pub fn parse(raw: &str) -> Result<Self, ManifestError> {
        let m: Manifest = toml::from_str(raw)?;
        m.validate()?;
        Ok(m)
    }

    fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != 1 {
            return Err(ManifestError::Validation(format!(
                "unsupported schema_version {}",
                self.schema_version
            )));
        }
        if self.migration.is_empty() {
            return Err(ManifestError::Validation("no [[migration]] entries".into()));
        }
        let mut seen = std::collections::HashSet::new();
        let mut last = -1i64;
        for e in &self.migration {
            if e.version <= last {
                return Err(ManifestError::Validation(format!(
                    "versions must be sorted unique; got {} after {}",
                    e.version, last
                )));
            }
            last = e.version;
            if !seen.insert(e.version) {
                return Err(ManifestError::Validation(format!(
                    "duplicate version {}",
                    e.version
                )));
            }
            if !e.file.starts_with(&format!("{:03}_", e.version))
                && !e.file.starts_with(&format!("{}_", e.version))
            {
                // Allow 001_ or 1_ prefixes; require version digits at start.
                let digits: String = e.file.chars().take_while(|c| c.is_ascii_digit()).collect();
                if digits.parse::<i64>().ok() != Some(e.version) {
                    return Err(ManifestError::Validation(format!(
                        "file {} does not match version {}",
                        e.file, e.version
                    )));
                }
            }
            for f in &e.fossils {
                if f.sha384.len() != 96 || !f.sha384.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(ManifestError::Validation(format!(
                        "version {}: fossil sha384 must be 96 hex chars",
                        e.version
                    )));
                }
            }
            if e.confirm_drop && e.phase != MigrationPhase::Contract {
                return Err(ManifestError::Validation(format!(
                    "version {}: confirm_drop requires phase=contract",
                    e.version
                )));
            }
        }
        let max_ver = self.migration.last().map(|e| e.version).unwrap_or(0);
        if self.compat_serve_max != max_ver {
            return Err(ManifestError::Validation(format!(
                "compat_serve_max {} != max migration {}",
                self.compat_serve_max, max_ver
            )));
        }
        for v in &self.irreversible_drop {
            let Some(e) = self.by_version(*v) else {
                return Err(ManifestError::Validation(format!(
                    "irreversible_drop {v} missing from [[migration]]"
                )));
            };
            if e.phase != MigrationPhase::Contract {
                return Err(ManifestError::Validation(format!(
                    "irreversible_drop {v} must be phase=contract"
                )));
            }
        }
        if self.by_version(self.legacy_cutover_assert).is_none() {
            return Err(ManifestError::Validation(format!(
                "legacy_cutover_assert {} missing",
                self.legacy_cutover_assert
            )));
        }
        Ok(())
    }

    pub fn by_version(&self, version: i64) -> Option<&MigrationEntry> {
        self.migration.iter().find(|e| e.version == version)
    }

    pub fn embedded_max(&self) -> i64 {
        self.compat_serve_max
    }

    pub fn is_irreversible_drop(&self, version: i64) -> bool {
        self.irreversible_drop.contains(&version)
    }

    pub fn is_legacy_cutover_assert(&self, version: i64) -> bool {
        version == self.legacy_cutover_assert
    }

    /// Versions that have at least one non-dev_only fossil (production auto-accept).
    pub fn known_checksum_repair_versions(&self) -> Vec<i64> {
        self.migration
            .iter()
            .filter(|e| e.fossils.iter().any(|f| !f.dev_only))
            .map(|e| e.version)
            .collect()
    }

    /// Production-accepted fossil hashes for `version` (excludes `dev_only`).
    pub fn production_fossils(&self, version: i64) -> Vec<&str> {
        self.by_version(version)
            .map(|e| {
                e.fossils
                    .iter()
                    .filter(|f| !f.dev_only)
                    .map(|f| f.sha384.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// All fossil hashes for `version`, including `dev_only` (test harness).
    pub fn all_fossils(&self, version: i64) -> Vec<&str> {
        self.by_version(version)
            .map(|e| e.fossils.iter().map(|f| f.sha384.as_str()).collect())
            .unwrap_or_default()
    }

    /// True when `stored_hex` is a known production fossil for `version`.
    pub fn is_known_production_fossil(&self, version: i64, stored_hex: &str) -> bool {
        let lower = stored_hex.to_ascii_lowercase();
        self.production_fossils(version)
            .iter()
            .any(|h| h.eq_ignore_ascii_case(&lower))
    }

    /// True when `stored_hex` is any known fossil (incl. dev_only).
    pub fn is_known_fossil(&self, version: i64, stored_hex: &str) -> bool {
        let lower = stored_hex.to_ascii_lowercase();
        self.all_fossils(version)
            .iter()
            .any(|h| h.eq_ignore_ascii_case(&lower))
    }

    /// Lookup map version → entry for hot paths.
    pub fn index(&self) -> HashMap<i64, &MigrationEntry> {
        self.migration.iter().map(|e| (e.version, e)).collect()
    }
}

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

/// Process-wide parsed manifest. Panics on first call if invalid (fail closed).
pub fn load() -> &'static Manifest {
    MANIFEST.get_or_init(|| {
        Manifest::parse(MANIFEST_TOML).unwrap_or_else(|e| {
            panic!("SPEC-150 manifest.toml invalid: {e}");
        })
    })
}

/// Convenience: irreversible drop versions from the manifest.
pub fn irreversible_drop_versions() -> &'static [i64] {
    load().irreversible_drop.as_slice()
}

pub fn legacy_cutover_assert_version() -> i64 {
    load().legacy_cutover_assert
}

pub fn known_checksum_repair_versions() -> Vec<i64> {
    load().known_checksum_repair_versions()
}

pub fn is_irreversible_drop(version: i64) -> bool {
    load().is_irreversible_drop(version)
}

pub fn is_legacy_cutover_assert(version: i64) -> bool {
    load().is_legacy_cutover_assert(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_manifest_parses() {
        let m = Manifest::parse(MANIFEST_TOML).expect("parse");
        assert_eq!(m.schema_version, 1);
        assert!(!m.migration.is_empty());
        assert_eq!(m.irreversible_drop, vec![125, 126, 131]);
        assert_eq!(m.legacy_cutover_assert, 142);
        assert!(!m.by_version(1).unwrap().fossils.is_empty());
        assert!(!m.by_version(19).unwrap().fossils.is_empty());
        assert!(m.by_version(131).unwrap().fossils.len() >= 2);
        let f150 = m.by_version(150).unwrap();
        assert!(f150.fossils.iter().all(|f| f.dev_only));
    }

    #[test]
    fn known_repair_versions_include_fossils_not_dev_only() {
        let vs = known_checksum_repair_versions();
        for expect in [1i64, 19, 71, 78, 118, 121, 125, 131] {
            assert!(vs.contains(&expect), "missing {expect} in {vs:?}");
        }
        assert!(
            !vs.contains(&150),
            "150 is dev_only and must not be in production repair list"
        );
    }

    #[test]
    fn fossil_001_is_production() {
        let m = load();
        assert!(m.is_known_production_fossil(
            1,
            "9e44513e1b22ab482a3703f394d1f0e35fe24625b77eca236789a3b702bbf6c1ceb9ed8beed4e13c9c4ca4b28feae925"
        ));
        assert!(!m.is_known_production_fossil(1, "deadbeef".repeat(12).as_str()));
    }
}
