//! SPEC-150: fossils in manifest.toml must not equal the current checksums.lock hash.

use sha2::{Digest, Sha384};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

fn load_lock() -> HashMap<i64, String> {
    let lock = fs::read_to_string(migrations_dir().join("checksums.lock")).expect("checksums.lock");
    let mut map = HashMap::new();
    for line in lock.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else { continue };
        let Some(file) = parts.next() else { continue };
        let digits: String = file.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<i64>() {
            map.insert(v, hash.to_ascii_lowercase());
        }
    }
    map
}

#[test]
fn every_migration_file_has_manifest_entry() {
    let dir = migrations_dir();
    let manifest = edgequake_migrate_manifest::load();
    let mut files = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if name.ends_with(".sql")
            && name.chars().next().is_some_and(|c| c.is_ascii_digit())
            && !name.starts_with("000_")
        {
            files.push(name);
        }
    }
    files.sort();
    assert_eq!(
        files.len(),
        manifest.migration.len(),
        "manifest entry count must equal numbered *.sql count"
    );
    for (file, entry) in files.iter().zip(manifest.migration.iter()) {
        assert_eq!(file, &entry.file, "manifest file order mismatch");
    }
}

#[test]
fn fossils_differ_from_lock_and_from_file_bytes() {
    let lock = load_lock();
    let dir = migrations_dir();
    let manifest = edgequake_migrate_manifest::load();
    for entry in &manifest.migration {
        let path = dir.join(&entry.file);
        let bytes = fs::read(&path).unwrap_or_else(|_| panic!("read {}", entry.file));
        let file_hash = hex_sha384(&bytes);
        let Some(lock_hash) = lock.get(&entry.version) else {
            panic!("checksums.lock missing version {}", entry.version);
        };
        assert_eq!(
            lock_hash, &file_hash,
            "checksums.lock must match file bytes for {}",
            entry.file
        );
        for fossil in &entry.fossils {
            assert_ne!(
                fossil.sha384.to_ascii_lowercase(),
                file_hash,
                "fossil equals current file hash for version {}",
                entry.version
            );
            assert_eq!(fossil.sha384.len(), 96);
        }
    }
}

fn hex_sha384(bytes: &[u8]) -> String {
    let digest = Sha384::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
