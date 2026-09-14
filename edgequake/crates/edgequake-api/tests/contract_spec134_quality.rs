//! SPEC-134 Slice D contract: PDF→Markdown quality levers are wired on the
//! production path (acquisition guard, empty-page escalation, language
//! fidelity, verify observability).

/// PDF conversion processor (language detect, escalate, verify persist).
fn prod_pdf_src() -> &'static str {
    include_str!("../src/processor/pdf_processing.rs")
}

fn vision_backend_src() -> &'static str {
    include_str!("../../edgequake-pdf/src/backend/vision.rs")
}

fn verify_src() -> &'static str {
    include_str!("../src/services/manuscript_verify.rs")
}

fn prompt_context_src() -> &'static str {
    include_str!("../src/services/multimodal/prompt_context.rs")
}

fn extract_src() -> &'static str {
    include_str!("../src/processor/text_insert/mod.rs")
}

#[test]
fn image_guard_wraps_pass_a_pass_b_and_judge() {
    let vision = vision_backend_src();
    assert!(
        vision.contains("ImageGuardProvider::wrap"),
        "Pass-A page OCR must go through the acquisition size guard"
    );
    let prod = prod_pdf_src();
    assert!(
        prod.contains("ImageGuardProvider::wrap"),
        "escalation / verify / figure-filter providers must be size-guarded"
    );
    let resolver = include_str!("../src/services/vlm_provider_resolver.rs");
    assert!(
        resolver.contains("ImageGuardProvider::wrap"),
        "Pass-B resolve must wrap the provider so analyze inherits the guard"
    );
}

#[test]
fn empty_page_escalation_wired_before_verify() {
    let src = prod_pdf_src();
    let escalate = src
        .find("escalate_empty_pages")
        .expect("escalation must run on the production path");
    let verify = src
        .find("verify_manuscript_markdown")
        .expect("verify must run on the production path");
    assert!(
        escalate < verify,
        "escalation must run before verify so recovered pages are judged"
    );
    assert!(
        src.contains("pages_escalated"),
        "escalation outcome must be persisted"
    );
}

#[test]
fn document_language_detected_and_propagated() {
    let src = prod_pdf_src();
    assert!(
        src.contains("detect_document_language"),
        "Pass-A markdown must be language-detected"
    );
    assert!(
        src.contains("with_optional_document_language"),
        "Pass-B analyze must run under the document-language scope"
    );
    assert!(
        src.contains("document_language"),
        "detected language must be persisted / forwarded to ingest"
    );
    let prompt = prompt_context_src();
    assert!(
        prompt.contains("document_language_override"),
        "prompt_language() must read the task-local document language"
    );
    let extract = extract_src();
    assert!(
        extract.contains("with_optional_document_language"),
        "entity extraction must run under the document-language scope"
    );
}

#[test]
fn verify_records_fail_reason_and_retries() {
    let src = verify_src();
    assert!(
        src.contains("fail_reason"),
        "VerifyOutcome must carry the fail-open reason"
    );
    assert!(
        src.contains("judge_page_with_retry"),
        "judge must retry once before fail-open"
    );
    let prod = prod_pdf_src();
    assert!(
        prod.contains("grounding_fail_reason"),
        "fail-open reason must be persisted on the document"
    );
}

#[test]
fn tiling_and_quarantine_still_wired() {
    // Slice D must not regress P0 (belief gate).
    let vision = vision_backend_src();
    assert!(vision.contains("extract_embedded_figures"));
    assert!(vision.contains("skip_all_region_crops") || vision.contains("omit_page"));
    assert!(vision.contains("prune_artifact_figures"));
    let prepare = include_str!("../src/processor/text_insert/prepare.rs");
    assert!(prepare.contains("strip_low_grounding_sections(&text_content)"));
}

#[test]
fn detect_french_fixture_reaches_prompt_language() {
    let md = "<!-- edgequake-page:1 -->\n\nRésultat essais mécanique.\nPas d'essai en réception.\n21 essais pour l'homologation.\nLes valeurs sont dans le tableau.\n";
    let lang = edgequake_pdf::detect_document_language(md).expect("French sample");
    assert_eq!(lang, "French");
}
