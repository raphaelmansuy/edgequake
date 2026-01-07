# OODA Iteration 10 - Create file_validation Module

## Observe

- Re-read mission file (every 5 iterations checkpoint)
- Analyzed handler file sizes - documents.rs at 3,639 lines
- Found 2 duplicated file validation patterns (lines 1848-1876 and 2321-2354)
- Pattern includes: size check, extension validation, UTF-8 conversion, MIME type

## Orient

- File validation is a common cross-cutting concern
- Same allowed extensions list duplicated (`txt`, `md`, `json`, `csv`, `html`, `htm`, `xml`, `yaml`, `yml`)
- Same MIME type mapping duplicated
- Opportunity for a focused `file_validation.rs` module

## Decide

1. Create `file_validation.rs` with:
   - `ALLOWED_EXTENSIONS` constant
   - `validate_file_size()` function
   - `validate_extension()` function
   - `validate_utf8()` function
   - `get_mime_type()` function
   - `validate_file()` comprehensive function
2. Add 14 unit tests
3. Apply to both handlers in documents.rs

## Act

### Created file_validation.rs (215 lines)

```rust
pub const ALLOWED_EXTENSIONS: [&str; 9] = [
    "txt", "md", "json", "csv", "html", "htm", "xml", "yaml", "yml",
];

pub fn validate_file(
    filename: &str,
    content: &[u8],
    max_size: usize,
) -> ApiResult<(String, String, &'static str)> {
    validate_file_size(content.len(), max_size)?;
    let extension = validate_extension(filename)?;
    let text_content = validate_utf8(content)?;

    if text_content.trim().is_empty() {
        return Err(ApiError::ValidationError("File content cannot be empty".to_string()));
    }

    let mime_type = get_mime_type(&extension);
    Ok((extension, text_content, mime_type))
}
```

### Applied to documents.rs

Before (duplicated 2x):

```rust
// ~25 lines of validation logic each
if content.len() > max_size { ... }
let extension = filename.rsplit('.').next()...
let allowed_extensions = [...]
if !allowed_extensions.contains(...) { ... }
let text_content = String::from_utf8(...)?;
if text_content.trim().is_empty() { ... }
let mime_type = match extension.as_str() { ... }
```

After (1 line each):

```rust
let (_extension, text_content, mime_type) =
    validate_file(&filename, &content, state.config.max_document_size)?;
```

## Metrics

| Metric                | Before | After | Change      |
| --------------------- | ------ | ----- | ----------- |
| documents.rs lines    | 3,639  | 3,572 | -67 (-1.8%) |
| file_validation.rs    | 0      | 215   | +215 (new)  |
| file_validation tests | 0      | 14    | +14         |
| Total API tests       | 108    | 122   | +14         |

## Test Results

- file_validation module: 14/14 passed ✅
- edgequake-api lib: 122/122 passed ✅
- clippy: 0 warnings ✅

## Commit

`de2fe39` - refactor(api): Create file_validation module for DRY file handling
