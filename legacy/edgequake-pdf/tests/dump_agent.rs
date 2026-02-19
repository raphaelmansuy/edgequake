use std::fs;
use std::path::PathBuf;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

#[test]
fn dump_agent_paper() {
    let pdf_name = "agent_2510.09244v1";
    let real_dir = test_data_dir().join("real_dataset");
    let pdf_path = real_dir.join(format!("{}.pdf", pdf_name));
    let gold_path = real_dir.join(format!("{}.pymupdf.gold.md", pdf_name));

    if !pdf_path.exists() {
        eprintln!("SKIP: PDF not found");
        return;
    }

    let gold = fs::read_to_string(&gold_path).expect("read gold");
    let pipeline = edgequake_pdf::pipeline::PymupdfPipeline::new().expect("pipeline");
    let extracted = pipeline.convert_file(&pdf_path).expect("convert");

    fs::write("/tmp/edgequake_extracted_agent.md", &extracted).expect("write extracted");
    fs::write("/tmp/edgequake_gold_agent.md", &gold).expect("write gold");

    eprintln!(
        "Extracted: {} chars -> /tmp/edgequake_extracted_agent.md",
        extracted.len()
    );
    eprintln!(
        "Gold:      {} chars -> /tmp/edgequake_gold_agent.md",
        gold.len()
    );

    // Also dump SPS computation details
    let sps =
        edgequake_pdf::layout::quality_metrics::structure_preservation_score(&extracted, &gold);
    eprintln!("SPS = {:.3}", sps);
}
