//! Data-access profile validation and runtime composition entry point.

use super::data_access_config::{
    DataAccessConfig, GraphProvider, RelationalProvider, VectorProvider,
};
use edgequake_storage::StorageError;
#[cfg(feature = "postgres")]
use edgequake_storage::{
    contracts::{DocumentReader, IngestionCommitter, LifecycleCommitter},
    traits::{GraphStorage, VectorStorage, WorkspaceVectorRegistry},
    DimensionEnsureOutcome, DimensionReconcilePolicy, PgIngestionCommitter, PgProjectionLedger,
    PgVectorStorage, PgWorkspaceVectorRegistry, PostgresAGEGraphStorage, PostgresConfig,
    PostgresPool, ProjectionWorkLedger,
};
#[cfg(feature = "postgres")]
use std::sync::Arc;
#[cfg(feature = "qdrant")]
use uuid::Uuid;

/// Validated provider selection.
///
/// `DataAccessFactory::build` may accept future P1–P4 configurations for
/// compile/feature checks. Product serving still only wires PostgreSQL + AGE +
/// colocated pgvector (`P0`). Call [`DataAccessRuntimes::assert_product_serving_allowed`]
/// before constructing adapters so a non-P0 env cannot silently serve AGE/pgvector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataAccessRuntimes {
    pub profile_label: String,
    pub config: DataAccessConfig,
}

/// Fully constructed P0 provider bundle.
///
/// This type is intentionally separate from [`DataAccessRuntimes`]: `build`
/// remains a side-effect-free validation step, while materialization starts
/// only after the role-split PostgreSQL pools exist.
#[cfg(feature = "postgres")]
pub struct MaterializedP0Runtimes {
    pub profile_label: String,
    pub config: DataAccessConfig,
    pub graph_storage: Arc<dyn GraphStorage>,
    pub graph_query: Arc<dyn GraphStorage>,
    pub vector_storage: Arc<dyn VectorStorage>,
    pub vector_query: Arc<dyn VectorStorage>,
    pub vector_registry: Arc<dyn WorkspaceVectorRegistry>,
    pub ingestion_committer: Arc<dyn IngestionCommitter>,
    pub lifecycle_committer: Arc<dyn LifecycleCommitter>,
    pub document_reader: Arc<dyn DocumentReader>,
    pub projection_ledger: Arc<dyn ProjectionWorkLedger>,
    pub recreated_default_vector: bool,
}

impl DataAccessRuntimes {
    /// Fail closed unless the selected profile is the only product-wired composition.
    ///
    /// PROVIDER-ACCESS adapters for Qdrant/Neo4j/SQLite exist as prototypes; they
    /// are not selected by `AppState::new_postgres`. Allowing those profiles past
    /// this gate would advertise a binding that is not what the process serves.
    pub fn assert_product_serving_allowed(&self) -> Result<(), StorageError> {
        let is_p0 = self.profile_label == "P0"
            && self.config.relational.provider == RelationalProvider::Postgres
            && self.config.graph.provider == GraphProvider::Age
            && self.config.vector.provider == VectorProvider::PgvectorColocated;
        if is_p0 {
            return Ok(());
        }
        Err(StorageError::InvalidConfig(format!(
            "data-access profile '{}' is not product-selected yet \
             (relational={:?}, graph={:?}, vector={:?}). \
             Serving still hard-wires PostgreSQL + AGE + colocated pgvector (P0). \
             Keep alternate providers unavailable until factory selects adapters, \
             projection delivery is wired, and provider-access certification passes. \
             Unset EDGEQUAKE_DATA_ACCESS_* overrides or use the P0 defaults.",
            self.profile_label,
            self.config.relational.provider,
            self.config.graph.provider,
            self.config.vector.provider,
        )))
    }

    /// Verify required external providers without creating collections or indexes.
    pub async fn verify_readiness(&self) -> Result<(), StorageError> {
        if self.config.graph.provider == GraphProvider::Neo4j {
            #[cfg(feature = "neo4j")]
            {
                let client = edgequake_storage::Neo4jClient::new(
                    edgequake_storage::Neo4jConfig::from_env().map_err(StorageError::from)?,
                )
                .map_err(StorageError::from)?;
                client
                    .verify_connectivity()
                    .await
                    .map_err(StorageError::from)?;
            }
            #[cfg(not(feature = "neo4j"))]
            {
                return Err(unsupported(
                    "graph",
                    "neo4j",
                    "rebuild with the edgequake-api/neo4j feature",
                ));
            }
        }
        match self.config.vector.provider {
            VectorProvider::PgvectorColocated | VectorProvider::PgvectorStandalone => Ok(()),
            VectorProvider::Qdrant => {
                #[cfg(feature = "qdrant")]
                {
                    let url = connection_value("vector", &self.config.vector.connection_env)?;
                    let binding_id = parse_binding_id(&self.config)?;
                    let client = edgequake_storage::QdrantClient::from_env_url(url, binding_id)
                        .map_err(StorageError::from)?;
                    edgequake_storage::verify_qdrant_binding(&client, None)
                        .await
                        .map_err(StorageError::from)
                }
                #[cfg(not(feature = "qdrant"))]
                {
                    Err(unsupported(
                        "vector",
                        "qdrant",
                        "rebuild with the edgequake-api/qdrant feature",
                    ))
                }
            }
        }
    }
}

pub struct DataAccessFactory;

impl DataAccessFactory {
    pub fn build(config: DataAccessConfig) -> Result<DataAccessRuntimes, StorageError> {
        validate_connection_env("relational", &config.relational.connection_env)?;
        validate_connection_env("graph", &config.graph.connection_env)?;
        validate_connection_env("vector", &config.vector.connection_env)?;
        validate_layout_compatibility(config.relational.provider, config.vector.provider)?;

        match config.relational.provider {
            RelationalProvider::Postgres => {}
            RelationalProvider::Sqlite => {
                #[cfg(feature = "sqlite")]
                {
                    let path = connection_value("relational", &config.relational.connection_env)?;
                    edgequake_storage::validate_sqlite_deployment(&path)?;
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    return Err(unsupported(
                        "relational",
                        "sqlite",
                        "rebuild with the edgequake-api/sqlite feature",
                    ));
                }
            }
        }
        match config.graph.provider {
            GraphProvider::Age => {}
            GraphProvider::Neo4j => {
                #[cfg(feature = "neo4j")]
                {
                    edgequake_storage::Neo4jConfig::from_env().map_err(StorageError::from)?;
                }
                #[cfg(not(feature = "neo4j"))]
                {
                    return Err(unsupported(
                        "graph",
                        "neo4j",
                        "rebuild with the edgequake-api/neo4j feature or use AGE",
                    ));
                }
            }
        }
        match config.vector.provider {
            VectorProvider::PgvectorColocated => {}
            VectorProvider::PgvectorStandalone => {
                #[cfg(not(feature = "postgres"))]
                {
                    return Err(unsupported(
                        "vector",
                        "pgvector_standalone",
                        "P4 requires the postgres feature for the standalone vector provider",
                    ));
                }
            }
            VectorProvider::Qdrant => {
                #[cfg(feature = "qdrant")]
                {
                    parse_binding_id(&config)?;
                    connection_value("vector", &config.vector.connection_env)?;
                }
                #[cfg(not(feature = "qdrant"))]
                {
                    return Err(unsupported(
                        "vector",
                        "qdrant",
                        "rebuild with the edgequake-api/qdrant feature or use pgvector_colocated",
                    ));
                }
            }
        }

        let profile_label = match (
            config.relational.provider,
            config.graph.provider,
            config.vector.provider,
        ) {
            (RelationalProvider::Sqlite, GraphProvider::Neo4j, VectorProvider::Qdrant) => "P3",
            (
                RelationalProvider::Sqlite,
                GraphProvider::Neo4j,
                VectorProvider::PgvectorStandalone,
            ) => "P4",
            (_, GraphProvider::Age, VectorProvider::Qdrant) => "P1",
            (_, GraphProvider::Neo4j, VectorProvider::Qdrant) => "P2b",
            (_, GraphProvider::Neo4j, _) => "P2a",
            (_, GraphProvider::Age, _) => "P0",
        };
        Ok(DataAccessRuntimes {
            profile_label: profile_label.into(),
            config,
        })
    }

    /// Materialize the only product-serving composition: PostgreSQL + AGE +
    /// colocated pgvector (P0).
    ///
    /// The fail-closed gate is repeated here deliberately so no caller can
    /// validate one profile and accidentally construct the P0 adapters for it.
    #[cfg(feature = "postgres")]
    pub async fn materialize_p0(
        validated: &DataAccessRuntimes,
        ingest_pool: PostgresPool,
        query_pool: PostgresPool,
        pg_config: PostgresConfig,
        embedding_dim: usize,
    ) -> Result<MaterializedP0Runtimes, StorageError> {
        validated.assert_product_serving_allowed()?;

        let graph_storage: Arc<dyn GraphStorage> = Arc::new(PostgresAGEGraphStorage::with_pool(
            ingest_pool.clone(),
            pg_config.clone(),
        ));
        let graph_query: Arc<dyn GraphStorage> = Arc::new(PostgresAGEGraphStorage::with_pool(
            query_pool.clone(),
            pg_config.clone(),
        ));

        let provisional = PgVectorStorage::with_pool_and_dimension(
            ingest_pool.clone(),
            pg_config.clone(),
            embedding_dim,
        );
        let outcome = provisional
            .reconcile_dimension(embedding_dim, DimensionReconcilePolicy::PreferExisting)
            .await?;
        let (vector_storage, recreated_default_vector): (Arc<dyn VectorStorage>, bool) =
            match outcome {
                DimensionEnsureOutcome::Matched => (Arc::new(provisional), false),
                DimensionEnsureOutcome::Recreated => (Arc::new(provisional), true),
                DimensionEnsureOutcome::KeptExisting { stored, required } => {
                    tracing::warn!(
                        stored_dimension = stored,
                        provider_dimension = required,
                        "Default vector table kept at stored dimension (PreferExisting)"
                    );
                    (
                        Arc::new(PgVectorStorage::with_pool_and_dimension(
                            ingest_pool.clone(),
                            pg_config.clone(),
                            stored,
                        )),
                        false,
                    )
                }
            };
        let vector_query: Arc<dyn VectorStorage> =
            Arc::new(PgVectorStorage::with_pool_and_dimension(
                query_pool,
                pg_config.clone(),
                vector_storage.dimension(),
            ));
        let vector_registry: Arc<dyn WorkspaceVectorRegistry> =
            Arc::new(PgWorkspaceVectorRegistry::new(
                pg_config,
                ingest_pool.clone(),
                Arc::clone(&vector_storage),
                embedding_dim,
            ));

        let sqlx_pool = ingest_pool.get().await?;
        let committer = Arc::new(PgIngestionCommitter::new(sqlx_pool.clone()));
        let ingestion_committer: Arc<dyn IngestionCommitter> = committer.clone();
        let lifecycle_committer: Arc<dyn LifecycleCommitter> = committer.clone();
        let document_reader: Arc<dyn DocumentReader> = committer;
        let projection_ledger: Arc<dyn ProjectionWorkLedger> =
            Arc::new(PgProjectionLedger::new(sqlx_pool));

        Ok(MaterializedP0Runtimes {
            profile_label: "P0".into(),
            config: validated.config.clone(),
            graph_storage,
            graph_query,
            vector_storage,
            vector_query,
            vector_registry,
            ingestion_committer,
            lifecycle_committer,
            document_reader,
            projection_ledger,
            recreated_default_vector,
        })
    }

    /// P3 has no PostgreSQL fallback, so all extracted operational groups must
    /// be installed before the server can accept traffic.
    pub fn require_operational_ports(
        runtimes: &DataAccessRuntimes,
        stores: &super::OperationalStores,
    ) -> Result<(), StorageError> {
        if runtimes.profile_label == "P3" && !stores.required_p3_ports_present() {
            return Err(StorageError::InvalidConfig(
                "P3 requires identity, session, workspace, and checkpoint/artifact ports".into(),
            ));
        }
        Ok(())
    }
}

fn validate_layout_compatibility(
    relational: RelationalProvider,
    vector: VectorProvider,
) -> Result<(), StorageError> {
    if relational == RelationalProvider::Sqlite && vector == VectorProvider::PgvectorColocated {
        return Err(StorageError::InvalidConfig(
            "pgvector_colocated requires PostgreSQL relational authority; \
             use pgvector_standalone with SQLite"
                .into(),
        ));
    }
    // Standalone pgvector deliberately has no authority-table joins/FKs, so
    // it remains a valid future layout with SQLite.
    Ok(())
}

fn validate_connection_env(axis: &str, connection_env: &str) -> Result<(), StorageError> {
    if connection_env.trim().is_empty() {
        return Err(StorageError::InvalidConfig(format!(
            "{axis}.connection_env must name a non-empty environment variable"
        )));
    }
    Ok(())
}

#[cfg(any(feature = "qdrant", feature = "sqlite"))]
fn connection_value(axis: &str, connection_env: &str) -> Result<String, StorageError> {
    std::env::var(connection_env)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            StorageError::InvalidConfig(format!(
                "{axis} provider requires non-empty environment variable {connection_env}"
            ))
        })
}

#[cfg(feature = "qdrant")]
fn parse_binding_id(config: &DataAccessConfig) -> Result<Uuid, StorageError> {
    let value = config.binding_id.as_deref().ok_or_else(|| {
        StorageError::InvalidConfig("Qdrant requires EDGEQUAKE_DATA_ACCESS_BINDING_ID".into())
    })?;
    Uuid::parse_str(value).map_err(|error| {
        StorageError::InvalidConfig(format!("invalid Qdrant binding UUID '{value}': {error}"))
    })
}

#[allow(dead_code)] // all alternate-provider feature combinations can compile this helper out
fn unsupported(axis: &str, provider: &str, guidance: &str) -> StorageError {
    StorageError::InvalidConfig(format!(
        "unsupported {axis} provider '{provider}': {guidance}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(feature = "qdrant", feature = "neo4j", feature = "sqlite"))]
    use std::sync::{Mutex, MutexGuard};

    #[cfg(any(feature = "qdrant", feature = "neo4j", feature = "sqlite"))]
    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn legacy_profile_builds_as_p0() {
        let runtimes = DataAccessFactory::build(DataAccessConfig::from_legacy_env())
            .expect("legacy profile must remain supported");
        assert_eq!(runtimes.profile_label, "P0");
        runtimes
            .assert_product_serving_allowed()
            .expect("P0 must remain product-selected");
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn materialize_p0_refuses_non_p0_before_adapter_construction() {
        let mut config = DataAccessConfig::from_legacy_env();
        config.vector.provider = VectorProvider::Qdrant;
        let runtimes = DataAccessRuntimes {
            profile_label: "P1".into(),
            config,
        };
        let pg_config = PostgresConfig::default();
        let pool = PostgresPool::new(pg_config.clone());
        let error =
            match DataAccessFactory::materialize_p0(&runtimes, pool.clone(), pool, pg_config, 1536)
                .await
            {
                Ok(_) => panic!("non-P0 must fail closed before adapter construction"),
                Err(error) => error,
            };
        assert!(error.to_string().contains("not product-selected"));
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn materialize_p0_returns_age_pgvector_committer_and_ledger() {
        let database_url = std::env::var("DATABASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                std::fs::read_to_string("/tmp/edgequake-db-url")
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            });
        let Some(database_url) = database_url else {
            eprintln!("SKIP: no DATABASE_URL for materialize_p0 integration check");
            return;
        };
        let Ok(sqlx_pool) = sqlx::PgPool::connect(&database_url).await else {
            eprintln!("SKIP: cannot connect for materialize_p0 integration check");
            return;
        };
        let pg_config = PostgresConfig::default();
        let pool = PostgresPool::from_existing(sqlx_pool, pg_config.clone());
        let runtimes = DataAccessFactory::build(DataAccessConfig::from_legacy_env())
            .expect("P0 config must validate");
        let materialized =
            DataAccessFactory::materialize_p0(&runtimes, pool.clone(), pool, pg_config, 1536)
                .await
                .expect("P0 materialize must succeed against a reachable PostgreSQL");
        assert_eq!(materialized.profile_label, "P0");
        assert_eq!(materialized.config.graph.provider, GraphProvider::Age);
        assert_eq!(
            materialized.config.vector.provider,
            VectorProvider::PgvectorColocated
        );
        assert_eq!(materialized.vector_storage.dimension(), 1536);
        // Handles must be usable as trait objects (committer/ledger/graph).
        let _ = Arc::clone(&materialized.ingestion_committer);
        let _ = Arc::clone(&materialized.lifecycle_committer);
        let _ = Arc::clone(&materialized.document_reader);
        let _ = Arc::clone(&materialized.projection_ledger);
        let _ = Arc::clone(&materialized.graph_storage);
    }

    #[test]
    fn non_p0_profiles_are_refused_for_product_serving() {
        let mut config = DataAccessConfig::from_legacy_env();
        config.vector.provider = VectorProvider::Qdrant;
        // build may fail without the qdrant feature; synthesize a runtime label.
        let runtimes = DataAccessRuntimes {
            profile_label: "P1".into(),
            config,
        };
        let error = runtimes.assert_product_serving_allowed().unwrap_err();
        assert!(matches!(error, StorageError::InvalidConfig(_)));
        assert!(error.to_string().contains("not product-selected"));
        assert!(error.to_string().contains("P0"));
    }

    #[cfg(not(feature = "qdrant"))]
    #[test]
    fn qdrant_is_actionably_rejected_without_feature() {
        let mut config = DataAccessConfig::from_legacy_env();
        config.vector.provider = VectorProvider::Qdrant;

        let error = DataAccessFactory::build(config).unwrap_err();
        assert!(matches!(error, StorageError::InvalidConfig(_)));
        assert!(error.to_string().contains("qdrant"));
        assert!(error.to_string().contains("pgvector_colocated"));
    }

    #[cfg(feature = "qdrant")]
    #[test]
    fn qdrant_profile_builds_as_p1_when_configured() {
        let _guard = env_lock();
        const URL_ENV: &str = "EDGEQUAKE_TEST_QDRANT_URL";
        std::env::set_var(URL_ENV, "http://127.0.0.1:6333");
        let mut config = DataAccessConfig::from_legacy_env();
        config.vector.provider = VectorProvider::Qdrant;
        config.vector.connection_env = URL_ENV.into();
        config.binding_id = Some(Uuid::from_u128(149).to_string());

        let runtimes = DataAccessFactory::build(config).unwrap();

        std::env::remove_var(URL_ENV);
        assert_eq!(runtimes.profile_label, "P1");
        let refuse = runtimes.assert_product_serving_allowed().unwrap_err();
        assert!(refuse.to_string().contains("not product-selected"));
    }

    #[cfg(not(feature = "neo4j"))]
    #[test]
    fn neo4j_is_actionably_rejected_without_feature() {
        let mut config = DataAccessConfig::from_legacy_env();
        config.graph.provider = GraphProvider::Neo4j;
        config.graph.connection_env = "EDGEQUAKE_GRAPH_URL".into();

        let error = DataAccessFactory::build(config).unwrap_err();
        assert!(error.to_string().contains("neo4j"));
        assert!(error.to_string().contains("edgequake-api/neo4j"));
    }

    #[cfg(feature = "neo4j")]
    #[test]
    fn neo4j_pgvector_profile_builds_as_p2a() {
        let _guard = env_lock();
        set_neo4j_env();
        let mut config = DataAccessConfig::from_legacy_env();
        config.graph.provider = GraphProvider::Neo4j;
        config.graph.connection_env = "EDGEQUAKE_GRAPH_URL".into();

        let runtimes = DataAccessFactory::build(config).unwrap();

        clear_neo4j_env();
        assert_eq!(runtimes.profile_label, "P2a");
        assert!(runtimes
            .assert_product_serving_allowed()
            .unwrap_err()
            .to_string()
            .contains("not product-selected"));
    }

    #[cfg(all(feature = "neo4j", feature = "qdrant"))]
    #[test]
    fn neo4j_qdrant_profile_builds_as_p2b() {
        let _guard = env_lock();
        set_neo4j_env();
        std::env::set_var("EDGEQUAKE_TEST_QDRANT_URL", "http://127.0.0.1:6333");
        let mut config = DataAccessConfig::from_legacy_env();
        config.graph.provider = GraphProvider::Neo4j;
        config.graph.connection_env = "EDGEQUAKE_GRAPH_URL".into();
        config.vector.provider = VectorProvider::Qdrant;
        config.vector.connection_env = "EDGEQUAKE_TEST_QDRANT_URL".into();
        config.binding_id = Some(Uuid::from_u128(149).to_string());

        let runtimes = DataAccessFactory::build(config).unwrap();

        std::env::remove_var("EDGEQUAKE_TEST_QDRANT_URL");
        clear_neo4j_env();
        assert_eq!(runtimes.profile_label, "P2b");
        assert!(runtimes
            .assert_product_serving_allowed()
            .unwrap_err()
            .to_string()
            .contains("not product-selected"));
    }

    #[test]
    fn colocated_pgvector_is_rejected_with_sqlite() {
        let error = validate_layout_compatibility(
            RelationalProvider::Sqlite,
            VectorProvider::PgvectorColocated,
        )
        .unwrap_err();
        assert!(error.to_string().contains("requires PostgreSQL"));
    }

    #[test]
    fn standalone_pgvector_layout_is_compatible_with_sqlite() {
        validate_layout_compatibility(
            RelationalProvider::Sqlite,
            VectorProvider::PgvectorStandalone,
        )
        .unwrap();
    }

    #[cfg(not(feature = "sqlite"))]
    #[test]
    fn p3_fails_closed_without_sqlite_feature() {
        let mut config = DataAccessConfig::from_legacy_env();
        config.relational.provider = RelationalProvider::Sqlite;
        config.relational.connection_env = "SQLITE_PATH".into();
        config.vector.provider = VectorProvider::Qdrant;
        let error = DataAccessFactory::build(config).unwrap_err();
        assert!(error.to_string().contains("edgequake-api/sqlite"));
    }

    #[cfg(all(feature = "sqlite", feature = "neo4j", feature = "qdrant"))]
    #[test]
    fn p3_validates_without_database_url() {
        let _guard = env_lock();
        std::env::remove_var("DATABASE_URL");
        std::env::set_var("SQLITE_PATH", "/tmp/edgequake-p3-validation.db");
        std::env::set_var("EDGEQUAKE_TEST_QDRANT_URL", "http://127.0.0.1:6333");
        set_neo4j_env();
        let config = DataAccessConfig {
            relational: crate::state::data_access_config::RelationalConfig {
                provider: RelationalProvider::Sqlite,
                connection_env: "SQLITE_PATH".into(),
            },
            graph: crate::state::data_access_config::GraphConfig {
                provider: GraphProvider::Neo4j,
                connection_env: "EDGEQUAKE_GRAPH_URL".into(),
            },
            vector: crate::state::data_access_config::VectorConfig {
                provider: VectorProvider::Qdrant,
                connection_env: "EDGEQUAKE_TEST_QDRANT_URL".into(),
            },
            binding_id: Some(Uuid::from_u128(149).to_string()),
        };

        let runtimes = DataAccessFactory::build(config).unwrap();
        assert_eq!(runtimes.profile_label, "P3");
        assert!(runtimes
            .assert_product_serving_allowed()
            .unwrap_err()
            .to_string()
            .contains("not product-selected"));
        let error = DataAccessFactory::require_operational_ports(
            &runtimes,
            &crate::state::OperationalStores::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("requires identity"));

        std::env::remove_var("SQLITE_PATH");
        std::env::remove_var("EDGEQUAKE_TEST_QDRANT_URL");
        clear_neo4j_env();
    }

    #[cfg(feature = "neo4j")]
    fn set_neo4j_env() {
        std::env::set_var("EDGEQUAKE_GRAPH_URL", "http://127.0.0.1:7474");
        std::env::set_var("EDGEQUAKE_NEO4J_USER", "neo4j");
        std::env::set_var("EDGEQUAKE_NEO4J_PASSWORD", "password");
        std::env::set_var("EDGEQUAKE_NEO4J_TOPOLOGY", "single");
    }

    #[cfg(feature = "neo4j")]
    fn clear_neo4j_env() {
        for name in [
            "EDGEQUAKE_GRAPH_URL",
            "EDGEQUAKE_NEO4J_USER",
            "EDGEQUAKE_NEO4J_PASSWORD",
            "EDGEQUAKE_NEO4J_TOPOLOGY",
        ] {
            std::env::remove_var(name);
        }
    }
}
