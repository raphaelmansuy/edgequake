//! API Integration Tests for Search Quality
//!
//! These tests verify the complete end-to-end flow by calling the actual API.
//! They are marked as `#[ignore]` by default because they require:
//! 1. The EdgeQuake API server to be running
//! 2. A populated knowledge graph
//! 3. LLM API access (OpenAI key)
//!
//! Run these tests with:
//! ```bash
//! # Start the API server first
//! make dev
//!
//! # Setup fresh workspace and run tests
//! cargo test --package edgequake-query --test api_integration_tests -- --ignored --nocapture
//! ```

mod test_fixtures;

use std::time::Duration;
use test_fixtures::{setup_fresh_workspace, SetupOptions};

/// API base URL (can be overridden with API_BASE_URL env var)
fn get_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Quality thresholds from OODA 63
const EXCELLENT_THRESHOLD: usize = 1000; // chars
const GOOD_THRESHOLD: usize = 500;
const PARTIAL_THRESHOLD: usize = 200;

/// Test query definition matching extended_challenge_query.py
#[derive(Debug)]
#[allow(dead_code)] // theme used for documentation/categorization
struct ApiTestQuery {
    id: &'static str,
    theme: &'static str,
    query: &'static str,
    mode: &'static str,
    expected_entities: Vec<&'static str>,
}

/// All 11 test queries from OODA 63
fn get_test_queries() -> Vec<ApiTestQuery> {
    vec![
        ApiTestQuery {
            id: "Q1_STLA_BYD",
            theme: "Electrification",
            query: "J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas. Concrètement, qu'est-ce que la plateforme STLA Medium du E-3008 m'apporte de plus en termes d'efficience réelle sur autoroute et de vitesse de recharge par rapport au chinois ?",
            mode: "hybrid",
            expected_entities: vec!["BYD Seal U", "E-3008", "STLA Medium", "LFP"],
        },
        ApiTestQuery {
            id: "Q2_E208_R5",
            theme: "Electrification",
            query: "J'hésite avec la future Renault 5 ou une BYD Dolphin. La E-208 a été restylée, mais son autonomie WLTP est-elle fiable en hiver par rapport à la pompe à chaleur de la Renault ?",
            mode: "hybrid",
            expected_entities: vec!["E-208", "Renault 5", "BYD Dolphin"],
        },
        ApiTestQuery {
            id: "Q3_ALLURE_CARE",
            theme: "Warranty",
            query: "BYD garantit ses batteries très longtemps. J'ai vu que Peugeot a lancé Allure Care garantissant le véhicule 8 ans/160 000 km. Est-ce que cela couvre vraiment tout comme chez Kia/Hyundai ou y a-t-il des exclusions majeures ?",
            mode: "hybrid",
            expected_entities: vec!["Allure Care", "BYD", "Peugeot"],
        },
        ApiTestQuery {
            id: "Q4_PEUGEOT_2008",
            theme: "Product",
            query: "Quels sont les caractéristiques d'une Peugeot 2008 ?",
            mode: "local",
            expected_entities: vec!["Peugeot 2008"],
        },
        ApiTestQuery {
            id: "Q5_ICOCKPIT_GOOGLE",
            theme: "Technology",
            query: "Je sors d'un essai du Renault Austral/Rafale et leur système OpenR Link avec Google intégré est ultra-fluide. Le nouveau i-Cockpit Panoramique de Peugeot est beau, mais est-il aussi réactif et connecté ?",
            mode: "hybrid",
            expected_entities: vec!["i-Cockpit", "OpenR Link", "Renault Austral"],
        },
        ApiTestQuery {
            id: "Q6_408_ITOGGLE",
            theme: "Ergonomics",
            query: "Le design de la 408 me plaît, c'est très différent de ce que fait BYD. Mais à l'usage, l'interface des i-Toggles est-elle vraiment personnalisable ou est-ce un gadget ?",
            mode: "local",
            expected_entities: vec!["Peugeot 408", "i-Toggles", "BYD"],
        },
        ApiTestQuery {
            id: "Q7_HYBRID_136",
            theme: "Hybrid",
            query: "Je ne suis pas encore sûr de passer au 100% électrique comme le veut BYD. Comment se comporte votre nouveau moteur Hybride 136 e-DCS6 ? Y a-t-il encore des à-coups ?",
            mode: "hybrid",
            expected_entities: vec!["Hybride 136", "e-DCS6", "BYD"],
        },
        ApiTestQuery {
            id: "Q8_PHEV_CONSUMPTION",
            theme: "Hybrid",
            query: "Si je prends une 308 ou 408 Hybride Rechargeable, quelle est la consommation réelle une fois la batterie vide ?",
            mode: "local",
            expected_entities: vec!["308 Hybrid", "408 Hybrid", "PHEV"],
        },
        ApiTestQuery {
            id: "Q9_BONUS_ECOLOGIQUE",
            theme: "Economy",
            query: "Les BYD sont moins chères à l'achat, mais elles ont perdu le bonus écologique en France. Si je configure un E-2008 ou un E-3008 Made in France, l'écart de prix final est-il compensé par le bonus et la valeur de revente ?",
            mode: "hybrid",
            expected_entities: vec!["E-2008", "E-3008", "BYD", "bonus écologique"],
        },
        ApiTestQuery {
            id: "Q10_E3008_SCENIC",
            theme: "Premium",
            query: "Peugeot se veut Access Premium. Par rapport à un Renault Scénic qui est Voiture de l'année, qu'est-ce qui justifie l'écart de prix sur un E-3008 en finition GT ?",
            mode: "hybrid",
            expected_entities: vec!["E-3008", "Renault Scenic", "GT"],
        },
        ApiTestQuery {
            id: "Q11_DRIVING_DYNAMICS",
            theme: "Driving",
            query: "J'ai trouvé la BYD Atto 3 un peu molle en suspension et la direction floue. Peugeot est réputé pour son châssis. Sur des véhicules aussi lourds que les électriques actuels, avez-vous gardé le toucher de route Peugeot ?",
            mode: "local",
            expected_entities: vec!["BYD Atto 3", "Peugeot", "châssis"],
        },
    ]
}

/// Check API health
async fn check_health() -> bool {
    let client = reqwest::Client::new();
    let url = format!("{}/health", get_base_url());

    match client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Send query to API
async fn query_api(query: &str, mode: &str) -> Result<ApiResponse, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/query", get_base_url());

    let body = serde_json::json!({
        "query": query,
        "mode": mode,
        "top_k": 10
    });

    match client
        .post(&url)
        .json(&body)
        .timeout(Duration::from_secs(120))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                let answer = data["answer"].as_str().unwrap_or("").to_string();
                let sources = data["sources"].as_array().map(|a| a.len()).unwrap_or(0);
                Ok(ApiResponse {
                    answer,
                    sources_count: sources,
                })
            } else {
                Err(format!("HTTP {}", resp.status()))
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Debug)]
struct ApiResponse {
    answer: String,
    sources_count: usize,
}

/// Assess quality of a response
fn assess_quality(response: &ApiResponse, expected_entities: &[&str]) -> QualityResult {
    let content = response.answer.to_lowercase();
    let length = response.answer.len();

    // Check for no-info indicators
    let no_info_phrases = [
        "ne contient pas d'information",
        "no information available",
        "pas d'information disponible",
        "cannot find any relevant",
    ];
    let has_no_info = no_info_phrases.iter().any(|p| content.contains(p)) && length < 300;

    // Count entities found
    let entities_found: Vec<&str> = expected_entities
        .iter()
        .filter(|e| content.contains(&e.to_lowercase()))
        .copied()
        .collect();

    let entity_recall = if expected_entities.is_empty() {
        1.0
    } else {
        entities_found.len() as f64 / expected_entities.len() as f64
    };

    // Determine quality level
    let (quality, base_score) = if has_no_info {
        ("NO_INFO", 0.0)
    } else if length < PARTIAL_THRESHOLD {
        ("TOO_SHORT", 20.0)
    } else if length < GOOD_THRESHOLD {
        ("PARTIAL", 50.0)
    } else if length < EXCELLENT_THRESHOLD {
        ("GOOD", 75.0)
    } else {
        ("EXCELLENT", 100.0)
    };

    // Add entity bonus
    let entity_bonus = entity_recall * 30.0;
    let score = (base_score + entity_bonus).min(100.0);

    QualityResult {
        quality: quality.to_string(),
        score,
        length,
        entities_found: entities_found.iter().map(|s| s.to_string()).collect(),
        entity_recall,
    }
}

#[derive(Debug)]
struct QualityResult {
    quality: String,
    score: f64,
    length: usize,
    entities_found: Vec<String>,
    entity_recall: f64,
}

// =============================================================================
// API Integration Tests (require running server)
// =============================================================================

/// Setup fresh workspace with test data - run this FIRST
/// This clears existing documents and ingests all test fixtures
#[tokio::test]
#[ignore = "Requires running API server"]
async fn test_00_setup_fresh_workspace() {
    println!("\n🔧 Setting up fresh workspace with test data...\n");

    let result = setup_fresh_workspace(SetupOptions::default())
        .await
        .expect("Failed to setup fresh workspace");

    println!("\n📊 Setup Results:");
    println!("   Documents cleared:  {}", result.documents_cleared);
    println!("   Documents ingested: {}", result.documents_ingested);
    println!("   Documents failed:   {}", result.documents_failed);
    println!("   Total documents:    {}", result.total_documents);

    assert!(
        result.documents_ingested > 0,
        "Should have ingested at least one document"
    );
    assert_eq!(
        result.documents_failed, 0,
        "No documents should have failed to ingest"
    );
}

/// Test API health endpoint
#[tokio::test]
#[ignore = "Requires running API server"]
async fn test_api_health() {
    let healthy = check_health().await;
    assert!(
        healthy,
        "API should be healthy. Did you start it with `make dev`?"
    );
}

/// Full test suite matching OODA 63 extended_challenge_query.py
#[tokio::test]
#[ignore = "Requires running API server with populated knowledge graph"]
async fn test_full_api_quality_suite() {
    println!("\n========================================");
    println!("  API INTEGRATION TEST SUITE");
    println!("  (Matching OODA 63 extended_challenge_query.py)");
    println!("========================================\n");

    if !check_health().await {
        panic!("API is not running. Start with `make dev` first.");
    }
    println!("✓ API is healthy\n");

    let queries = get_test_queries();
    let mut total_score = 0.0;
    let mut passed = 0;
    let mut failed = 0;

    for query in &queries {
        print!("{:20} | ", query.id);

        match query_api(query.query, query.mode).await {
            Ok(response) => {
                let quality = assess_quality(&response, &query.expected_entities);

                let status = if quality.score >= 50.0 { "✓" } else { "✗" };
                println!(
                    "{} {:10} | {:4} chars | {:5.1} score | {:.1}% recall",
                    status,
                    quality.quality,
                    quality.length,
                    quality.score,
                    quality.entity_recall * 100.0
                );

                total_score += quality.score;
                if quality.score >= 50.0 {
                    passed += 1;
                } else {
                    failed += 1;
                }
            }
            Err(e) => {
                println!("✗ ERROR: {}", e);
                failed += 1;
            }
        }
    }

    let avg_score = total_score / queries.len() as f64;
    let pass_rate = passed as f64 / queries.len() as f64 * 100.0;

    println!("\n----------------------------------------");
    println!("SUMMARY:");
    println!("  Total:     {} tests", queries.len());
    println!("  Passed:    {} ({:.1}%)", passed, pass_rate);
    println!("  Failed:    {}", failed);
    println!("  Avg Score: {:.1}", avg_score);
    println!("----------------------------------------");

    // OODA 63 target: 100% pass rate with EXCELLENT quality
    assert!(
        pass_rate >= 80.0,
        "Pass rate should be at least 80% (was {:.1}%)",
        pass_rate
    );
    assert!(
        avg_score >= 50.0,
        "Average score should be at least 50 (was {:.1})",
        avg_score
    );
}

/// Test the original French challenge query
#[tokio::test]
#[ignore = "Requires running API server"]
async fn test_french_challenge_query() {
    if !check_health().await {
        panic!("API is not running");
    }

    let query = "J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas. \
                 Concrètement, qu'est-ce que le E-3008 apporte de plus pour justifier la différence de prix ? \
                 Surtout sur l'autoroute où l'autonomie réelle chute avec la plateforme STLA Medium.";

    let response = query_api(query, "hybrid")
        .await
        .expect("Query should succeed");

    println!("\n=== French Challenge Query ===");
    println!("Query: {}...", &query[..80]);
    println!("Answer length: {} chars", response.answer.len());
    println!("Sources: {}", response.sources_count);

    // OODA 71 target: 2226+ chars for this query
    let quality = assess_quality(&response, &["BYD Seal U", "E-3008", "LFP", "STLA Medium"]);
    println!("Quality: {} (score: {:.1})", quality.quality, quality.score);

    // After OODA 62 fix, should be at least GOOD quality
    assert!(
        quality.length >= GOOD_THRESHOLD,
        "Response should be at least {} chars (was {})",
        GOOD_THRESHOLD,
        quality.length
    );
}

/// Test off-topic query handling (OODA 69)
#[tokio::test]
#[ignore = "Requires running API server"]
async fn test_offtopic_graceful_degradation() {
    if !check_health().await {
        panic!("API is not running");
    }

    let query = "Best pizza restaurants in New York";
    let response = query_api(query, "hybrid")
        .await
        .expect("Query should succeed even for off-topic");

    println!("\n=== Off-Topic Query (Pizza) ===");
    println!("Answer length: {} chars", response.answer.len());

    // Should gracefully decline rather than crash
    assert!(response.answer.len() > 0, "Should return some response");
}

/// Test adjacent domain query (OODA 69)
#[tokio::test]
#[ignore = "Requires running API server"]
async fn test_adjacent_domain_tesla() {
    if !check_health().await {
        panic!("API is not running");
    }

    let query = "Tesla Model 3 specifications and range";
    let response = query_api(query, "hybrid")
        .await
        .expect("Query should succeed");

    println!("\n=== Adjacent Domain (Tesla) ===");
    println!("Answer length: {} chars", response.answer.len());

    // Should respond gracefully, possibly suggesting alternatives
    assert!(response.answer.len() > 0, "Should return some response");
}

/// Test all query modes work
#[tokio::test]
#[ignore = "Requires running API server"]
async fn test_all_query_modes() {
    if !check_health().await {
        panic!("API is not running");
    }

    let query = "Caractéristiques du E-3008";

    for mode in ["local", "global", "hybrid"] {
        let response = query_api(query, mode).await;
        assert!(
            response.is_ok(),
            "Mode '{}' should work: {:?}",
            mode,
            response.err()
        );
        println!("Mode {}: {} chars", mode, response.unwrap().answer.len());
    }
}

// =============================================================================
// Quick Sanity Check (doesn't require server)
// =============================================================================

#[test]
fn test_quality_assessment_logic() {
    // Test EXCELLENT
    let resp = ApiResponse {
        answer: "A".repeat(1500),
        sources_count: 10,
    };
    let quality = assess_quality(&resp, &["test"]);
    assert_eq!(quality.quality, "EXCELLENT");

    // Test GOOD
    let resp = ApiResponse {
        answer: "A".repeat(800),
        sources_count: 5,
    };
    let quality = assess_quality(&resp, &[]);
    assert_eq!(quality.quality, "GOOD");

    // Test TOO_SHORT
    let resp = ApiResponse {
        answer: "Short".to_string(),
        sources_count: 0,
    };
    let quality = assess_quality(&resp, &[]);
    assert_eq!(quality.quality, "TOO_SHORT");
}

#[test]
fn test_entity_recall_calculation() {
    let resp = ApiResponse {
        answer: "The E-3008 competes with BYD Seal U".to_string(),
        sources_count: 5,
    };
    let quality = assess_quality(&resp, &["E-3008", "BYD Seal U", "Tesla"]);

    // Found 2 out of 3 entities
    assert!(quality.entity_recall > 0.6 && quality.entity_recall < 0.7);
    assert_eq!(quality.entities_found.len(), 2);
}
