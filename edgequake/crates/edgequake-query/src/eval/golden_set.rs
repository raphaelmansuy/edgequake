//! Golden Q&A dataset loader for regression eval (SPEC-025 8.1).

use serde::Deserialize;

/// One golden question with lightweight quality gates (RAGAS-style skeleton).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GoldenQaCase {
    pub id: String,
    pub query: String,
    pub expected_answer_keywords: Vec<String>,
    pub expected_context_entities: Vec<String>,
    #[serde(default)]
    pub mode: Option<String>,
}

/// Load the SPEC-025 golden set embedded at compile time.
pub fn load_spec025_golden_set() -> Vec<GoldenQaCase> {
    serde_json::from_str(include_str!("../../tests/fixtures/spec025_golden_qa.json"))
        .expect("spec025 golden Q&A fixture must parse")
}

/// Summary stats for CI gates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoldenSetStats {
    pub case_count: usize,
    pub with_mode_hint: usize,
    pub with_context_entities: usize,
}

impl GoldenSetStats {
    pub fn from_cases(cases: &[GoldenQaCase]) -> Self {
        Self {
            case_count: cases.len(),
            with_mode_hint: cases.iter().filter(|c| c.mode.is_some()).count(),
            with_context_entities: cases
                .iter()
                .filter(|c| !c.expected_context_entities.is_empty())
                .count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_set_loads_at_least_fifty_cases() {
        let cases = load_spec025_golden_set();
        assert!(
            cases.len() >= 50,
            "SPEC-025 requires ≥50 golden Q&A cases, got {}",
            cases.len()
        );
    }
}
