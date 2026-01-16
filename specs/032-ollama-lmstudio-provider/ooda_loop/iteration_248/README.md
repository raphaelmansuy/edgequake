# OODA-248: Path Traversal Vulnerability Audit

## Observe

Audited file path handling in the API for directory traversal vulnerabilities.

### CRITICAL FINDING: Path Traversal Vulnerability

| Endpoint | Location | Risk Level |
|----------|----------|------------|
| `/api/v1/documents/scan` | `documents.rs:2437` | **HIGH** |

### Vulnerable Code (BEFORE FIX)

```rust
// documents.rs:2437
let base_path = Path::new(&request.path);

// No validation! User can pass:
// - "/etc/passwd"
// - "../../../etc/passwd"  
// - "/root/.ssh/id_rsa"
```

### Missing Protections

1. ❌ No allowed paths/directories configuration
2. ❌ No path canonicalization
3. ❌ No `..` traversal detection
4. ❌ No symlink resolution check
5. ❌ No chroot/sandbox

## Orient

### Attack Scenarios

1. **Direct Path Access**
   ```json
   {"path": "/etc/passwd"}
   ```
   Returns file listing of system directories.

2. **Traversal Attack**
   ```json
   {"path": "../../../etc"}
   ```
   Could escape any intended directory.

3. **Sensitive File Enumeration**
   ```json
   {"path": "/root/.ssh"}
   ```
   Could expose SSH keys, configs.

### Risk Assessment

| Factor | Rating |
|--------|--------|
| Exploitability | HIGH - Simple HTTP request |
| Impact | HIGH - Full filesystem read |
| Authentication Required | Yes (tenant auth) |
| CVSS Estimate | 7.5-8.5 (High) |

## Decide

Add path traversal protection:

1. **Add allowed paths configuration** in `ServerConfig`
2. **Canonicalize paths** before use
3. **Validate paths** against allowed list
4. **Detect traversal patterns** (`..`, symlinks)

## Act

### Files Created/Modified

| File | Change |
|------|--------|
| `path_validation.rs` | NEW - Path traversal protection module |
| `lib.rs` | Added `pub mod path_validation` |
| `state.rs` | Added `path_validation_config` to `AppState` |
| `state.rs` | Added `load_path_validation_config()` for env-based config |
| `documents.rs` | Updated `scan_directory` to use validated paths |
| `Cargo.toml` | Added `tempfile` dev dependency for tests |

### Security Controls Implemented

1. **Path Canonicalization**: `safe_canonicalize()` resolves `.` and `..`
2. **Traversal Pattern Detection**: `contains_traversal_pattern()` blocks encoded attacks
3. **Allowed Path Validation**: Only configured directories permitted
4. **Depth Limiting**: Prevents deeply nested paths
5. **Symlink Blocking**: Prevents symlink-based escapes

### Configuration

Environment variables for production:

```bash
# Allow specific directories (recommended)
ALLOWED_SCAN_PATHS=/data/uploads:/home/user/documents

# Allow any path (NOT RECOMMENDED - development only)
ALLOW_ANY_SCAN_PATH=true
```

### Default Security Posture

| Mode | Default |
|------|---------|
| Memory/Dev | Permissive (allow_any_path=true) |
| PostgreSQL | Secure (require explicit paths) |

## Metrics

| Metric | Value |
|--------|-------|
| Tests added | 6 |
| Tests passing | 421 |
| LOC added | ~200 |
| Security controls | 5 |

## Conclusion

✅ **PATH TRAVERSAL VULNERABILITY FIXED**

- Created comprehensive path validation module
- Added 6 unit tests for security controls
- Integrated with AppState and scan_directory handler
- Secure defaults for production deployment
