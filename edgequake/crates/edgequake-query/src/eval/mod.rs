//! RAG evaluation harness skeleton (SPEC-025 8.1 / N-10).

pub mod golden_set;
pub mod metrics;

pub use golden_set::{load_spec025_golden_set, GoldenQaCase, GoldenSetStats};
pub use metrics::{context_entity_recall, keyword_recall_in_text};
