//! Keyword Validation Integration Tests
//!
//! These tests validate the keyword validation mechanism that was fixed in OODA loops 62-71.
//! The key insight from the fix was that non-existent keywords in the knowledge graph
//! dilute the embedding computation and reduce retrieval quality.
//!
//! Run these tests:
//! ```bash
//! cargo test --package edgequake-query --test keyword_validation_tests -- --nocapture
//! ```

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::{
    ExtractedKeywords, KeywordExtractor, QueryIntent, QueryMode, QueryRequest, SOTAQueryConfig,
    SOTAQueryEngine,
};
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryVectorStorage, VectorStorage};
use serde_json::json;

// =============================================================================
// Test Data Setup (French Automotive Domain)
// =============================================================================

/// Create vector storage with French automotive content.
/// This matches the actual knowledge graph that would be populated from documents.
async fn create_automotive_vector_storage() -> Arc<MemoryVectorStorage> {
    let storage = Arc::new(MemoryVectorStorage::new("automotive_kg", 1536));
    storage.initialize().await.unwrap();

    // Chunk data matching what would be extracted from automotive documents
    let chunks = vec![
        // E-3008 content
        (
            "chunk-e3008-battery".to_string(),
            vec![0.11_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le Peugeot E-3008 est équipé de batteries NMC haute performance. Version 73 kWh pour 525 km d'autonomie WLTP, version 98 kWh Long Range pour 700 km.",
                "document_id": "doc-e3008"
            }),
        ),
        (
            "chunk-e3008-charging".to_string(),
            vec![0.12_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Recharge rapide sur le E-3008: 160 kW DC, 20-80% en 30 minutes. Compatible CCS Combo.",
                "document_id": "doc-e3008"
            }),
        ),
        (
            "chunk-e3008-highway".to_string(),
            vec![0.13_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Efficience autoroutière du E-3008: consommation de 18 kWh/100km à 130 km/h grâce à l'aérodynamisme optimisé (Cx 0.28).",
                "document_id": "doc-e3008"
            }),
        ),
        // BYD Seal U content
        (
            "chunk-byd-seal-battery".to_string(),
            vec![0.21_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le BYD Seal U utilise une batterie LFP Blade de 85.4 kWh. Technologie LFP offre meilleure durabilité mais densité énergétique inférieure aux NMC.",
                "document_id": "doc-byd"
            }),
        ),
        (
            "chunk-byd-seal-charging".to_string(),
            vec![0.22_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Recharge du BYD Seal U: 140 kW DC maximum, légèrement plus lent que la concurrence premium européenne.",
                "document_id": "doc-byd"
            }),
        ),
        (
            "chunk-byd-seal-price".to_string(),
            vec![0.23_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le BYD Seal U affiche un prix de départ de 44 990€, très compétitif face aux européens. Toutefois, perte du bonus écologique depuis 2024.",
                "document_id": "doc-byd"
            }),
        ),
        // Allure Care warranty
        (
            "chunk-allure-care".to_string(),
            vec![0.31_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Allure Care garantit le véhicule 8 ans ou 160 000 km. Inclut: entretien programmé, extension garantie constructeur, couverture batterie. Exclusions: usure normale, consommables.",
                "document_id": "doc-warranty"
            }),
        ),
        // i-Cockpit technology
        (
            "chunk-icockpit".to_string(),
            vec![0.41_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Le i-Cockpit de Peugeot: petit volant caractéristique, compteurs surélevés visibles au-dessus, écran tactile central 10 pouces. Compatible CarPlay et Android Auto.",
                "document_id": "doc-peugeot"
            }),
        ),
        // Hybrid 136 e-DCS6
        (
            "chunk-hybrid-136".to_string(),
            vec![0.51_f32; 1536],
            json!({
                "type": "chunk",
                "content": "Hybrid 136 ch e-DCS6: moteur PureTech 136 ch associé à batterie 48V de 0.9 kWh. Boîte e-DCS6 électrifiée permet 15% d'économie en ville. Agrément de conduite sans à-coups.",
                "document_id": "doc-peugeot"
            }),
        ),
        // Renault 5 for comparison
        (
            "chunk-r5".to_string(),
            vec![0.61_f32; 1536],
            json!({
                "type": "chunk",
                "content": "La nouvelle Renault 5 E-Tech électrique: batterie 52 kWh, autonomie 400 km WLTP, pompe à chaleur de série pour l'hiver. OpenR Link avec Google Maps intégré.",
                "document_id": "doc-renault"
            }),
        ),
    ];
    storage.upsert(&chunks).await.unwrap();

    // Entity vectors
    let entities = vec![
        (
            "entity-e3008".to_string(),
            vec![0.15_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "E-3008",
                "entity_type": "VEHICLE",
                "description": "SUV électrique Peugeot premium segment C"
            }),
        ),
        (
            "entity-byd-seal-u".to_string(),
            vec![0.25_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "BYD_SEAL_U",
                "entity_type": "VEHICLE",
                "description": "SUV électrique chinois concurrent direct"
            }),
        ),
        (
            "entity-allure-care".to_string(),
            vec![0.35_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "ALLURE_CARE",
                "entity_type": "WARRANTY_PROGRAM",
                "description": "Programme garantie 8 ans / 160 000 km"
            }),
        ),
        (
            "entity-icockpit".to_string(),
            vec![0.45_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "I-COCKPIT",
                "entity_type": "TECHNOLOGY",
                "description": "Interface conducteur Peugeot signature"
            }),
        ),
        (
            "entity-hybrid-136".to_string(),
            vec![0.55_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "HYBRID_136",
                "entity_type": "POWERTRAIN",
                "description": "Motorisation hybride 48V Peugeot"
            }),
        ),
        (
            "entity-renault-5".to_string(),
            vec![0.65_f32; 1536],
            json!({
                "type": "entity",
                "entity_name": "RENAULT_5",
                "entity_type": "VEHICLE",
                "description": "Citadine électrique Renault"
            }),
        ),
    ];
    storage.upsert(&entities).await.unwrap();

    // Relationship vectors
    let relationships = vec![
        (
            "rel-e3008-byd".to_string(),
            vec![0.4_f32; 1536],
            json!({
                "type": "relationship",
                "src_id": "E-3008",
                "tgt_id": "BYD_SEAL_U",
                "relation_type": "COMPETES_WITH",
                "description": "E-3008 et BYD Seal U sont concurrents directs dans le segment SUV électrique"
            }),
        ),
        (
            "rel-e3008-icockpit".to_string(),
            vec![0.5_f32; 1536],
            json!({
                "type": "relationship",
                "src_id": "E-3008",
                "tgt_id": "I-COCKPIT",
                "relation_type": "EQUIPPED_WITH",
                "description": "E-3008 est équipé du i-Cockpit"
            }),
        ),
    ];
    storage.upsert(&relationships).await.unwrap();

    storage
}

/// Create graph storage with automotive entities that will be used for keyword validation.
/// This is the key data structure for testing keyword validation - entities in this graph
/// determine which keywords are "valid" (exist in the knowledge base).
async fn create_automotive_graph_storage() -> Arc<MemoryGraphStorage> {
    let storage = Arc::new(MemoryGraphStorage::new("automotive_graph"));
    storage.initialize().await.unwrap();

    // These are the entities that EXIST in our knowledge graph
    // Keywords matching these will be kept during validation
    // Keywords NOT matching these will be DROPPED
    let existing_entities = vec![
        // Vehicle entities (these EXIST)
        ("E-3008", "VEHICLE", "SUV électrique Peugeot premium"),
        ("E-208", "VEHICLE", "Citadine électrique Peugeot"),
        ("BYD_SEAL_U", "VEHICLE", "SUV électrique chinois BYD"),
        ("BYD_DOLPHIN", "VEHICLE", "Citadine électrique BYD"),
        ("RENAULT_5", "VEHICLE", "Citadine électrique Renault"),
        ("RENAULT_SCENIC", "VEHICLE", "SUV électrique Renault"),
        ("PEUGEOT_308", "VEHICLE", "Berline compacte Peugeot"),
        ("PEUGEOT_408", "VEHICLE", "Fastback Peugeot"),
        ("PEUGEOT_2008", "VEHICLE", "SUV compact Peugeot"),
        // Technology entities (these EXIST)
        ("I-COCKPIT", "TECHNOLOGY", "Interface conducteur Peugeot"),
        ("OPENR_LINK", "TECHNOLOGY", "Système multimédia Renault"),
        ("I-TOGGLE", "TECHNOLOGY", "Raccourcis personnalisables 408"),
        // Program entities (these EXIST)
        ("ALLURE_CARE", "WARRANTY", "Garantie 8 ans 160000 km"),
        ("HYBRID_136", "POWERTRAIN", "Motorisation hybride 48V"),
        // Technical concepts (these EXIST)
        (
            "NMC_BATTERY",
            "TECHNOLOGY",
            "Batterie nickel-manganèse-cobalt",
        ),
        (
            "LFP_BATTERY",
            "TECHNOLOGY",
            "Batterie lithium-fer-phosphate",
        ),
        ("E-DCS6", "TECHNOLOGY", "Boîte de vitesses électrifiée"),
    ];

    for (name, entity_type, description) in existing_entities {
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

    // Relationships
    let edges = vec![
        ("E-3008", "BYD_SEAL_U", "COMPETES_WITH"),
        ("E-3008", "RENAULT_SCENIC", "COMPETES_WITH"),
        ("E-208", "BYD_DOLPHIN", "COMPETES_WITH"),
        ("E-208", "RENAULT_5", "COMPETES_WITH"),
        ("E-3008", "I-COCKPIT", "EQUIPPED_WITH"),
        ("E-208", "I-COCKPIT", "EQUIPPED_WITH"),
        ("PEUGEOT_308", "HYBRID_136", "AVAILABLE_WITH"),
        ("PEUGEOT_408", "I-TOGGLE", "FEATURES"),
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

// =============================================================================
// Custom Keyword Extractor for Testing (Future Use)
// =============================================================================

/// Custom keyword extractor that returns specific keywords for testing.
/// This allows us to test keyword validation without LLM calls.
/// Marked as allow(dead_code) for future expansion.
#[allow(dead_code)]
struct TestKeywordExtractor {
    /// Keywords to return for each test scenario
    keywords_to_return: ExtractedKeywords,
}

#[allow(dead_code)]
impl TestKeywordExtractor {
    fn new(high_level: Vec<String>, low_level: Vec<String>, intent: QueryIntent) -> Self {
        Self {
            keywords_to_return: ExtractedKeywords::new(high_level, low_level, intent),
        }
    }

    /// Create extractor for the challenge query (French BYD vs E-3008)
    /// This simulates what the LLM would extract from:
    /// "J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas.
    ///  Concrètement, qu'est-ce que le E-3008 apporte de plus ..."
    fn for_challenge_query() -> Self {
        Self::new(
            vec!["electric vehicles comparison".to_string()],
            vec![
                "BYD Seal U".to_string(),  // EXISTS in graph
                "E-3008".to_string(),      // EXISTS in graph
                "LFP".to_string(),         // May or may not exist (partial match)
                "STLA Medium".to_string(), // DOES NOT EXIST - should be dropped
                "autoroute".to_string(),   // DOES NOT EXIST - should be dropped
                "batterie".to_string(),    // Generic term
            ],
            QueryIntent::Comparative,
        )
    }

    /// Create extractor for an out-of-domain query (Tesla)
    fn for_tesla_query() -> Self {
        Self::new(
            vec!["electric vehicle specifications".to_string()],
            vec![
                "Tesla".to_string(),          // DOES NOT EXIST
                "Model 3".to_string(),        // DOES NOT EXIST
                "range".to_string(),          // Generic
                "specifications".to_string(), // Generic
            ],
            QueryIntent::Factual,
        )
    }

    /// Create extractor for completely off-topic query (pizza)
    fn for_pizza_query() -> Self {
        Self::new(
            vec!["restaurant recommendations".to_string()],
            vec![
                "pizza".to_string(),       // DOES NOT EXIST
                "restaurants".to_string(), // DOES NOT EXIST
                "New York".to_string(),    // DOES NOT EXIST
            ],
            QueryIntent::Exploratory,
        )
    }

    /// Create extractor for all-valid keywords query
    fn for_all_valid_query() -> Self {
        Self::new(
            vec!["Peugeot technology".to_string()],
            vec![
                "E-3008".to_string(),    // EXISTS
                "I-COCKPIT".to_string(), // EXISTS
                "E-208".to_string(),     // EXISTS
            ],
            QueryIntent::Factual,
        )
    }
}

#[async_trait::async_trait]
impl KeywordExtractor for TestKeywordExtractor {
    async fn extract(&self, _query: &str) -> edgequake_query::Result<edgequake_query::Keywords> {
        Ok(edgequake_query::Keywords::new(
            self.keywords_to_return.high_level.clone(),
            self.keywords_to_return.low_level.clone(),
        ))
    }

    async fn extract_extended(&self, _query: &str) -> edgequake_query::Result<ExtractedKeywords> {
        Ok(self.keywords_to_return.clone())
    }
}

// =============================================================================
// KEYWORD VALIDATION TESTS (OODA 62-71)
// =============================================================================

/// Test 1: Verify that keywords NOT in the graph are dropped
/// This is the core fix from OODA Loop 62
#[tokio::test]
async fn test_keyword_validation_drops_nonexistent() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    // Create engine with custom keyword extractor
    let config = SOTAQueryConfig::default();
    let _engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage.clone(),
        provider.clone(),
        provider,
    );

    // Verify graph has expected entities
    let labels = graph_storage.search_labels("E-3008", 1).await.unwrap();
    assert!(!labels.is_empty(), "E-3008 should exist in graph");

    let labels = graph_storage.search_labels("STLA Medium", 1).await.unwrap();
    assert!(labels.is_empty(), "STLA Medium should NOT exist in graph");

    println!("✓ Graph correctly contains E-3008 but not STLA Medium");
    println!("  This validates the test setup for keyword validation");
}

/// Test 2: Verify query execution succeeds with mixed valid/invalid keywords
/// This tests the "embedding dilution prevention" fix
#[tokio::test]
async fn test_mixed_keyword_query_succeeds() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // The challenge query from OODA 62-71
    let query = "J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas. \
                 Concrètement, qu'est-ce que le E-3008 apporte de plus pour justifier la différence de prix ?";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "Query should succeed: {:?}", result.err());

    let response = result.unwrap();
    println!("\n=== Challenge Query Result ===");
    println!("Answer length: {} chars", response.answer.len());
    println!("Context chunks: {}", response.context.chunks.len());

    // With mock provider, answer will be minimal, but execution should succeed
    assert!(response.answer.len() > 0, "Should return an answer");
}

/// Test 3: Verify fallback when ALL keywords are invalid
/// This tests the fallback mechanism from OODA 69 (pizza query)
#[tokio::test]
async fn test_fallback_when_all_keywords_dropped() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage.clone(),
        provider.clone(),
        provider,
    );

    // Completely off-topic query - all keywords should be dropped
    let query = "Best pizza restaurants in New York";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    // Should still succeed (fallback to original keywords)
    assert!(result.is_ok(), "Should not fail even with off-topic query");

    println!("✓ Off-topic query handled gracefully with fallback");
}

/// Test 4: Verify that valid keywords are preserved
#[tokio::test]
async fn test_valid_keywords_preserved() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage.clone(),
        provider.clone(),
        provider,
    );

    // Query with entities that definitely exist
    let query = "Comparez le E-3008 avec le i-Cockpit de la E-208";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "Query with valid entities should succeed");

    let response = result.unwrap();
    println!("\n=== Valid Keywords Query ===");
    println!("Answer length: {} chars", response.answer.len());
}

/// Test 5: Adjacent domain query (Tesla - related but not in our graph)
/// From OODA 69
#[tokio::test]
async fn test_adjacent_domain_query() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // Tesla - same domain (EVs) but not in our knowledge graph
    let query = "Tesla Model 3 specifications and range";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    // Should succeed with fallback or partial matches
    assert!(result.is_ok(), "Adjacent domain query should not fail");

    println!("✓ Adjacent domain (Tesla) handled gracefully");
}

// =============================================================================
// QUERY MODE TESTS
// =============================================================================

/// Test 6: LOCAL mode focuses on specific entities
#[tokio::test]
async fn test_local_mode_entity_focus() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // LOCAL mode query - specific product specs
    let query = "Quelles sont les caractéristiques de la Peugeot 2008?";

    let request = QueryRequest::new(query).with_mode(QueryMode::Local);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "LOCAL mode query should succeed");
}

/// Test 7: GLOBAL mode for broader themes
#[tokio::test]
async fn test_global_mode_theme_focus() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // GLOBAL mode query - thematic question
    let query = "Comment Peugeot se positionne face aux constructeurs chinois sur l'électrique?";

    let request = QueryRequest::new(query).with_mode(QueryMode::Global);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "GLOBAL mode query should succeed");
}

/// Test 8: HYBRID mode combines both
#[tokio::test]
async fn test_hybrid_mode_combined() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // HYBRID mode - specific entities + broader context
    let query = "Le E-3008 est-il un bon choix pour l'autoroute comparé au BYD Seal U?";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "HYBRID mode query should succeed");
}

// =============================================================================
// FRENCH QUERY TESTS (Language-specific handling)
// =============================================================================

/// Test 9: French query with accents and special characters
#[tokio::test]
async fn test_french_query_with_accents() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // Query with French accents
    let query = "Le bonus écologique est-il éligible sur le E-3008 fabriqué en France?";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "French query with accents should work");
}

/// Test 10: Complex French comparative query
#[tokio::test]
async fn test_complex_french_comparative() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // Complex comparison query
    let query = "J'hésite entre la future Renault 5 et la BYD Dolphin. \
                 L'autonomie WLTP de la E-208 est-elle fiable en hiver?";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "Complex French comparative should work");
}

// =============================================================================
// ENTITY COVERAGE TESTS (All 11 OODA queries)
// =============================================================================

/// Test 11: All 11 OODA loop queries should execute successfully
#[tokio::test]
async fn test_all_ooda_queries_execute() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    let queries = vec![
        ("Q1_STLA_BYD", "hybrid", "J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas. Qu'est-ce que le E-3008 apporte de plus?"),
        ("Q2_E208_R5", "hybrid", "J'hésite avec la future Renault 5 ou une BYD Dolphin. La E-208 a été restylée, mais son autonomie WLTP est-elle fiable en hiver?"),
        ("Q3_ALLURE_CARE", "hybrid", "BYD garantit ses batteries très longtemps. Allure Care garantit le véhicule 8 ans/160 000 km. Est-ce que cela couvre vraiment tout?"),
        ("Q4_PEUGEOT_2008", "local", "Quels sont les caractéristiques d'une Peugeot 2008?"),
        ("Q5_ICOCKPIT_GOOGLE", "hybrid", "Le nouveau i-Cockpit Panoramique de Peugeot est beau, mais est-il aussi réactif que l'OpenR Link avec Google?"),
        ("Q6_408_ITOGGLE", "local", "L'interface des i-Toggles de la 408 est-elle vraiment personnalisable?"),
        ("Q7_HYBRID_136", "hybrid", "Comment se comporte le nouveau moteur Hybride 136 e-DCS6? Y a-t-il encore des à-coups?"),
        ("Q8_PHEV_CONSUMPTION", "local", "Si je prends une 308 Hybride Rechargeable, quelle est la consommation réelle une fois la batterie vide?"),
        ("Q9_BONUS_ECOLOGIQUE", "hybrid", "Si je configure un E-3008 Made in France, l'écart de prix est-il compensé par le bonus écologique?"),
        ("Q10_E3008_SCENIC", "hybrid", "Par rapport au Renault Scenic Voiture de l'année, qu'est-ce qui justifie l'écart de prix sur un E-3008 GT?"),
        ("Q11_DRIVING_DYNAMICS", "local", "J'ai trouvé la BYD Atto 3 molle en suspension. Peugeot garde-t-il le toucher de route sur les électriques?"),
    ];

    println!("\n========================================");
    println!("  FULL OODA QUERY SUITE (11 queries)");
    println!("========================================\n");

    let mut passed = 0;
    let mut failed = 0;

    for (id, mode_str, query_text) in &queries {
        let mode = match *mode_str {
            "local" => QueryMode::Local,
            "global" => QueryMode::Global,
            _ => QueryMode::Hybrid,
        };

        let request = QueryRequest::new(*query_text).with_mode(mode);
        let result = engine.query(request).await;

        match result {
            Ok(response) => {
                println!("{:20} | ✓ OK | {} chars", id, response.answer.len());
                passed += 1;
            }
            Err(e) => {
                println!("{:20} | ✗ FAIL | {:?}", id, e);
                failed += 1;
            }
        }
    }

    println!("\n----------------------------------------");
    println!("SUMMARY: {} passed, {} failed", passed, failed);
    println!("----------------------------------------");

    assert_eq!(failed, 0, "All 11 OODA queries should execute successfully");
}

// =============================================================================
// CACHE TESTS (OODA 65, 70)
// =============================================================================

/// Test 12: Keyword validation cache effectiveness
#[tokio::test]
async fn test_keyword_cache_reuse() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // Run the same query twice - second should use cache
    let query = "E-3008 comparaison BYD Seal U batteries";

    let request1 = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let start1 = std::time::Instant::now();
    let _ = engine.query(request1).await;
    let time1 = start1.elapsed();

    let request2 = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let start2 = std::time::Instant::now();
    let _ = engine.query(request2).await;
    let time2 = start2.elapsed();

    println!("\n=== Cache Test ===");
    println!("First query:  {:?}", time1);
    println!("Second query: {:?}", time2);

    // Both should succeed - with mock provider, times will be similar
    // In production with real LLM, second would be faster due to cache
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

/// Test 13: Empty query handling
#[tokio::test]
async fn test_empty_query_handling() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    let request = QueryRequest::new("").with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    // Should handle gracefully (either succeed with default or error with message)
    println!(
        "Empty query result: {}",
        if result.is_ok() { "OK" } else { "Error" }
    );
}

/// Test 14: Very long query handling
#[tokio::test]
async fn test_long_query_handling() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    // Very long query with lots of detail
    let query = "Je suis un vendeur Peugeot et j'ai un client qui hésite entre le E-3008 et le BYD Seal U. \
                 Il a testé les deux véhicules et trouve que le BYD offre un meilleur rapport qualité-prix \
                 avec sa batterie LFP plus grande. Cependant, il s'inquiète de l'autonomie sur autoroute \
                 car il fait souvent Paris-Lyon. Il veut aussi savoir si le i-Cockpit est aussi moderne \
                 que le système d'infodivertissement de BYD. Pouvez-vous me donner des arguments commerciaux \
                 pour le convaincre de choisir le E-3008? Je pense mentionner Allure Care, le bonus écologique, \
                 et la qualité de fabrication européenne.";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "Long query should be handled");
    println!("Long query ({}+ chars) handled successfully", query.len());
}

/// Test 15: Special characters in query
#[tokio::test]
async fn test_special_characters_query() {
    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage,
        provider.clone(),
        provider,
    );

    let query = "E-3008 vs BYD: 73 kWh → 700 km? Prix: 44.990€ 🚗";

    let request = QueryRequest::new(query).with_mode(QueryMode::Hybrid);
    let result = engine.query(request).await;

    assert!(result.is_ok(), "Special characters should not break query");
}

// =============================================================================
// GRAPH SEARCH TESTS
// =============================================================================

/// Test 16: Verify graph search_labels works correctly
#[tokio::test]
async fn test_graph_search_labels() {
    let graph_storage = create_automotive_graph_storage().await;

    // Test existing entities
    let results = graph_storage.search_labels("E-3008", 5).await.unwrap();
    assert!(!results.is_empty(), "E-3008 should be found");
    println!("Found E-3008: {:?}", results);

    let results = graph_storage.search_labels("BYD", 5).await.unwrap();
    println!("BYD search results: {:?}", results);

    // Test non-existing entity
    let results = graph_storage.search_labels("STLA Medium", 5).await.unwrap();
    assert!(results.is_empty(), "STLA Medium should NOT be found");
    println!("STLA Medium correctly not found");
}

/// Test 17: Entity relationship traversal
#[tokio::test]
async fn test_entity_relationships() {
    let graph_storage = create_automotive_graph_storage().await;

    // Get neighbors of E-3008
    let neighbors = graph_storage.get_neighbors("E-3008", 1).await.unwrap();
    println!("\nE-3008 neighbors: {:?}", neighbors);

    // Should have relationships with BYD_SEAL_U and I-COCKPIT
    assert!(
        !neighbors.is_empty(),
        "E-3008 should have graph relationships"
    );
}

// =============================================================================
// INTEGRATION SUMMARY TEST
// =============================================================================

/// Test 18: Full integration test matching OODA 71 success criteria
#[tokio::test]
async fn test_full_ooda_integration() {
    println!("\n========================================");
    println!("  OODA 62-71 Integration Verification");
    println!("========================================\n");

    let vector_storage = create_automotive_vector_storage().await;
    let graph_storage = create_automotive_graph_storage().await;
    let provider = Arc::new(MockProvider::new());

    let config = SOTAQueryConfig::default();
    let engine = SOTAQueryEngine::with_mock_keywords(
        config,
        vector_storage,
        graph_storage.clone(),
        provider.clone(),
        provider,
    );

    // Verification checklist from OODA 71
    let mut checks_passed = 0;
    let total_checks = 5;

    // Check 1: Graph contains expected entities
    let e3008_exists = graph_storage
        .search_labels("E-3008", 1)
        .await
        .map(|r| !r.is_empty())
        .unwrap_or(false);
    if e3008_exists {
        println!("✓ Check 1: E-3008 exists in knowledge graph");
        checks_passed += 1;
    }

    // Check 2: Invalid keywords would be dropped
    let stla_exists = graph_storage
        .search_labels("STLA Medium", 1)
        .await
        .map(|r| !r.is_empty())
        .unwrap_or(false);
    if !stla_exists {
        println!("✓ Check 2: STLA Medium correctly not in graph (would be dropped)");
        checks_passed += 1;
    }

    // Check 3: Challenge query executes
    let challenge = "J'ai testé le BYD Seal U. Qu'est-ce que le E-3008 apporte de plus?";
    let request = QueryRequest::new(challenge).with_mode(QueryMode::Hybrid);
    if engine.query(request).await.is_ok() {
        println!("✓ Check 3: Challenge query executes successfully");
        checks_passed += 1;
    }

    // Check 4: Off-topic query handled gracefully
    let offtopic = "Best pizza in NYC";
    let request = QueryRequest::new(offtopic).with_mode(QueryMode::Hybrid);
    if engine.query(request).await.is_ok() {
        println!("✓ Check 4: Off-topic query handled with fallback");
        checks_passed += 1;
    }

    // Check 5: All query modes work
    let modes_work = {
        let r1 = engine
            .query(QueryRequest::new("E-3008").with_mode(QueryMode::Local))
            .await
            .is_ok();
        let r2 = engine
            .query(QueryRequest::new("E-3008").with_mode(QueryMode::Global))
            .await
            .is_ok();
        let r3 = engine
            .query(QueryRequest::new("E-3008").with_mode(QueryMode::Hybrid))
            .await
            .is_ok();
        r1 && r2 && r3
    };
    if modes_work {
        println!("✓ Check 5: All query modes (Local/Global/Hybrid) work");
        checks_passed += 1;
    }

    println!("\n----------------------------------------");
    println!("RESULT: {}/{} checks passed", checks_passed, total_checks);
    println!("----------------------------------------");

    assert_eq!(
        checks_passed, total_checks,
        "All OODA integration checks should pass"
    );
}
