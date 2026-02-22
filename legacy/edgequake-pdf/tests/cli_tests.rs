//! CLI integration tests for edgequake-pdf.
//!
//! These tests verify that the CLI commands work correctly with various options.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the compiled CLI binary.
fn get_binary_path() -> PathBuf {
    // Build path relative to cargo target directory
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go up from edgequake-pdf
    path.pop(); // Go up from crates
    path.push("target");
    path.push("debug");
    path.push("edgequake-pdf");
    path
}

/// Get path to a test PDF file.
fn get_test_pdf(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("test-data");
    path.push(name);
    path
}

/// Helper to create a temporary output path.
fn temp_output_path(suffix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "edgequake-pdf-test-{}-{}.md",
        std::process::id(),
        suffix
    ));
    path
}

#[test]
fn test_cli_help() {
    let binary = get_binary_path();

    let output = Command::new(&binary)
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Help command should succeed");
    assert!(stdout.contains("edgequake-pdf"), "Should show program name");
    assert!(stdout.contains("convert"), "Should list convert command");
    assert!(stdout.contains("info"), "Should list info command");
    assert!(stdout.contains("--vision"), "Should mention vision option");
}

#[test]
fn test_cli_version() {
    let binary = get_binary_path();

    let output = Command::new(&binary)
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Version command should succeed");
    assert!(
        stdout.contains("edgequake-pdf"),
        "Should show program name in version"
    );
}

#[test]
fn test_cli_convert_help() {
    let binary = get_binary_path();

    let output = Command::new(&binary)
        .args(["convert", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Convert help should succeed");
    assert!(stdout.contains("--vision"), "Should show vision option");
    assert!(
        stdout.contains("--vision-model"),
        "Should show vision-model option"
    );
    assert!(stdout.contains("--format"), "Should show format option");
    assert!(stdout.contains("--stdout"), "Should show stdout option");
    assert!(
        stdout.contains("--max-pages"),
        "Should show max-pages option"
    );
}

#[test]
fn test_cli_info_command() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found at {:?}", test_pdf);
        return;
    }

    let output = Command::new(&binary)
        .args(["info", "-i", test_pdf.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Info command should succeed");
    assert!(
        stdout.contains("Pages:"),
        "Should show page count: {}",
        stdout
    );
    assert!(stdout.contains("Version:"), "Should show PDF version");
    assert!(stdout.contains("Size:"), "Should show file size");
}

#[test]
fn test_cli_info_json_format() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    let output = Command::new(&binary)
        .args([
            "info",
            "-i",
            test_pdf.to_str().unwrap(),
            "--format",
            "json",
            "-q",
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Info JSON command should succeed");

    // Find JSON in output (may have log lines before)
    let json_start = stdout.find('{').expect("Should contain JSON object");
    let json_str = &stdout[json_start..];

    // Should be valid JSON
    let json: serde_json::Value =
        serde_json::from_str(json_str).expect("Output should be valid JSON");
    assert!(json.get("pages").is_some(), "JSON should have pages field");
    assert!(
        json.get("version").is_some(),
        "JSON should have version field"
    );
}

#[test]
fn test_cli_convert_shorthand() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");
    let output_path = temp_output_path("shorthand");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    // Clean up any existing output
    let _ = fs::remove_file(&output_path);

    let output = Command::new(&binary)
        .args([
            test_pdf.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
            "-q",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Shorthand convert should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_path.exists(), "Output file should be created");

    let content = fs::read_to_string(&output_path).expect("Should read output");
    assert!(!content.is_empty(), "Output should not be empty");

    // Clean up
    let _ = fs::remove_file(&output_path);
}

#[test]
fn test_cli_convert_explicit() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");
    let output_path = temp_output_path("explicit");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    let _ = fs::remove_file(&output_path);

    let output = Command::new(&binary)
        .args([
            "convert",
            "-i",
            test_pdf.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
            "-q",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Explicit convert should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_path.exists(), "Output file should be created");

    let _ = fs::remove_file(&output_path);
}

#[test]
fn test_cli_convert_to_stdout() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    let output = Command::new(&binary)
        .args([
            "convert",
            "-i",
            test_pdf.to_str().unwrap(),
            "--stdout",
            "-q",
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Stdout convert should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!stdout.is_empty(), "Should output markdown to stdout");
}

#[test]
fn test_cli_convert_json_format() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    let output = Command::new(&binary)
        .args([
            "convert",
            "-i",
            test_pdf.to_str().unwrap(),
            "--format",
            "json",
            "--stdout",
            "-q",
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "JSON convert should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should be valid JSON
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    assert!(json.get("pages").is_some(), "JSON should have pages field");
}

#[test]
fn test_cli_convert_with_max_pages() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");
    let output_path = temp_output_path("maxpages");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    let _ = fs::remove_file(&output_path);

    let output = Command::new(&binary)
        .args([
            "convert",
            "-i",
            test_pdf.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
            "--max-pages",
            "1",
            "-q",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Max pages convert should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_path.exists(), "Output file should be created");

    let _ = fs::remove_file(&output_path);
}

#[test]
fn test_cli_convert_with_page_numbers() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");
    let output_path = temp_output_path("pagenumbers");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    let _ = fs::remove_file(&output_path);

    let output = Command::new(&binary)
        .args([
            "convert",
            "-i",
            test_pdf.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
            "--page-numbers",
            "-q",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Page numbers convert should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_path.exists(), "Output file should be created");

    let _ = fs::remove_file(&output_path);
}

#[test]
fn test_cli_vision_without_api_key() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");
    let output_path = temp_output_path("vision-nokey");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    let _ = fs::remove_file(&output_path);

    // Ensure OPENAI_API_KEY is not set for this test
    let output = Command::new(&binary)
        .args([
            "convert",
            "-i",
            test_pdf.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
            "--vision",
            "-q",
        ])
        .env_remove("OPENAI_API_KEY")
        .output()
        .expect("Failed to execute command");

    // Should still succeed (falls back to non-vision mode)
    assert!(
        output.status.success(),
        "Vision without API key should fall back gracefully: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should show warning about missing API key
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("OPENAI_API_KEY") || output_path.exists(),
        "Should warn about missing API key or succeed with fallback"
    );

    let _ = fs::remove_file(&output_path);
}

#[test]
fn test_cli_file_not_found() {
    let binary = get_binary_path();

    let output = Command::new(&binary)
        .args(["convert", "-i", "/nonexistent/file.pdf", "-q"])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail for non-existent file"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("No such file"),
        "Should show file not found error"
    );
}

#[test]
fn test_cli_default_output_path() {
    let binary = get_binary_path();
    let test_pdf = get_test_pdf("001_simple_text.pdf");

    if !test_pdf.exists() {
        eprintln!("Skipping test: test PDF not found");
        return;
    }

    // Expected output path (same as input but with .md extension)
    let mut expected_output = test_pdf.clone();
    expected_output.set_extension("md");

    // Clean up before test
    let _ = fs::remove_file(&expected_output);

    let output = Command::new(&binary)
        .args(["convert", "-i", test_pdf.to_str().unwrap(), "-q"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Convert should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        expected_output.exists(),
        "Output file should be created at default path: {:?}",
        expected_output
    );

    let content = fs::read_to_string(&expected_output).expect("Should read output");
    assert!(!content.is_empty(), "Output should not be empty");

    // Clean up
    let _ = fs::remove_file(&expected_output);
}
