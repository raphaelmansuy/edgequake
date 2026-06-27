//! SPEC-026 Phase 4e — JSON recovery contract tests.

use edgequake_api::services::vision_content::ImageAnalysisResult;
use edgequake_api::services::{extract_json_object, parse_json_object};

#[test]
fn extracts_fenced_json_object() {
    let raw = "```json\n{\"name\":\"x\",\"type\":\"Chart\",\"description\":\"y\"}\n```";
    let json = extract_json_object(raw).expect("json");
    let parsed: ImageAnalysisResult = parse_json_object(json).unwrap();
    assert_eq!(parsed.name, "x");
    assert_eq!(parsed.image_type, "Chart");
}

#[test]
fn parse_rejects_non_json() {
    assert!(parse_json_object::<ImageAnalysisResult>("not json").is_err());
}
