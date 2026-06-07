//! SPEC-017 edgequake-pipeline contract — P0 normalizer + chunker strategy + JSON DRY.
//!
//! Proves:
//! - Single `normalize_entity_name` used by parser and merger (parse→merge key alignment)
//! - `Chunker::with_strategy` changes chunk output
//! - Shared JSON prompts and `JsonExtractionParser` path for gleaning

use std::sync::Arc;

use edgequake_pipeline::{
    chunker::{Chunker, ChunkerConfig, SentenceBoundaryChunking},
    extractor::{
        extraction_completion_options, ConfigurableEntitySchema, ExtractedEntity,
        ExtractedRelationship, LLMExtractor, SOTAExtractor,
    },
    ingestion_types::UnifiedStage,
    merger::normalize_entity_name,
    progress::{PipelineStage, StageStatus},
    prompts::{
        detect_format_markers, json_extraction_prompt, json_gleaning_prompt,
        EntityExtractionSchema, JsonExtractionParser,
    },
    stage_bridge::{
        pipeline_stage_to_unified, tasks_phase_slug_to_unified, unified_stage_slug,
        unified_to_pipeline_stage,
    },
};

#[test]
fn spec017_parse_merge_key_alignment() {
    let parser = JsonExtractionParser::new();
    let response = r#"{"entities":[{"name":"The Company","type":"ORG","description":"Acme"}],"relationships":[]}"#;
    let parsed = parser.parse(response, "chunk-0").expect("parse");
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].name, "COMPANY");

    let merge_key = normalize_entity_name(&parsed.entities[0].name);
    assert_eq!(merge_key, "COMPANY");
    assert_eq!(merge_key, normalize_entity_name("The Company"));
}

#[test]
fn spec017_special_entity_names_consistent() {
    for name in ["O'Brien", "AI/ML", "John's Lab"] {
        let parser = JsonExtractionParser::new();
        let response = format!(
            r#"{{"entities":[{{"name":"{name}","type":"PERSON","description":"x"}}],"relationships":[]}}"#
        );
        let parsed = parser.parse(&response, "c1").expect("parse");
        assert_eq!(
            normalize_entity_name(&parsed.entities[0].name),
            normalize_entity_name(name),
            "divergence for {name}"
        );
    }
}

#[test]
fn spec017_chunker_strategy_changes_output() {
    let text = "First sentence here. Second sentence follows. Third one ends.";
    let config = ChunkerConfig {
        chunk_size: 5,
        min_chunk_size: 1,
        chunk_overlap: 0,
        ..ChunkerConfig::default()
    };

    let token_chunker = Chunker::new(config.clone());
    let sentence_chunker = Chunker::with_strategy(config, Arc::new(SentenceBoundaryChunking));

    let token_chunks = token_chunker.chunk(text, "doc").expect("token chunks");
    let sentence_chunks = sentence_chunker
        .chunk(text, "doc")
        .expect("sentence chunks");

    assert_ne!(
        token_chunker.strategy_name(),
        sentence_chunker.strategy_name()
    );
    assert!(
        sentence_chunks.len() >= 2,
        "sentence strategy should split on boundaries"
    );
    assert!(!token_chunks.is_empty());
}

#[test]
fn spec017_json_prompts_single_source() {
    let schema = EntityExtractionSchema::server_default();
    let primary = json_extraction_prompt("Hello world.", &schema);
    let glean = json_gleaning_prompt("Hello world.", &["ALICE".into()]);

    assert!(primary.contains("\"entities\""));
    assert!(glean.contains("\"entities\""));
    assert!(primary.contains("Hello world."));
    assert!(glean.contains("ALICE"));
    assert!(primary.contains("JSON Response"));
    assert!(glean.contains("JSON Response"));
}

#[test]
fn spec017_json_parser_filters_self_relationships() {
    let parser = JsonExtractionParser::new();
    let response = r#"{
        "entities":[{"name":"Alice","type":"PERSON","description":"x"}],
        "relationships":[{"source":"Alice","target":"Alice","type":"SELF","description":"bad"}]
    }"#;
    let parsed = parser.parse(response, "c1").expect("parse");
    assert!(parsed.relationships.is_empty(), "BR0006 self-edge filtered");
}

#[test]
fn spec017_merge_entity_key_matches_extracted() {
    let entity = ExtractedEntity::new("The Company", "ORG", "desc");
    assert_eq!(normalize_entity_name(&entity.name), "COMPANY");

    let rel = ExtractedRelationship::new("O'Brien", "AI/ML", "WORKS_WITH");
    assert_eq!(normalize_entity_name(&rel.source), "O'BRIEN");
    assert_eq!(normalize_entity_name(&rel.target), "AI/ML");
}

#[test]
fn spec017_extraction_completion_options_reasoning_models() {
    let opts = extraction_completion_options("gpt-5-nano", 8192);
    assert_eq!(opts.max_tokens, Some(8192));
    assert!(opts.temperature.is_none());
    assert_eq!(opts.reasoning_effort.as_deref(), Some("none"));
}

#[test]
fn spec017_detect_format_markers_tuple_and_json() {
    let tuple_resp = r#"entity<|PERSON|>ALICE<|>A person"#;
    let (has_tuple, has_json) = detect_format_markers(tuple_resp);
    assert!(has_tuple);
    assert!(!has_json);

    let json_resp = r#"{"entities":[],"relationships":[]}"#;
    let (has_tuple, has_json) = detect_format_markers(json_resp);
    assert!(!has_tuple);
    assert!(has_json);
}

#[test]
fn spec017_shared_stage_status_across_modules() {
    assert_eq!(StageStatus::Pending, StageStatus::default());
}

#[test]
fn spec017_configurable_entity_schema_trait() {
    fn accepts_schema<E: ConfigurableEntitySchema>(_e: E) {}

    let provider = Arc::new(edgequake_llm::MockProvider::default());
    accepts_schema(
        LLMExtractor::new(provider.clone()).with_entity_types(vec!["ORG".into(), "PERSON".into()]),
    );
    accepts_schema(
        SOTAExtractor::new(provider).with_entity_types(vec!["ORG".into(), "PERSON".into()]),
    );
}

#[test]
fn spec017_stage_bridge_pipeline_to_unified() {
    assert_eq!(
        pipeline_stage_to_unified(PipelineStage::Extracting),
        UnifiedStage::Extracting
    );
    assert_eq!(
        unified_to_pipeline_stage(UnifiedStage::Extracting),
        Some(PipelineStage::Extracting)
    );
}

#[test]
fn spec017_stage_bridge_tasks_slug_to_unified() {
    assert_eq!(
        tasks_phase_slug_to_unified("pdf_conversion"),
        Some(UnifiedStage::Converting)
    );
    assert_eq!(unified_stage_slug(UnifiedStage::Extracting), "extracting");
}
