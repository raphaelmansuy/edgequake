//! SPEC-149 PROVIDER-ACCESS-E2E01 baseline smoke.
//!
//! J01 proves strict P0 fixture availability and the production PostgreSQL
//! constructor's health endpoint. E2E01-15 are expanded in later SPEC-149 steps.

#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::provider_access::harness;
use edgequake_api::{AppState, Server, ServerConfig};
use serial_test::serial;
use tower::ServiceExt;

#[tokio::test]
#[serial]
async fn provider_access_e2e01_p0_health() {
    let database_url = match harness::certification_database_url() {
        Ok(Some(url)) => url,
        Ok(None) => {
            eprintln!(
                r#"{{"event":"provider_access_skip","test_id":"PROVIDER-ACCESS-E2E01","reason":"explicit_runner_configuration_missing","certification_successes":0}}"#
            );
            return;
        }
        Err(reason) => panic!(
            r#"{{"event":"provider_access_failure","test_id":"PROVIDER-ACCESS-E2E01","reason":"{reason}","certification_successes":0}}"#
        ),
    };

    std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");

    let state = AppState::new_postgres(&database_url, "")
        .await
        .expect("PROVIDER-ACCESS-E2E01: AppState::new_postgres must succeed");
    assert!(
        state.document_reader.is_some(),
        "PROVIDER-ACCESS-E2E01: document_reader must be wired on P0"
    );
    assert!(
        state.ingestion_committer.is_some(),
        "PROVIDER-ACCESS-E2E01: ingestion_committer must be wired on P0"
    );
    assert!(
        state.lifecycle_committer.is_some(),
        "PROVIDER-ACCESS-E2E01: lifecycle_committer must be wired on P0"
    );
    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("health request"),
        )
        .await
        .expect("health response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "PROVIDER-ACCESS-E2E01: production health endpoint must be ready"
    );

    eprintln!(
        r#"{{"event":"provider_access_pass","test_id":"PROVIDER-ACCESS-E2E01","profile":"P0","certification_successes":1}}"#
    );
}

#[cfg(not(feature = "sqlite"))]
#[test]
fn provider_access_e2e14_p3_without_sqlite_feature_fails_closed() {
    use edgequake_api::state::data_access_config::{
        DataAccessConfig, RelationalProvider, VectorProvider,
    };
    use edgequake_api::state::data_access_factory::DataAccessFactory;

    let mut config = DataAccessConfig::from_legacy_env();
    config.relational.provider = RelationalProvider::Sqlite;
    config.relational.connection_env = "SQLITE_PATH".into();
    config.vector.provider = VectorProvider::Qdrant;
    let error = DataAccessFactory::build(config).unwrap_err();
    assert!(
        error.to_string().contains("edgequake-api/sqlite"),
        "PROVIDER-ACCESS-E2E14 must reject P3 when SQLite is not compiled"
    );
}

#[test]
fn provider_access_e2e15_p3_without_required_ports_fails_closed() {
    use edgequake_api::state::data_access_config::{
        DataAccessConfig, GraphConfig, GraphProvider, RelationalConfig, RelationalProvider,
        VectorConfig, VectorProvider,
    };
    use edgequake_api::state::data_access_factory::{DataAccessFactory, DataAccessRuntimes};
    use edgequake_api::state::OperationalStores;

    let runtimes = DataAccessRuntimes {
        profile_label: "P3".into(),
        config: DataAccessConfig {
            relational: RelationalConfig {
                provider: RelationalProvider::Sqlite,
                connection_env: "SQLITE_PATH".into(),
            },
            graph: GraphConfig {
                provider: GraphProvider::Neo4j,
                connection_env: "EDGEQUAKE_GRAPH_URL".into(),
            },
            vector: VectorConfig {
                provider: VectorProvider::Qdrant,
                connection_env: "EDGEQUAKE_VECTOR_URL".into(),
            },
            binding_id: None,
        },
    };
    let error =
        DataAccessFactory::require_operational_ports(&runtimes, &OperationalStores::default())
            .unwrap_err();
    assert!(
        error.to_string().contains("requires identity"),
        "PROVIDER-ACCESS-E2E15 must reject a P3 runtime with missing operational ports"
    );
}
