//! Read/write support for run-owned provider-access evidence.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestCounts {
    pub selected: u64,
    pub passed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub certification_successes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunManifest {
    pub schema_version: u32,
    pub run_id: String,
    pub profile: String,
    pub suite: String,
    pub status: String,
    pub feature_flags: Vec<String>,
    pub tests: TestCounts,
}

pub fn write(path: &Path, manifest: &RunManifest) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "provider_access_manifest_missing_parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let json = serde_json::to_string_pretty(manifest).map_err(|error| error.to_string())?;
    fs::write(path, format!("{json}\n")).map_err(|error| error.to_string())
}

pub fn read(path: &Path) -> Result<RunManifest, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}
