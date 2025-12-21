//! Hashing utilities.

/// Generate an MD5 hash of the input string.
///
/// # Example
///
/// ```rust
/// use edgequake_core::utils::md5_hash;
///
/// let hash = md5_hash("Hello, World!");
/// assert_eq!(hash.len(), 32);
/// ```
pub fn md5_hash(input: &str) -> String {
    format!("{:x}", md5::compute(input.as_bytes()))
}

/// Generate a prefixed MD5 hash.
///
/// # Example
///
/// ```rust
/// use edgequake_core::utils::prefixed_md5_hash;
///
/// let hash = prefixed_md5_hash("doc", "content");
/// assert!(hash.starts_with("doc-"));
/// ```
pub fn prefixed_md5_hash(prefix: &str, input: &str) -> String {
    format!("{}-{}", prefix, md5_hash(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_hash() {
        let hash1 = md5_hash("hello");
        let hash2 = md5_hash("hello");
        let hash3 = md5_hash("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_prefixed_hash() {
        let hash = prefixed_md5_hash("doc", "content");
        assert!(hash.starts_with("doc-"));
    }
}
