//! Build script for EdgeQuake - generates build metadata at compile time.
//!
//! WHY: Embeds git hash, build timestamp, and build number into the binary
//! so that the health API and frontend can display exact version information.
//! This ensures every build is traceable to a specific commit and time.

use std::process::Command;

fn main() {
    // Build timestamp (ISO 8601 UTC)
    let build_timestamp = chrono_lite_utc_now();
    println!("cargo:rustc-env=EDGEQUAKE_BUILD_TIMESTAMP={build_timestamp}");

    // Git short hash
    let git_hash = git_short_hash().unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=EDGEQUAKE_GIT_HASH={git_hash}");

    // Git branch
    let git_branch = git_branch().unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=EDGEQUAKE_GIT_BRANCH={git_branch}");

    // Build number: YYYYMMDD.HHMMSS format for monotonic ordering
    let build_number = build_timestamp
        .replace(['-', ':'], "")
        .replace('T', ".")
        .replace('Z', "");
    println!("cargo:rustc-env=EDGEQUAKE_BUILD_NUMBER={build_number}");

    // Rebuild when git HEAD changes (new commit)
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");
    // Rebuild when build.rs itself changes
    println!("cargo:rerun-if-changed=build.rs");
}

/// Get current UTC timestamp without external dependencies.
fn chrono_lite_utc_now() -> String {
    let output = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    }
}

/// Get short git commit hash.
fn git_short_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get current git branch name.
fn git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
