use std::fs;
use std::path::PathBuf;

#[test]
fn test_gold_dataset_has_at_least_100_docs() {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/gold");

    let mut count = 0usize;

    if let Ok(entries) = fs::read_dir(&test_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(files) = fs::read_dir(&path) {
                    for f in files.flatten() {
                        if let Some(ext) = f.path().extension() {
                            if ext == "md" {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    println!("Found {} gold markdown documents", count);
    assert!(count >= 100, "Need at least 100 gold markdown documents")
}
