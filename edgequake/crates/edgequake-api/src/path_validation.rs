//! Path validation module for secure file system access.
//!
//! WHY: Path traversal is a critical security vulnerability that allows attackers
//! to access files outside intended directories (e.g., `../../../etc/passwd`).
//! This module provides canonicalization, allowed-path validation, and traversal detection.
//!
//! SECURITY: All filesystem paths from user input MUST go through `validate_path()`.
//!
//! # Example
//!
//! ```rust
//! use edgequake_api::path_validation::{validate_path, PathValidationConfig};
//!
//! let config = PathValidationConfig::default();
//! let result = validate_path("/data/uploads/doc.pdf", &config);
//! ```

use std::path::{Path, PathBuf};

use crate::error::ApiError;

/// Configuration for path validation.
#[derive(Debug, Clone)]
pub struct PathValidationConfig {
    /// Allowed base directories for file access.
    /// If empty, no paths are allowed (secure default).
    pub allowed_paths: Vec<PathBuf>,

    /// Whether to allow paths outside allowed_paths list.
    /// SECURITY: Should be false in production.
    pub allow_any_path: bool,

    /// Whether to follow symlinks.
    /// SECURITY: false prevents symlink-based escapes.
    pub follow_symlinks: bool,

    /// Maximum path depth allowed (prevents deeply nested traversal attempts).
    pub max_depth: usize,
}

impl Default for PathValidationConfig {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            allow_any_path: false, // Secure default: no paths allowed
            follow_symlinks: false,
            max_depth: 50,
        }
    }
}

impl PathValidationConfig {
    /// Create a permissive config for development/testing only.
    ///
    /// WARNING: Do not use in production!
    #[cfg(test)]
    pub fn permissive() -> Self {
        Self {
            allowed_paths: Vec::new(),
            allow_any_path: true,
            follow_symlinks: true,
            max_depth: 100,
        }
    }

    /// Create a config with specific allowed paths.
    pub fn with_allowed_paths<P: Into<PathBuf>>(paths: impl IntoIterator<Item = P>) -> Self {
        Self {
            allowed_paths: paths.into_iter().map(Into::into).collect(),
            allow_any_path: false,
            follow_symlinks: false,
            max_depth: 50,
        }
    }
}

/// Result of successful path validation.
#[derive(Debug)]
pub struct ValidatedPath {
    /// The canonicalized absolute path.
    pub canonical: PathBuf,

    /// The original path that was validated.
    pub original: PathBuf,

    /// Whether the path is within an allowed directory.
    pub is_allowed: bool,
}

/// Validate a path for safe filesystem access.
///
/// # Security Checks
///
/// 1. **Canonicalization**: Resolves `.`, `..`, and symlinks to absolute path
/// 2. **Traversal Detection**: Rejects paths with `..` components
/// 3. **Allowed Path Check**: Verifies path is within allowed directories
/// 4. **Depth Limit**: Prevents excessive nesting
///
/// # Arguments
///
/// * `path` - The user-provided path to validate
/// * `config` - Validation configuration
///
/// # Returns
///
/// * `Ok(ValidatedPath)` - Path is safe to use
/// * `Err(ApiError)` - Path failed validation
///
/// # Example
///
/// ```rust,no_run
/// use edgequake_api::path_validation::{validate_path, PathValidationConfig};
///
/// let config = PathValidationConfig::with_allowed_paths(["/data/uploads"]);
/// let validated = validate_path("/data/uploads/doc.pdf", &config)?;
/// println!("Safe to access: {}", validated.canonical.display());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn validate_path<P: AsRef<Path>>(
    path: P,
    config: &PathValidationConfig,
) -> Result<ValidatedPath, ApiError> {
    let path = path.as_ref();
    let original = path.to_path_buf();

    // Check for obvious traversal patterns in the raw path
    let path_str = path.to_string_lossy();
    if contains_traversal_pattern(&path_str) {
        return Err(ApiError::BadRequest(format!(
            "Path contains forbidden traversal pattern: {}",
            path_str
        )));
    }

    // Check path depth
    let depth = path.components().count();
    if depth > config.max_depth {
        return Err(ApiError::BadRequest(format!(
            "Path depth {} exceeds maximum allowed depth {}",
            depth, config.max_depth
        )));
    }

    // Canonicalize the path (resolves symlinks and ..)
    let canonical = if config.follow_symlinks {
        std::fs::canonicalize(path).map_err(|e| {
            ApiError::BadRequest(format!("Invalid path '{}': {}", path.display(), e))
        })?
    } else {
        // Use a safer canonicalization that doesn't follow symlinks
        safe_canonicalize(path)?
    };

    // Check if the canonicalized path is within allowed directories
    let is_allowed = if config.allow_any_path {
        true
    } else {
        config
            .allowed_paths
            .iter()
            .any(|allowed| is_path_within(allowed, &canonical))
    };

    if !is_allowed {
        return Err(ApiError::Forbidden);
    }

    Ok(ValidatedPath {
        canonical,
        original,
        is_allowed,
    })
}

/// Check if a path string contains traversal patterns.
///
/// WHY: Raw string check catches encoded/obfuscated traversal attempts
/// before canonicalization.
fn contains_traversal_pattern(path: &str) -> bool {
    // Check for common traversal patterns
    let patterns = [
        "..",         // Direct parent traversal
        "%2e%2e",     // URL encoded ..
        "%252e%252e", // Double URL encoded ..
        "..%2f",      // Mixed encoding
        "%2f..",      // Mixed encoding
        "..\\",       // Windows traversal
        "\\..\\",     // Windows traversal
        "....//",     // Obfuscated traversal
        "..;/",       // Tomcat-style traversal
        "..\\/",      // Mixed separator
    ];

    let lower = path.to_lowercase();
    patterns.iter().any(|p| lower.contains(p))
}

/// Canonicalize a path without following symlinks.
///
/// WHY: Following symlinks can allow escape from allowed directories.
fn safe_canonicalize(path: &Path) -> Result<PathBuf, ApiError> {
    // First, check if the path exists
    if !path.exists() {
        return Err(ApiError::NotFound(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }

    // Check if it's a symlink
    if path.is_symlink() {
        return Err(ApiError::BadRequest(format!(
            "Symlinks are not allowed: {}",
            path.display()
        )));
    }

    // Get the canonical path
    std::fs::canonicalize(path).map_err(|e| {
        ApiError::BadRequest(format!("Cannot canonicalize '{}': {}", path.display(), e))
    })
}

/// Check if a path is within (or equal to) a base directory.
///
/// WHY: Path prefix checking must use canonicalized paths to prevent
/// traversal via symbolic links or `..` components.
fn is_path_within(base: &Path, path: &Path) -> bool {
    // Both paths should be canonical at this point
    match (base.canonicalize(), path.canonicalize()) {
        (Ok(base_canon), Ok(path_canon)) => path_canon.starts_with(&base_canon),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_traversal_pattern_detection() {
        assert!(contains_traversal_pattern(".."));
        assert!(contains_traversal_pattern("../etc"));
        assert!(contains_traversal_pattern("/foo/../bar"));
        assert!(contains_traversal_pattern("foo%2e%2ebar"));
        assert!(contains_traversal_pattern("foo\\..\\bar"));

        assert!(!contains_traversal_pattern("/foo/bar"));
        assert!(!contains_traversal_pattern("./foo"));
        assert!(!contains_traversal_pattern("/data/uploads"));
    }

    #[test]
    fn test_validate_path_rejects_traversal() {
        let config = PathValidationConfig::default();

        let result = validate_path("../etc/passwd", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }

    #[test]
    fn test_validate_path_rejects_depth() {
        let config = PathValidationConfig {
            max_depth: 3,
            allow_any_path: true,
            ..Default::default()
        };

        // This has 5+ components
        let result = validate_path("/a/b/c/d/e/f/g", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("depth"));
    }

    #[test]
    fn test_validate_path_within_allowed() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_path_buf();

        // Create a test file
        let test_file = allowed_path.join("test.txt");
        fs::write(&test_file, "test").unwrap();

        let config = PathValidationConfig::with_allowed_paths([&allowed_path]);

        let result = validate_path(&test_file, &config);
        assert!(result.is_ok());
        assert!(result.unwrap().is_allowed);
    }

    #[test]
    fn test_validate_path_outside_allowed_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let config = PathValidationConfig::with_allowed_paths([temp_dir.path()]);

        // /tmp exists but is outside allowed path
        let result = validate_path("/tmp", &config);
        assert!(
            result.is_err(),
            "Path outside allowed directories should be rejected"
        );
    }

    #[test]
    fn test_default_config_rejects_all() {
        let config = PathValidationConfig::default();

        // Even existing paths are rejected with default config
        let result = validate_path("/tmp", &config);
        assert!(result.is_err());
    }
}
