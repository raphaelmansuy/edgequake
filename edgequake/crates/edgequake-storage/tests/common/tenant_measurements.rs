//! Repeatable local provider timings; assertions stay in the calling integration test.
pub fn record(provider: &str, fixture_rows: usize, mut samples_ms: Vec<f64>) {
    assert_eq!(samples_ms.len(), 21);
    samples_ms.sort_by(f64::total_cmp);
    let report = serde_json::json!({"provider":provider,"fixture_rows":fixture_rows,"samples":21,"warmup":5,"p50_ms":samples_ms[10],"p95_ms":samples_ms[19],"samples_ms":samples_ms,"limits":"local sequential adapter read envelope; not a production load test"});
    if let Ok(directory) = std::env::var("EQ_TENANT_PROVIDER_REPORT_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join(format!("{provider}.json")),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    eprintln!("TENANT_PROVIDER_REPORT={report}");
}
