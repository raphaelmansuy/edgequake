//! PostgreSQL runtime capability probes (SPEC-042-E SSOT).
//!
//! Gates Phase E features: uuidv7, halfvec, AGE RLS, AGE COPY loader.
//!
//! ## Vector distance metric (SPEC-083 X-04)
//!
//! EdgeQuake indexes and queries are **cosine-only**
//! (`vector_cosine_ops` / `halfvec_cosine_ops`, operator `<=>`).
//! pgvector also supports L2 (`<->`) and inner product (`<#>`), but those
//! opclasses are not created or queried by this codebase. Do not configure
//! non-cosine metrics expecting a runtime effect.

use sqlx::PgPool;

/// Sole ANN distance metric supported by EdgeQuake (X-04 honesty).
pub const SUPPORTED_VECTOR_METRIC: &str = "cosine";

/// Opclass suffix used for full `vector` columns (cosine only).
pub const VECTOR_COSINE_OPCLASS: &str = "vector_cosine_ops";

/// Vector column storage mode (`EDGEQUAKE_VECTOR_STORAGE`, default `halfvec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorStorageMode {
    Full,
    Half,
}

/// pgvector HNSW dimension ceilings — [pgvector HNSW docs](https://github.com/pgvector/pgvector#hnsw).
pub const HNSW_MAX_DIM_VECTOR: usize = 2000;
pub const HNSW_MAX_DIM_HALFVEC: usize = 4000;

/// Minimum pgvector for iterative index scans (0.8.0 feature).
pub const PGVECTOR_MIN_ITERATIVE_SCAN: &str = "0.8.0";

/// CVE-safe floor (CVE-2026-3172 affected 0.8.0/0.8.1 parallel HNSW builds).
/// Prefer image pin >=0.8.5; readiness warns below this floor.
pub const PGVECTOR_MIN_CVE_SAFE: &str = "0.8.2";

/// True when `extversion` meets the CVE-safe pgvector floor.
pub fn pgvector_meets_cve_floor(version: &str) -> bool {
    extension_version_at_least(version, PGVECTOR_MIN_CVE_SAFE)
}

/// Resolved ANN column type + index viability for a workspace embedding dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnIndexPolicy {
    pub column_type: &'static str,
    pub opclass: &'static str,
    pub hnsw_viable: bool,
    /// `vector` column must become `halfvec` to build HNSW when dim ∈ (2000, 4000].
    pub requires_halfvec_promotion: bool,
}

impl AnnIndexPolicy {
    /// SSOT for runtime DDL and migration SQL authors (SPEC-042 / #275).
    pub fn resolve(dimension: usize, mode: VectorStorageMode) -> Self {
        if dimension > HNSW_MAX_DIM_HALFVEC {
            return Self {
                column_type: mode.pg_type(),
                opclass: mode.cosine_opclass(),
                hnsw_viable: false,
                requires_halfvec_promotion: false,
            };
        }
        if dimension > HNSW_MAX_DIM_VECTOR {
            return Self {
                column_type: "halfvec",
                opclass: "halfvec_cosine_ops",
                hnsw_viable: true,
                requires_halfvec_promotion: mode == VectorStorageMode::Full,
            };
        }
        Self {
            column_type: mode.pg_type(),
            opclass: mode.cosine_opclass(),
            hnsw_viable: true,
            requires_halfvec_promotion: false,
        }
    }
}

impl VectorStorageMode {
    pub fn from_env() -> Self {
        match std::env::var("EDGEQUAKE_VECTOR_STORAGE")
            .unwrap_or_else(|_| "halfvec".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "halfvec" | "half" => Self::Half,
            _ => Self::Full,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Half => "halfvec",
        }
    }

    pub fn pg_type(self) -> &'static str {
        match self {
            Self::Full => "vector",
            Self::Half => "halfvec",
        }
    }

    pub fn cosine_opclass(self) -> &'static str {
        match self {
            Self::Full => VECTOR_COSINE_OPCLASS,
            Self::Half => "halfvec_cosine_ops",
        }
    }

    /// Distance metric label — always `"cosine"` until L2/IP ops exist (X-04).
    pub fn distance_metric(self) -> &'static str {
        let _ = self;
        SUPPORTED_VECTOR_METRIC
    }
}

/// Document ID generator selected at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentIdGenerator {
    UuidV4,
    UuidV7,
}

impl DocumentIdGenerator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UuidV4 => "uuidv4",
            Self::UuidV7 => "uuidv7",
        }
    }
}

/// Parse dotted semver-like extension version into numeric parts + pre-release flag.
fn parse_extension_version(v: &str) -> (Vec<u32>, bool) {
    let v = v.trim();
    let (main, has_prerelease) = match v.find('-') {
        Some(i) => (&v[..i], true),
        None => (v, false),
    };
    let parts: Vec<u32> = main
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    (parts, has_prerelease)
}

/// Compare dotted semver-like extension versions (e.g. `0.8.3`, `1.7.0`).
///
/// Pre-releases sort below release: `0.8.0-rc1` < `0.8.0` (SPEC-090 F-090-22).
pub fn extension_version_at_least(version: &str, minimum: &str) -> bool {
    let (va, va_pre) = parse_extension_version(version);
    let (vb, vb_pre) = parse_extension_version(minimum);
    if va.is_empty() {
        return false;
    }
    for i in 0..vb.len().max(va.len()) {
        let a = va.get(i).copied().unwrap_or(0);
        let b = vb.get(i).copied().unwrap_or(0);
        if a > b {
            return true;
        }
        if a < b {
            return false;
        }
    }
    // Same numeric tuple: a pre-release is strictly less than a release.
    if va_pre && !vb_pre {
        return false;
    }
    true
}

pub fn age_rls_requested() -> bool {
    std::env::var("EDGEQUAKE_AGE_RLS")
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

pub fn age_copy_loader_min_rows() -> usize {
    std::env::var("EDGEQUAKE_BULK_COPY_MIN_ROWS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000)
}

pub fn age_supports_rls(age_extversion: Option<&str>) -> bool {
    age_extversion.is_some_and(|v| extension_version_at_least(v, "1.7.0"))
}

pub fn age_supports_copy_loader(age_extversion: Option<&str>) -> bool {
    age_supports_rls(age_extversion)
}

/// Runtime PostgreSQL / extension capabilities (probed once at pool init).
#[derive(Debug, Clone)]
pub struct PostgresCapabilities {
    pub postgres_major: u32,
    pub uuidv7_available: bool,
    pub vector_storage_mode: VectorStorageMode,
    pub document_id_generator: DocumentIdGenerator,
    pub age_extversion: Option<String>,
    pub age_rls_requested: bool,
    pub age_rls_effective: bool,
    pub age_copy_loader_effective: bool,
    pub age_copy_min_rows: usize,
}

/// Operator-facing PostgreSQL runtime capability matrix (LAW-I6 SSOT).
///
/// Built from [`PostgresCapabilities::detect`] plus live pgvector `extversion`.
/// `/health.schema.postgres_capabilities` and contract tests derive from this
/// struct — do not recompute version gates elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresCapabilityProbe {
    pub postgres_major: u32,
    pub pgvector_version: Option<String>,
    pub age_version: Option<String>,
    pub uuidv7_available: bool,
    pub iterative_scan_available: bool,
    /// AGE ≥ 1.8 provides agtype ↔ jsonb bidirectional casts (RM3 / F-RM-13).
    pub age_jsonb_agtype_cast_available: bool,
}

impl PostgresCapabilityProbe {
    /// Derive the capability matrix from runtime probes (DRY for `/health`).
    pub fn from_runtime(caps: &PostgresCapabilities, pgvector_version: Option<String>) -> Self {
        let iterative_scan_available = pgvector_version
            .as_deref()
            .is_some_and(|v| extension_version_at_least(v, PGVECTOR_MIN_ITERATIVE_SCAN));
        // Accept 1.8.0-rc0+ (PG18 AGE pin); require numeric ≥ 1.8 with rc floor.
        let age_jsonb_agtype_cast_available = caps
            .age_extversion
            .as_deref()
            .is_some_and(|v| extension_version_at_least(v, "1.8.0-rc0"));
        Self {
            postgres_major: caps.postgres_major,
            pgvector_version,
            age_version: caps.age_extversion.clone(),
            uuidv7_available: caps.uuidv7_available,
            iterative_scan_available,
            age_jsonb_agtype_cast_available,
        }
    }

    /// Load pgvector + AGE catalog versions and detect runtime caps in one call.
    pub async fn detect(pool: &PgPool) -> Self {
        let pgvector_version: Option<String> =
            sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'vector'")
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();
        let age_extversion: Option<String> =
            sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'age'")
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();
        let caps = PostgresCapabilities::detect(pool, age_extversion).await;
        Self::from_runtime(&caps, pgvector_version)
    }
}

impl PostgresCapabilities {
    pub async fn detect(pool: &PgPool, age_extversion: Option<String>) -> Self {
        let postgres_major: i32 =
            sqlx::query_scalar("SELECT current_setting('server_version_num')::int / 10000")
                .fetch_one(pool)
                .await
                .unwrap_or(16);

        // Built only on PG18+, where the function exists. A static literal
        // cannot be prepared on the PostgreSQL 16 servers this project ships.
        let uuidv7_probe = format!("SELECT {}() IS NOT NULL", "uuidv7");
        let uuidv7_available = postgres_major >= 18
            && sqlx::query_scalar::<_, bool>(&uuidv7_probe)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .unwrap_or(false);

        let vector_storage_mode = VectorStorageMode::from_env();
        let document_id_generator = if uuidv7_available {
            DocumentIdGenerator::UuidV7
        } else {
            DocumentIdGenerator::UuidV4
        };

        let age_rls_requested = age_rls_requested();
        let age_rls_effective = age_rls_requested && age_supports_rls(age_extversion.as_deref());
        let age_copy_loader_effective = age_supports_copy_loader(age_extversion.as_deref());

        Self {
            postgres_major: postgres_major.max(0) as u32,
            uuidv7_available,
            vector_storage_mode,
            document_id_generator,
            age_extversion,
            age_rls_requested,
            age_rls_effective,
            age_copy_loader_effective,
            age_copy_min_rows: age_copy_loader_min_rows(),
        }
    }
}

#[cfg(test)]
mod ann_index_policy_tests {
    use super::*;

    #[test]
    fn pgvector_cve_floor_rejects_081() {
        assert!(!pgvector_meets_cve_floor("0.8.0"));
        assert!(!pgvector_meets_cve_floor("0.8.1"));
        assert!(pgvector_meets_cve_floor("0.8.2"));
        assert!(pgvector_meets_cve_floor(PGVECTOR_MIN_CVE_SAFE));
        assert!(extension_version_at_least(
            "0.8.5",
            PGVECTOR_MIN_ITERATIVE_SCAN
        ));
    }

    #[test]
    fn contract_vector_metric_cosine_only() {
        assert_eq!(SUPPORTED_VECTOR_METRIC, "cosine");
        assert_eq!(
            VectorStorageMode::Full.distance_metric(),
            SUPPORTED_VECTOR_METRIC
        );
        assert_eq!(
            VectorStorageMode::Half.distance_metric(),
            SUPPORTED_VECTOR_METRIC
        );
        let p = AnnIndexPolicy::resolve(1536, VectorStorageMode::Full);
        assert_eq!(p.opclass, VECTOR_COSINE_OPCLASS);
        assert!(
            p.opclass.contains("cosine"),
            "X-04: ANN opclass must be cosine-only"
        );
        assert!(
            !p.opclass.contains("l2") && !p.opclass.contains("ip"),
            "X-04: must not expose L2/IP opclasses"
        );
    }

    #[test]
    fn resolve_vector_1536_full_mode() {
        let p = AnnIndexPolicy::resolve(1536, VectorStorageMode::Full);
        assert_eq!(p.column_type, "vector");
        assert!(p.hnsw_viable);
        assert!(!p.requires_halfvec_promotion);
    }

    #[test]
    fn resolve_3072_promotes_to_halfvec() {
        let p = AnnIndexPolicy::resolve(3072, VectorStorageMode::Full);
        assert_eq!(p.column_type, "halfvec");
        assert_eq!(p.opclass, "halfvec_cosine_ops");
        assert!(p.hnsw_viable);
        assert!(p.requires_halfvec_promotion);
    }

    #[test]
    fn resolve_5000_skips_hnsw() {
        let p = AnnIndexPolicy::resolve(5000, VectorStorageMode::Full);
        assert!(!p.hnsw_viable);
    }

    #[test]
    fn extension_version_prerelease_below_release() {
        assert!(!extension_version_at_least("0.8.0-rc1", "0.8.0"));
        assert!(extension_version_at_least("0.8.0", "0.8.0-rc1"));
        assert!(extension_version_at_least("0.8.1-rc1", "0.8.0"));
    }

    #[test]
    fn capability_probe_iterative_scan_gate() {
        let caps = PostgresCapabilities {
            postgres_major: 16,
            uuidv7_available: false,
            vector_storage_mode: VectorStorageMode::Half,
            document_id_generator: DocumentIdGenerator::UuidV4,
            age_extversion: Some("1.6.0".into()),
            age_rls_requested: false,
            age_rls_effective: false,
            age_copy_loader_effective: false,
            age_copy_min_rows: 1000,
        };
        let probe = PostgresCapabilityProbe::from_runtime(&caps, Some("0.8.5".into()));
        assert!(probe.iterative_scan_available);
        assert!(!probe.uuidv7_available);
        assert!(!probe.age_jsonb_agtype_cast_available);
        assert_eq!(probe.postgres_major, 16);
        assert_eq!(probe.pgvector_version.as_deref(), Some("0.8.5"));
        assert_eq!(probe.age_version.as_deref(), Some("1.6.0"));

        let old = PostgresCapabilityProbe::from_runtime(&caps, Some("0.7.4".into()));
        assert!(!old.iterative_scan_available);

        let mut caps18 = caps.clone();
        caps18.age_extversion = Some("1.8.0-rc0".into());
        let probe18 = PostgresCapabilityProbe::from_runtime(&caps18, Some("0.8.5".into()));
        assert!(probe18.age_jsonb_agtype_cast_available);
    }

    #[test]
    fn vector_storage_mode_defaults_to_halfvec() {
        let prev = std::env::var("EDGEQUAKE_VECTOR_STORAGE").ok();
        std::env::remove_var("EDGEQUAKE_VECTOR_STORAGE");
        assert_eq!(VectorStorageMode::from_env(), VectorStorageMode::Half);
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_VECTOR_STORAGE", v);
        }
    }
}
