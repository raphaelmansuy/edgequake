//! Shared mock LLM extraction payloads for integration / contract tests (SPEC-021 DRY).

/// Deterministic entity extraction used across worker E2E, sc2 ingestion, and orchestrator tests.
pub const SPEC021_SARAH_CHEN_EXTRACTION_JSON: &str = r#"{
  "entities": [
    {"name": "Sarah Chen", "type": "PERSON", "description": "Chief architect"},
    {"name": "EdgeQuake", "type": "SYSTEM", "description": "RAG system in Rust"}
  ],
  "relationships": [
    {"source": "Sarah Chen", "target": "EdgeQuake", "type": "LEADS", "description": "Sarah leads EdgeQuake"}
  ]
}"#;
