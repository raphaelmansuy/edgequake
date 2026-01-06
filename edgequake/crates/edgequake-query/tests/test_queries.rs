//! Test Queries Module
//!
//! Contains the 11 French automotive queries from the search quality test suite.
//! These queries test various aspects of the RAG system with French language content.
//!
//! WHY: These queries were designed to challenge the search system with:
//! - French language queries (accent handling, French terms)
//! - Technical automotive domain knowledge
//! - Entity matching (car models, features, brands)
//! - Different query modes (global, local, hybrid)

#![allow(dead_code)] // Test utility functions available for external use

use std::collections::HashSet;

/// A test query with expected results
#[derive(Debug, Clone)]
pub struct TestQuery {
    /// Query identifier
    pub id: &'static str,
    /// Theme or category of the query
    pub theme: &'static str,
    /// The actual query text (French)
    pub query: &'static str,
    /// Expected entities that should be mentioned in the response
    pub expected_entities: &'static [&'static str],
    /// Recommended query mode
    pub mode: &'static str,
}

impl TestQuery {
    /// Get expected entities as a HashSet for metrics calculation
    pub fn expected_entities_set(&self) -> HashSet<String> {
        self.expected_entities
            .iter()
            .map(|s| s.to_lowercase())
            .collect()
    }

    /// Get expected entities as a Vec<String>
    pub fn expected_entities_vec(&self) -> Vec<String> {
        self.expected_entities
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

/// All test queries - 11 French automotive queries
pub static TEST_QUERIES: &[TestQuery] = &[
    // Q1: STLA Large vs BYD - Challenge about product comparison
    TestQuery {
        id: "Q1_STLA_BYD",
        theme: "competitive_analysis",
        query: "Quels sont les avantages du E-3008 par rapport au BYD Seal U en termes de batteries et autonomie ?",
        expected_entities: &[
            "E-3008",
            "BYD Seal U",
            "batterie",
            "autonomie",
            "73 kWh",
            "98 kWh",
            "NMC",
            "LFP",
        ],
        mode: "global",
    },
    // Q2: E-208 vs Renault 5 vs BYD Dolphin
    TestQuery {
        id: "Q2_E208_R5",
        theme: "competitive_analysis",
        query: "Comment la E-208 se compare-t-elle à la Renault 5 et à la BYD Dolphin pour une utilisation quotidienne ?",
        expected_entities: &[
            "E-208",
            "Renault 5",
            "BYD Dolphin",
            "batterie",
            "autonomie",
            "quotidienne",
        ],
        mode: "global",
    },
    // Q3: Allure Care warranty
    TestQuery {
        id: "Q3_ALLURE_CARE",
        theme: "warranty_services",
        query: "Qu'est-ce que le programme Allure Care et quels avantages offre-t-il en termes de garantie et entretien ?",
        expected_entities: &[
            "Allure Care",
            "garantie",
            "entretien",
            "8 ans",
            "160 000 km",
        ],
        mode: "local",
    },
    // Q4: Peugeot 2008 features
    TestQuery {
        id: "Q4_PEUGEOT_2008",
        theme: "product_features",
        query: "Quelles sont les caractéristiques principales du Peugeot 2008 en version GT ?",
        expected_entities: &[
            "2008",
            "GT",
            "i-Cockpit",
            "moteur",
            "PureTech",
        ],
        mode: "local",
    },
    // Q5: i-Cockpit vs Google
    TestQuery {
        id: "Q5_ICOCKPIT_GOOGLE",
        theme: "technology",
        query: "Quels sont les avantages du i-Cockpit par rapport à l'OpenR Link de Renault avec Google ?",
        expected_entities: &[
            "i-Cockpit",
            "OpenR Link",
            "Google",
            "Renault",
            "écran",
            "tableau de bord",
        ],
        mode: "global",
    },
    // Q6: 408 design and i-Toggles
    TestQuery {
        id: "Q6_408_ITOGGLE",
        theme: "design_features",
        query: "Comment les i-Toggles de la 408 améliorent-ils l'expérience de conduite ?",
        expected_entities: &[
            "408",
            "i-Toggle",
            "raccourcis",
            "personnalisation",
        ],
        mode: "local",
    },
    // Q7: Hybrid 136 e-DCS6 motor
    TestQuery {
        id: "Q7_HYBRID_136",
        theme: "powertrain",
        query: "Quelles sont les spécificités techniques du moteur Hybrid 136 e-DCS6 ?",
        expected_entities: &[
            "Hybrid 136",
            "e-DCS6",
            "électrique",
            "boîte",
            "48V",
        ],
        mode: "local",
    },
    // Q8: PHEV consumption
    TestQuery {
        id: "Q8_PHEV_CONSUMPTION",
        theme: "efficiency",
        query: "Quelle est la consommation réelle d'un véhicule PHEV Peugeot en mode électrique et hybride ?",
        expected_entities: &[
            "PHEV",
            "consommation",
            "électrique",
            "hybride",
            "kWh",
        ],
        mode: "global",
    },
    // Q9: Bonus écologique comparison
    TestQuery {
        id: "Q9_BONUS_ECOLOGIQUE",
        theme: "pricing",
        query: "Le E-3008 est-il éligible au bonus écologique et comment son prix se compare-t-il au Scenic électrique ?",
        expected_entities: &[
            "E-3008",
            "bonus écologique",
            "prix",
            "Scenic",
            "électrique",
        ],
        mode: "global",
    },
    // Q10: E-3008 vs Scenic
    TestQuery {
        id: "Q10_E3008_SCENIC",
        theme: "competitive_analysis",
        query: "Quelles sont les différences principales entre le E-3008 et le Renault Scenic électrique ?",
        expected_entities: &[
            "E-3008",
            "Scenic",
            "Renault",
            "batterie",
            "autonomie",
        ],
        mode: "global",
    },
    // Q11: Driving dynamics
    TestQuery {
        id: "Q11_DRIVING_DYNAMICS",
        theme: "driving_experience",
        query: "Comment le châssis de la 308 contribue-t-il à une expérience de conduite dynamique ?",
        expected_entities: &[
            "308",
            "châssis",
            "suspension",
            "conduite",
            "dynamique",
        ],
        mode: "local",
    },
];

/// Get a query by ID
pub fn get_query_by_id(id: &str) -> Option<&'static TestQuery> {
    TEST_QUERIES.iter().find(|q| q.id == id)
}

/// Get all queries for a specific theme
pub fn get_queries_by_theme(theme: &str) -> Vec<&'static TestQuery> {
    TEST_QUERIES.iter().filter(|q| q.theme == theme).collect()
}

/// Get all queries for a specific mode
pub fn get_queries_by_mode(mode: &str) -> Vec<&'static TestQuery> {
    TEST_QUERIES.iter().filter(|q| q.mode == mode).collect()
}

/// Get all unique themes
pub fn get_all_themes() -> Vec<&'static str> {
    let mut themes: Vec<&str> = TEST_QUERIES.iter().map(|q| q.theme).collect();
    themes.sort();
    themes.dedup();
    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_count() {
        assert_eq!(TEST_QUERIES.len(), 11);
    }

    #[test]
    fn test_get_query_by_id() {
        let query = get_query_by_id("Q1_STLA_BYD");
        assert!(query.is_some());
        let q = query.unwrap();
        assert_eq!(q.theme, "competitive_analysis");
        assert!(q.query.contains("E-3008"));
    }

    #[test]
    fn test_get_queries_by_theme() {
        let competitive = get_queries_by_theme("competitive_analysis");
        assert_eq!(competitive.len(), 3); // Q1, Q2, Q10
    }

    #[test]
    fn test_get_queries_by_mode() {
        let global = get_queries_by_mode("global");
        let local = get_queries_by_mode("local");
        assert_eq!(global.len(), 6);
        assert_eq!(local.len(), 5);
    }

    #[test]
    fn test_expected_entities_set() {
        let query = get_query_by_id("Q1_STLA_BYD").unwrap();
        let entities = query.expected_entities_set();
        assert!(entities.contains("e-3008")); // lowercase
        assert!(entities.contains("byd seal u"));
    }

    #[test]
    fn test_all_queries_have_entities() {
        for query in TEST_QUERIES {
            assert!(
                !query.expected_entities.is_empty(),
                "Query {} should have expected entities",
                query.id
            );
        }
    }

    #[test]
    fn test_all_queries_are_french() {
        for query in TEST_QUERIES {
            // French queries typically contain accented characters or French words
            let is_french = query.query.contains("Quels")
                || query.query.contains("Qu'est")
                || query.query.contains("Comment")
                || query.query.contains("Quelle")
                || query.query.contains("Le ")
                || query.query.contains("La ");
            assert!(
                is_french,
                "Query {} should be in French: {}",
                query.id, query.query
            );
        }
    }
}
