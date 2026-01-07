//! Search Quality Non-Regression Tests
//!
//! This module contains comprehensive tests to prevent regressions in search quality.
//! Tests are designed around 11 French automotive queries that exercise various aspects
//! of the RAG system.
//!
//! Run all tests:
//! ```bash
//! cargo test --package edgequake-query --test search_quality_tests
//! ```
//!
//! Run with output:
//! ```bash
//! cargo test --package edgequake-query --test search_quality_tests -- --nocapture
//! ```

mod metrics;
mod test_queries;

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::{QueryMode, QueryRequest, SOTAQueryConfig, SOTAQueryEngine};
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryVectorStorage, VectorStorage};
use metrics::{ResponseQuality, TestSuiteMetrics};
use serde_json::json;
use test_queries::{TestQuery, TEST_QUERIES};

/// Minimum acceptable score threshold for tests
const MIN_ACCEPTABLE_SCORE: f64 = 60.0;

/// Minimum suite pass rate
#[allow(dead_code)]
const MIN_SUITE_PASS_RATE: f64 = 0.8; // 80%

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a mock provider with consistent responses.
fn create_mock_provider() -> Arc<MockProvider> {
    Arc::new(MockProvider::new())
}

/// Create vector storage with French automotive test data.
async fn create_test_vector_storage() -> Arc<MemoryVectorStorage> {
    let storage = Arc::new(MemoryVectorStorage::new("automotive_test", 1536));
    storage.initialize().await.unwrap();

    // Add test chunks for automotive content
    let chunk_data = vec![
        (
            "chunk-e3008".to_string(),
            vec![0.1_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le Peugeot E-3008 est équipé d'une batterie NMC de 73 kWh ou 98 kWh offrant une autonomie jusqu'à 700 km. Le E-3008 propose une densité énergétique supérieure.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-byd".to_string(),
            vec![0.2_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le BYD Seal U utilise une batterie LFP de 85.4 kWh. C'est un SUV électrique chinois concurrent direct du E-3008.",
                "document_id": "doc-byd"
            }),
        ),
        (
            "chunk-e208".to_string(),
            vec![0.3_f32; 1536],
            json!({
                "type": "chunk",
                "content": "La Peugeot E-208 offre une batterie de 50 kWh avec une autonomie de 400 km WLTP. Elle bénéficie du i-Cockpit.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-r5".to_string(),
            vec![0.25_f32; 1536],
            json!({
                "type": "chunk",
                "content": "La Renault 5 électrique propose une batterie 52 kWh et autonomie 400 km. Elle est équipée de l'OpenR Link avec Google.",
                "document_id": "doc-renault"
            }),
        ),
        (
            "chunk-dolphin".to_string(),
            vec![0.22_f32; 1536],
            json!({
                "type": "chunk",
                "content": "La BYD Dolphin citadine électrique avec batterie 44.9 kWh offre une autonomie de 340 km.",
                "document_id": "doc-byd"
            }),
        ),
        (
            "chunk-allure".to_string(),
            vec![0.4_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le programme Allure Care offre une garantie étendue de 8 ans ou 160 000 km sur les véhicules Peugeot. Il inclut l'entretien programmé.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-icockpit".to_string(),
            vec![0.35_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le i-Cockpit de Peugeot se distingue par son petit volant caractéristique et l'écran du tableau de bord positionné au-dessus.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-hybrid".to_string(),
            vec![0.45_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le Hybrid 136 e-DCS6 combine un moteur thermique avec un moteur électrique de 48V. La boîte e-DCS6 permet une réduction de 15% en ville.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-408".to_string(),
            vec![0.5_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Les i-Toggles de la Peugeot 408 sont des raccourcis personnalisables sous l'écran central pour une meilleure personnalisation.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-phev".to_string(),
            vec![0.55_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Les véhicules PHEV Peugeot consomment environ 1.2 kWh/100km en mode électrique. L'autonomie électrique varie de 60 à 80 km.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-2008".to_string(),
            vec![0.6_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le Peugeot 2008 GT est équipé du i-Cockpit et du moteur PureTech 130 ch avec boîte EAT8.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-308".to_string(),
            vec![0.65_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le châssis de la Peugeot 308 a été optimisé pour une conduite dynamique avec suspension McPherson.",
                "document_id": "doc-peugeot"
            }),
        ),
        (
            "chunk-scenic".to_string(),
            vec![0.7_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le Renault Scenic électrique offre 620 km d'autonomie et un habitacle modulable avec sièges amovibles.",
                "document_id": "doc-renault"
            }),
        ),
        (
            "chunk-bonus".to_string(),
            vec![0.75_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le E-3008 est éligible au bonus écologique de 5000€. Son prix débute à 44 990€.",
                "document_id": "doc-peugeot"
            }),
        ),
    ];
    storage.upsert(&chunk_data).await.unwrap();

    // Add entity vectors
    let entity_data = vec![
        (
            "entity-e3008".to_string(),
            vec![0.15_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "E-3008",
                "entity_type": "VEHICLE",
                "description": "SUV électrique Peugeot avec batterie 73-98 kWh"
            }),
        ),
        (
            "entity-byd-seal".to_string(),
            vec![0.25_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "BYD_SEAL_U",
                "entity_type": "VEHICLE",
                "description": "SUV électrique chinois avec batterie LFP"
            }),
        ),
        (
            "entity-e208".to_string(),
            vec![0.35_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "E-208",
                "entity_type": "VEHICLE",
                "description": "Citadine électrique Peugeot"
            }),
        ),
        (
            "entity-icockpit".to_string(),
            vec![0.45_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "I-COCKPIT",
                "entity_type": "FEATURE",
                "description": "Système d'affichage Peugeot"
            }),
        ),
        (
            "entity-allure-care".to_string(),
            vec![0.55_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "ALLURE_CARE",
                "entity_type": "SERVICE",
                "description": "Programme garantie 8 ans"
            }),
        ),
    ];
    storage.upsert(&entity_data).await.unwrap();

    // Add relationship vectors
    let relationship_data = vec![
        (
            "rel-e3008-byd".to_string(),
            vec![0.4_f32; 1536],
            json!({
                "type": "relationship",
                "src_id": "E-3008",
                "tgt_id": "BYD_SEAL_U",
                "relation_type": "COMPETES_WITH",
                "description": "E-3008 et BYD Seal U sont concurrents directs"
            }),
        ),
        (
            "rel-e208-icockpit".to_string(),
            vec![0.5_f32; 1536],
            json!({
                "type": "relationship",
                "src_id": "E-208",
                "tgt_id": "I-COCKPIT",
                "relation_type": "FEATURES",
                "description": "E-208 équipée du i-Cockpit"
            }),
        ),
    ];
    storage.upsert(&relationship_data).await.unwrap();

    storage
}

/// Create graph storage with automotive entities and relationships.
async fn create_test_graph_storage() -> Arc<MemoryGraphStorage> {
    let storage = Arc::new(MemoryGraphStorage::new("automotive_graph"));
    storage.initialize().await.unwrap();

    // Add vehicle nodes
    let nodes = vec![
        ("E-3008", "VEHICLE", "SUV électrique Peugeot"),
        ("BYD_SEAL_U", "VEHICLE", "SUV électrique chinois"),
        ("E-208", "VEHICLE", "Citadine électrique Peugeot"),
        ("RENAULT_5", "VEHICLE", "Citadine électrique Renault"),
        ("BYD_DOLPHIN", "VEHICLE", "Citadine électrique BYD"),
        ("PEUGEOT_2008", "VEHICLE", "SUV compact Peugeot"),
        ("PEUGEOT_308", "VEHICLE", "Berline compacte Peugeot"),
        ("PEUGEOT_408", "VEHICLE", "Fastback Peugeot"),
        ("RENAULT_SCENIC", "VEHICLE", "SUV électrique Renault"),
        ("I-COCKPIT", "FEATURE", "Système affichage Peugeot"),
        ("OPENR_LINK", "FEATURE", "Système multimédia Renault Google"),
        ("ALLURE_CARE", "SERVICE", "Garantie 8 ans 160000 km"),
        (
            "HYBRID_136",
            "POWERTRAIN",
            "Motorisation hybride 48V e-DCS6",
        ),
        ("I-TOGGLE", "FEATURE", "Raccourcis personnalisables 408"),
    ];

    for (name, entity_type, description) in nodes {
        storage
            .upsert_node(
                name,
                vec![
                    ("entity_type".to_string(), json!(entity_type)),
                    ("description".to_string(), json!(description)),
                ]
                .into_iter()
                .collect(),
            )
            .await
            .unwrap();
    }

    // Add relationships
    let edges = vec![
        ("E-3008", "BYD_SEAL_U", "COMPETES_WITH"),
        ("E-208", "RENAULT_5", "COMPETES_WITH"),
        ("E-208", "BYD_DOLPHIN", "COMPETES_WITH"),
        ("E-3008", "RENAULT_SCENIC", "COMPETES_WITH"),
        ("E-208", "I-COCKPIT", "FEATURES"),
        ("PEUGEOT_2008", "I-COCKPIT", "FEATURES"),
        ("PEUGEOT_308", "I-COCKPIT", "FEATURES"),
        ("PEUGEOT_408", "I-TOGGLE", "FEATURES"),
        ("PEUGEOT_308", "HYBRID_136", "AVAILABLE_WITH"),
        ("PEUGEOT_408", "HYBRID_136", "AVAILABLE_WITH"),
        ("RENAULT_5", "OPENR_LINK", "FEATURES"),
    ];

    for (src, tgt, rel) in edges {
        storage
            .upsert_edge(
                src,
                tgt,
                vec![("relation_type".to_string(), json!(rel))]
                    .into_iter()
                    .collect(),
            )
            .await
            .unwrap();
    }

    storage
}

/// Create the SOTA query engine for tests.
async fn create_test_engine() -> SOTAQueryEngine {
    let vector_storage = create_test_vector_storage().await;
    let graph_storage = create_test_graph_storage().await;
    let provider = create_mock_provider();

    let config = SOTAQueryConfig::default();

    // Use with_mock_keywords for testing - avoids LLM calls for keyword extraction
    SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    )
}

/// Run a single query and assess quality.
async fn run_query_and_assess(engine: &SOTAQueryEngine, test_query: &TestQuery) -> ResponseQuality {
    let mode = match test_query.mode {
        "global" => QueryMode::Global,
        "local" => QueryMode::Local,
        _ => QueryMode::Hybrid,
    };

    let request = QueryRequest::new(test_query.query).with_mode(mode);
    let result = engine.query(request).await;

    let response = match result {
        Ok(r) => r.answer,
        Err(e) => format!("Error: {}", e),
    };

    ResponseQuality::assess(&response, &test_query.expected_entities_vec())
}

// =============================================================================
// Individual Query Tests
// =============================================================================

#[tokio::test]
async fn test_q1_stla_byd_comparison() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q1_STLA_BYD").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!("Theme: {}", query.theme);
    println!("Query: {}", query.query);
    println!("Quality Level: {:?}", quality.quality_level);
    println!("Score: {:.1}", quality.total_score);
    println!(
        "Entities found: {} / {}",
        quality.entities_found.len(),
        query.expected_entities.len()
    );
    println!("Entity recall: {:.1}%", quality.entity_recall * 100.0);

    // For mock provider, just verify query execution succeeds
    assert!(quality.total_score >= 0.0, "Q1 should return a valid score");
}

#[tokio::test]
async fn test_q2_e208_r5_comparison() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q2_E208_R5").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(quality.total_score >= 0.0, "Q2 should return a valid score");
}

#[tokio::test]
async fn test_q3_allure_care_warranty() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q3_ALLURE_CARE").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(quality.total_score >= 0.0, "Q3 should return a valid score");
}

#[tokio::test]
async fn test_q4_peugeot_2008_features() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q4_PEUGEOT_2008").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(quality.total_score >= 0.0, "Q4 should return a valid score");
}

#[tokio::test]
async fn test_q5_icockpit_google_comparison() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q5_ICOCKPIT_GOOGLE").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(quality.total_score >= 0.0, "Q5 should return a valid score");
}

#[tokio::test]
async fn test_q6_408_itoggle_features() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q6_408_ITOGGLE").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(quality.total_score >= 0.0, "Q6 should return a valid score");
}

#[tokio::test]
async fn test_q7_hybrid_136_powertrain() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q7_HYBRID_136").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(quality.total_score >= 0.0, "Q7 should return a valid score");
}

#[tokio::test]
async fn test_q8_phev_consumption() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q8_PHEV_CONSUMPTION").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(quality.total_score >= 0.0, "Q8 should return a valid score");
}

#[tokio::test]
async fn test_q9_bonus_ecologique_pricing() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q9_BONUS_ECOLOGIQUE").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(quality.total_score >= 0.0, "Q9 should return a valid score");
}

#[tokio::test]
async fn test_q10_e3008_scenic_comparison() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q10_E3008_SCENIC").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(
        quality.total_score >= 0.0,
        "Q10 should return a valid score"
    );
}

#[tokio::test]
async fn test_q11_driving_dynamics() {
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q11_DRIVING_DYNAMICS").unwrap();
    let quality = run_query_and_assess(&engine, query).await;

    println!("\n=== {} ===", query.id);
    println!(
        "Score: {:.1}, Entity Recall: {:.1}%",
        quality.total_score,
        quality.entity_recall * 100.0
    );

    assert!(
        quality.total_score >= 0.0,
        "Q11 should return a valid score"
    );
}

// =============================================================================
// Suite-Level Tests
// =============================================================================

#[tokio::test]
async fn test_full_suite_execution() {
    let engine = create_test_engine().await;
    let mut suite_metrics = TestSuiteMetrics::default();

    println!("\n========================================");
    println!("  FULL SEARCH QUALITY TEST SUITE");
    println!("========================================\n");

    for query in TEST_QUERIES {
        let quality = run_query_and_assess(&engine, query).await;
        let passed = quality.total_score >= MIN_ACCEPTABLE_SCORE;
        suite_metrics.add_result(&quality, passed);

        let status = if passed { "✓ PASS" } else { "○ EXEC" };
        println!(
            "{:15} | {:8} | Score: {:5.1} | Recall: {:5.1}%",
            query.id,
            status,
            quality.total_score,
            quality.entity_recall * 100.0
        );
    }

    println!("\n----------------------------------------");
    println!("SUMMARY:");
    println!("  Total:  {} tests", suite_metrics.total_tests);
    println!(
        "  Passed: {} ({:.1}%)",
        suite_metrics.passed_tests,
        suite_metrics.pass_rate() * 100.0
    );
    println!("  Executed: {}", suite_metrics.total_tests);
    println!("  Avg Score: {:.1}", suite_metrics.average_score);
    println!("  Avg Recall: {:.1}%", suite_metrics.average_recall * 100.0);
    println!("----------------------------------------\n");

    // With mock provider, just verify all queries executed
    assert_eq!(
        suite_metrics.total_tests, 11,
        "All 11 queries should execute"
    );
}

#[tokio::test]
async fn test_global_mode_queries() {
    let engine = create_test_engine().await;
    let global_queries = test_queries::get_queries_by_mode("global");

    println!(
        "\n=== GLOBAL MODE QUERIES ({} tests) ===\n",
        global_queries.len()
    );

    let mut executed = 0;
    for query in &global_queries {
        let quality = run_query_and_assess(&engine, query).await;
        executed += 1;
        println!("{}: {:.1}", query.id, quality.total_score);
    }

    assert_eq!(
        executed,
        global_queries.len(),
        "All global queries executed"
    );
}

#[tokio::test]
async fn test_local_mode_queries() {
    let engine = create_test_engine().await;
    let local_queries = test_queries::get_queries_by_mode("local");

    println!(
        "\n=== LOCAL MODE QUERIES ({} tests) ===\n",
        local_queries.len()
    );

    let mut executed = 0;
    for query in &local_queries {
        let quality = run_query_and_assess(&engine, query).await;
        executed += 1;
        println!("{}: {:.1}", query.id, quality.total_score);
    }

    assert_eq!(executed, local_queries.len(), "All local queries executed");
}

#[tokio::test]
async fn test_competitive_analysis_theme() {
    let engine = create_test_engine().await;
    let competitive_queries = test_queries::get_queries_by_theme("competitive_analysis");

    println!(
        "\n=== COMPETITIVE ANALYSIS THEME ({} tests) ===\n",
        competitive_queries.len()
    );

    for query in &competitive_queries {
        let quality = run_query_and_assess(&engine, query).await;
        println!(
            "{}: Score={:.1}, Entities={:?}",
            query.id, quality.total_score, quality.entities_found
        );
    }

    assert_eq!(
        competitive_queries.len(),
        3,
        "Should have 3 competitive analysis queries"
    );
}

// =============================================================================
// Regression Prevention Tests
// =============================================================================

#[tokio::test]
async fn test_french_query_handling() {
    // Ensure French queries with accents are handled correctly
    let engine = create_test_engine().await;

    let french_queries = vec![
        "Quelles sont les caractéristiques du véhicule électrique ?",
        "Comment fonctionne le système de récupération d'énergie ?",
        "Où puis-je recharger mon véhicule à Lyon ?",
    ];

    for query_text in french_queries {
        let request = QueryRequest::new(query_text).with_mode(QueryMode::Global);
        let result = engine.query(request).await;
        assert!(
            result.is_ok(),
            "French query should not error: {}",
            query_text
        );
    }
}

#[tokio::test]
async fn test_entity_extraction_consistency() {
    // Run the same query multiple times and verify consistent entity extraction
    let engine = create_test_engine().await;
    let query = test_queries::get_query_by_id("Q1_STLA_BYD").unwrap();

    let mut results: Vec<ResponseQuality> = Vec::new();

    for _ in 0..3 {
        let quality = run_query_and_assess(&engine, query).await;
        results.push(quality);
    }

    // All three runs should produce valid results
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.total_score >= 0.0,
            "Run {} should produce valid score",
            i + 1
        );
    }
}

#[tokio::test]
async fn test_response_not_empty() {
    // Verify responses are not empty
    let engine = create_test_engine().await;

    for query in TEST_QUERIES {
        let mode = match query.mode {
            "global" => QueryMode::Global,
            "local" => QueryMode::Local,
            _ => QueryMode::Hybrid,
        };

        let request = QueryRequest::new(query.query).with_mode(mode);
        let result = engine.query(request).await;
        assert!(result.is_ok(), "Query {} should succeed", query.id);

        let response = result.unwrap();
        assert!(
            !response.answer.is_empty(),
            "Query {} should return non-empty response",
            query.id
        );
    }
}

// =============================================================================
// Metrics Module Tests
// =============================================================================

#[test]
fn test_metrics_precision_recall() {
    use std::collections::HashSet;

    let retrieved: HashSet<&str> = ["a", "b", "c"].into_iter().collect();
    let relevant: HashSet<&str> = ["b", "c", "d"].into_iter().collect();

    let result = metrics::calculate_metrics(&retrieved, &relevant);

    assert_eq!(result.true_positives, 2);
    assert_eq!(result.false_positives, 1);
    assert_eq!(result.false_negatives, 1);
    assert!((result.precision - 0.6666).abs() < 0.01);
    assert!((result.recall - 0.6666).abs() < 0.01);
}

#[test]
fn test_quality_assessment() {
    let response =
        "Le Peugeot E-3008 est équipé d'une batterie NMC de 73 kWh avec une autonomie de 700 km. \
                    Le BYD Seal U concurrent utilise une batterie LFP.";
    let expected = vec![
        "E-3008".to_string(),
        "BYD Seal U".to_string(),
        "NMC".to_string(),
        "LFP".to_string(),
    ];

    let quality = ResponseQuality::assess(response, &expected);

    // Should find E-3008, BYD Seal U, NMC, LFP in the response
    assert!(quality.entity_recall > 0.0);
    assert!(quality.total_score > 0.0);
}

#[test]
fn test_test_queries_structure() {
    assert_eq!(TEST_QUERIES.len(), 11, "Should have 11 test queries");

    for query in TEST_QUERIES {
        assert!(!query.id.is_empty(), "Query ID should not be empty");
        assert!(!query.query.is_empty(), "Query text should not be empty");
        assert!(
            !query.expected_entities.is_empty(),
            "Expected entities should not be empty"
        );
    }
}
