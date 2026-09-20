//! Serializable provider-axis configuration for the data-access composition root.

use edgequake_storage::StorageError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const RELATIONAL_PROVIDER_ENV: &str = "EDGEQUAKE_DATA_ACCESS_RELATIONAL_PROVIDER";
pub const GRAPH_PROVIDER_ENV: &str = "EDGEQUAKE_DATA_ACCESS_GRAPH_PROVIDER";
pub const VECTOR_PROVIDER_ENV: &str = "EDGEQUAKE_DATA_ACCESS_VECTOR_PROVIDER";
pub const BINDING_ID_ENV: &str = "EDGEQUAKE_DATA_ACCESS_BINDING_ID";
pub const VECTOR_URL_ENV: &str = "EDGEQUAKE_VECTOR_URL";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationalProvider {
    Postgres,
    Sqlite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphProvider {
    Age,
    Neo4j,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorProvider {
    PgvectorColocated,
    PgvectorStandalone,
    Qdrant,
}

macro_rules! provider_from_str {
    ($provider:ty, $($name:literal => $variant:path),+ $(,)?) => {
        impl FromStr for $provider {
            type Err = StorageError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value.trim().to_ascii_lowercase().as_str() {
                    $($name => Ok($variant),)+
                    unknown => Err(StorageError::InvalidConfig(format!(
                        "unknown {} provider '{unknown}'; expected one of: {}",
                        stringify!($provider),
                        [$($name),+].join(", ")
                    ))),
                }
            }
        }
    };
}

provider_from_str!(
    RelationalProvider,
    "postgres" => RelationalProvider::Postgres,
    "postgresql" => RelationalProvider::Postgres,
    "sqlite" => RelationalProvider::Sqlite,
);
provider_from_str!(
    GraphProvider,
    "age" => GraphProvider::Age,
    "neo4j" => GraphProvider::Neo4j,
);
provider_from_str!(
    VectorProvider,
    "pgvector_colocated" => VectorProvider::PgvectorColocated,
    "pgvector-colocated" => VectorProvider::PgvectorColocated,
    "pgvector_standalone" => VectorProvider::PgvectorStandalone,
    "pgvector-standalone" => VectorProvider::PgvectorStandalone,
    "qdrant" => VectorProvider::Qdrant,
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalConfig {
    pub provider: RelationalProvider,
    pub connection_env: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphConfig {
    pub provider: GraphProvider,
    pub connection_env: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorConfig {
    pub provider: VectorProvider,
    pub connection_env: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataAccessConfig {
    pub relational: RelationalConfig,
    pub graph: GraphConfig,
    pub vector: VectorConfig,
    pub binding_id: Option<String>,
}

impl DataAccessConfig {
    /// Preserve the existing DATABASE_URL-only deployment as profile P0.
    pub fn from_legacy_env() -> Self {
        Self {
            relational: RelationalConfig {
                provider: RelationalProvider::Postgres,
                connection_env: "DATABASE_URL".into(),
            },
            graph: GraphConfig {
                provider: GraphProvider::Age,
                connection_env: "DATABASE_URL".into(),
            },
            vector: VectorConfig {
                provider: VectorProvider::PgvectorColocated,
                connection_env: "DATABASE_URL".into(),
            },
            binding_id: None,
        }
    }

    /// Read optional provider overrides, falling back axis-by-axis to legacy P0.
    pub fn from_env() -> Result<Self, StorageError> {
        let mut config = Self::from_legacy_env();

        if let Some(value) = provider_env(RELATIONAL_PROVIDER_ENV, "EDGEQUAKE_RELATIONAL_PROVIDER")
        {
            config.relational.provider = value.parse()?;
            if config.relational.provider == RelationalProvider::Sqlite {
                config.relational.connection_env = "SQLITE_PATH".into();
            }
        }
        if let Some(value) = provider_env(GRAPH_PROVIDER_ENV, "EDGEQUAKE_GRAPH_PROVIDER") {
            config.graph.provider = value.parse()?;
            if config.graph.provider == GraphProvider::Neo4j {
                config.graph.connection_env = "EDGEQUAKE_GRAPH_URL".into();
            }
        }
        if let Some(value) = provider_env(VECTOR_PROVIDER_ENV, "EDGEQUAKE_VECTOR_PROVIDER") {
            config.vector.provider = value.parse()?;
        }

        override_connection_env(
            &mut config.relational.connection_env,
            "EDGEQUAKE_DATA_ACCESS_RELATIONAL_CONNECTION_ENV",
        );
        override_connection_env(
            &mut config.graph.connection_env,
            "EDGEQUAKE_DATA_ACCESS_GRAPH_CONNECTION_ENV",
        );
        match non_empty_env("EDGEQUAKE_DATA_ACCESS_VECTOR_CONNECTION_ENV") {
            Some(value) => config.vector.connection_env = value,
            None if config.vector.provider == VectorProvider::Qdrant => {
                config.vector.connection_env = VECTOR_URL_ENV.into();
            }
            None => {}
        }
        config.binding_id = non_empty_env(BINDING_ID_ENV);

        Ok(config)
    }
}

fn provider_env(primary: &str, compatibility: &str) -> Option<String> {
    non_empty_env(primary).or_else(|| non_empty_env(compatibility))
}

fn override_connection_env(target: &mut String, variable: &str) {
    if let Some(value) = non_empty_env(variable) {
        *target = value;
    }
}

fn non_empty_env(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_env_maps_to_p0_providers() {
        let config = DataAccessConfig::from_legacy_env();
        assert_eq!(config.relational.provider, RelationalProvider::Postgres);
        assert_eq!(config.graph.provider, GraphProvider::Age);
        assert_eq!(config.vector.provider, VectorProvider::PgvectorColocated);
        assert_eq!(config.relational.connection_env, "DATABASE_URL");
    }

    #[test]
    fn provider_typo_is_rejected() {
        let error = "qdrnat".parse::<VectorProvider>().unwrap_err();
        assert!(matches!(error, StorageError::InvalidConfig(_)));
        assert!(error.to_string().contains("qdrnat"));
        assert!(error.to_string().contains("qdrant"));
    }

    #[test]
    fn serde_rejects_unknown_provider() {
        let error = serde_json::from_str::<VectorProvider>("\"qdrnat\"").unwrap_err();
        assert!(error.to_string().contains("unknown variant"));
    }
}
