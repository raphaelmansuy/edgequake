//! PostgreSQL runtime capability probes (SPEC-042-E SSOT).
//!
//! Gates Phase E features: uuidv7, halfvec, AGE RLS, AGE COPY loader.

use sqlx::PgPool;

/// Vector column storage mode (`EDGEQUAKE_VECTOR_STORAGE`, default `full`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorStorageMode {
    Full,
    Half,
}

/// pgvector HNSW dimension ceilings — [pgvector HNSW docs](https://github.com/pgvector/pgvector#hnsw).
pub const HNSW_MAX_DIM_VECTOR: usize = 2000;
pub const HNSW_MAX_DIM_HALFVEC: usize = 4000;

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
            .unwrap_or_else(|_| "full".into())
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
            Self::Full => "vector_cosine_ops",
            Self::Half => "halfvec_cosine_ops",
        }
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

/// Compare dotted semver-like extension versions (e.g. `0.8.3`, `1.7.0`).
pub fn extension_version_at_least(version: &str, minimum: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    let va = parse(version);
    let vb = parse(minimum);
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

impl PostgresCapabilities {
    pub async fn detect(pool: &PgPool, age_extversion: Option<String>) -> Self {
        let postgres_major: i32 =
            sqlx::query_scalar("SELECT current_setting('server_version_num')::int / 10000")
                .fetch_one(pool)
                .await
                .unwrap_or(16);

        let uuidv7_available = postgres_major >= 18
            && sqlx::query_scalar::<_, bool>("SELECT uuidv7() IS NOT NULL")
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
}
