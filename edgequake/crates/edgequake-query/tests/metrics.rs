//! Search Quality Metrics
//!
//! This module provides metrics for evaluating search and RAG system quality:
//! - Precision: fraction of retrieved items that are relevant
//! - Recall: fraction of relevant items that are retrieved
//! - F1 Score: harmonic mean of precision and recall
//! - Response quality scoring based on length and content

#![allow(dead_code)] // Many utility methods for future expansion

use std::collections::HashSet;

/// Quality level for a response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    /// Response has no useful information (< 200 chars)
    NoInfo,
    /// Response is too short (200-500 chars)
    TooShort,
    /// Response has partial information (500-1000 chars)
    Partial,
    /// Response is good (1000-1500 chars)
    Good,
    /// Response is excellent (> 1500 chars)
    Excellent,
}

impl QualityLevel {
    /// Get the base score for this quality level
    pub fn base_score(&self) -> f64 {
        match self {
            QualityLevel::NoInfo => 0.0,
            QualityLevel::TooShort => 20.0,
            QualityLevel::Partial => 50.0,
            QualityLevel::Good => 75.0,
            QualityLevel::Excellent => 100.0,
        }
    }

    /// Determine quality level from response length
    pub fn from_length(len: usize) -> Self {
        match len {
            0..=199 => QualityLevel::NoInfo,
            200..=499 => QualityLevel::TooShort,
            500..=999 => QualityLevel::Partial,
            1000..=1499 => QualityLevel::Good,
            _ => QualityLevel::Excellent,
        }
    }
}

/// Calculate precision: true positives / (true positives + false positives)
///
/// # Arguments
/// * `retrieved` - Set of retrieved items
/// * `relevant` - Set of relevant (expected) items
///
/// # Returns
/// Precision score between 0.0 and 1.0
pub fn precision<T: Eq + std::hash::Hash>(retrieved: &HashSet<T>, relevant: &HashSet<T>) -> f64 {
    if retrieved.is_empty() {
        return 0.0;
    }
    let true_positives = retrieved.intersection(relevant).count();
    true_positives as f64 / retrieved.len() as f64
}

/// Calculate recall: true positives / (true positives + false negatives)
///
/// # Arguments
/// * `retrieved` - Set of retrieved items
/// * `relevant` - Set of relevant (expected) items
///
/// # Returns
/// Recall score between 0.0 and 1.0
pub fn recall<T: Eq + std::hash::Hash>(retrieved: &HashSet<T>, relevant: &HashSet<T>) -> f64 {
    if relevant.is_empty() {
        return 1.0; // If nothing expected, we got everything
    }
    let true_positives = retrieved.intersection(relevant).count();
    true_positives as f64 / relevant.len() as f64
}

/// Calculate F1 score: harmonic mean of precision and recall
///
/// # Arguments
/// * `precision` - Precision score
/// * `recall` - Recall score
///
/// # Returns
/// F1 score between 0.0 and 1.0
pub fn f1_score(precision: f64, recall: f64) -> f64 {
    if precision + recall == 0.0 {
        return 0.0;
    }
    2.0 * precision * recall / (precision + recall)
}

/// Calculate all metrics at once
pub fn calculate_metrics<T: Eq + std::hash::Hash>(
    retrieved: &HashSet<T>,
    relevant: &HashSet<T>,
) -> MetricsResult {
    let p = precision(retrieved, relevant);
    let r = recall(retrieved, relevant);
    let f1 = f1_score(p, r);

    MetricsResult {
        precision: p,
        recall: r,
        f1_score: f1,
        true_positives: retrieved.intersection(relevant).count(),
        false_positives: retrieved.difference(relevant).count(),
        false_negatives: relevant.difference(retrieved).count(),
    }
}

/// Result of metrics calculation
#[derive(Debug, Clone)]
pub struct MetricsResult {
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
}

impl MetricsResult {
    /// Check if metrics pass a minimum threshold
    pub fn passes_threshold(&self, min_precision: f64, min_recall: f64) -> bool {
        self.precision >= min_precision && self.recall >= min_recall
    }
}

/// Response quality assessment
#[derive(Debug, Clone)]
pub struct ResponseQuality {
    pub quality_level: QualityLevel,
    pub base_score: f64,
    pub entity_bonus: f64,
    pub total_score: f64,
    pub entities_found: Vec<String>,
    pub entities_expected: Vec<String>,
    pub entity_recall: f64,
}

impl ResponseQuality {
    /// Assess response quality
    ///
    /// # Arguments
    /// * `response` - The response text
    /// * `expected_entities` - Entities expected to be mentioned in the response
    pub fn assess(response: &str, expected_entities: &[String]) -> Self {
        let response_lower = response.to_lowercase();
        let quality_level = QualityLevel::from_length(response.len());
        let base_score = quality_level.base_score();

        // Find which expected entities are mentioned
        let entities_found: Vec<String> = expected_entities
            .iter()
            .filter(|e| response_lower.contains(&e.to_lowercase()))
            .cloned()
            .collect();

        let entity_recall = if expected_entities.is_empty() {
            1.0
        } else {
            entities_found.len() as f64 / expected_entities.len() as f64
        };

        // Entity bonus: up to 30 points
        let entity_bonus = entity_recall * 30.0;
        let total_score = (base_score + entity_bonus).min(100.0);

        Self {
            quality_level,
            base_score,
            entity_bonus,
            total_score,
            entities_found,
            entities_expected: expected_entities.to_vec(),
            entity_recall,
        }
    }

    /// Check if quality passes minimum threshold
    pub fn passes_threshold(&self, min_score: f64) -> bool {
        self.total_score >= min_score
    }

    /// Check if quality is at least "Good"
    pub fn is_good(&self) -> bool {
        matches!(
            self.quality_level,
            QualityLevel::Good | QualityLevel::Excellent
        )
    }
}

/// Aggregate metrics for a test suite
#[derive(Debug, Clone, Default)]
pub struct TestSuiteMetrics {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub average_score: f64,
    pub average_precision: f64,
    pub average_recall: f64,
    pub average_f1: f64,
    pub quality_distribution: std::collections::HashMap<String, usize>,
}

impl TestSuiteMetrics {
    /// Add a test result
    pub fn add_result(&mut self, quality: &ResponseQuality, passed: bool) {
        self.total_tests += 1;
        if passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }

        // Update running average for score
        let n = self.total_tests as f64;
        self.average_score = self.average_score * (n - 1.0) / n + quality.total_score / n;
        self.average_recall = self.average_recall * (n - 1.0) / n + quality.entity_recall / n;

        // Track quality distribution
        let quality_name = format!("{:?}", quality.quality_level);
        *self.quality_distribution.entry(quality_name).or_insert(0) += 1;
    }

    /// Get pass rate
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }
        self.passed_tests as f64 / self.total_tests as f64
    }

    /// Check if suite passes minimum pass rate
    pub fn passes_min_rate(&self, min_rate: f64) -> bool {
        self.pass_rate() >= min_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_recall_basic() {
        let retrieved: HashSet<i32> = [1, 2, 3, 4].into_iter().collect();
        let relevant: HashSet<i32> = [2, 3, 5].into_iter().collect();

        // Precision: 2 true positives out of 4 retrieved = 0.5
        assert!((precision(&retrieved, &relevant) - 0.5).abs() < 0.001);

        // Recall: 2 true positives out of 3 relevant = 0.667
        assert!((recall(&retrieved, &relevant) - 0.6666).abs() < 0.01);
    }

    #[test]
    fn test_f1_score() {
        assert!((f1_score(1.0, 1.0) - 1.0).abs() < 0.001);
        assert!((f1_score(0.5, 0.5) - 0.5).abs() < 0.001);
        assert!((f1_score(0.0, 1.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_quality_level_from_length() {
        assert_eq!(QualityLevel::from_length(100), QualityLevel::NoInfo);
        assert_eq!(QualityLevel::from_length(300), QualityLevel::TooShort);
        assert_eq!(QualityLevel::from_length(700), QualityLevel::Partial);
        assert_eq!(QualityLevel::from_length(1200), QualityLevel::Good);
        assert_eq!(QualityLevel::from_length(2000), QualityLevel::Excellent);
    }

    #[test]
    fn test_response_quality_assess() {
        let response = "The Peugeot E-3008 features a 73 kWh battery with up to 513 km range. The BYD Seal U has an 85.4 kWh LFP battery.";
        let expected = vec![
            "E-3008".to_string(),
            "BYD Seal U".to_string(),
            "LFP".to_string(),
        ];

        let quality = ResponseQuality::assess(response, &expected);

        assert!(quality.entities_found.contains(&"E-3008".to_string()));
        assert!(quality.entities_found.contains(&"LFP".to_string()));
        assert_eq!(quality.entities_found.len(), 3); // All found
        assert!((quality.entity_recall - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_metrics_result() {
        let retrieved: HashSet<&str> = ["a", "b", "c"].into_iter().collect();
        let relevant: HashSet<&str> = ["b", "c", "d"].into_iter().collect();

        let result = calculate_metrics(&retrieved, &relevant);

        assert_eq!(result.true_positives, 2);
        assert_eq!(result.false_positives, 1);
        assert_eq!(result.false_negatives, 1);
    }

    #[test]
    fn test_test_suite_metrics() {
        let mut suite = TestSuiteMetrics::default();

        let q1 = ResponseQuality::assess(&"x".repeat(1500), &[]);
        let q2 = ResponseQuality::assess(&"x".repeat(500), &[]);

        suite.add_result(&q1, true);
        suite.add_result(&q2, false);

        assert_eq!(suite.total_tests, 2);
        assert_eq!(suite.passed_tests, 1);
        assert_eq!(suite.failed_tests, 1);
        assert!((suite.pass_rate() - 0.5).abs() < 0.001);
    }
}
