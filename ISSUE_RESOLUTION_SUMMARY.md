# Issue Resolution Summary: Can't Import Documents

## Problem Statement

Users reported errors when trying to upload documents using the REST API:

```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: multipart/form-data" \
  -F "file=@document.pdf"
```

**Errors Encountered**:
- "Expected request with `Content-Type: application/json`"
- "Failed to parse the request body as JSON: invalid number at line 1 column 2"
- "missing field `content`"

## Root Cause Analysis

**The Issue**: Documentation/Implementation Mismatch

- **Documentation** (README, tutorials) showed using `-F "file=@..."` with `/api/v1/documents`
- **Implementation**: This endpoint only accepts JSON (`Content-Type: application/json`)
- **Correct Endpoint**: File uploads should use `/api/v1/documents/upload` (multipart/form-data)

The API has two separate endpoints with different purposes:
1. `POST /api/v1/documents` - For JSON text content uploads
2. `POST /api/v1/documents/upload` - For file uploads (multipart form-data)

Users were trying to use multipart form-data on the JSON-only endpoint because the documentation was inconsistent.

## Solution Implemented

### 1. Documentation Fixes (6 files updated)

**README.md**:
```bash
# OLD (incorrect)
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: multipart/form-data" \
  -F "file=@your-document.pdf"

# NEW (correct)
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@your-document.pdf"
```

**docs/api-reference/rest-api.md**:
- Split into two clear sections
- `POST /api/v1/documents` - JSON only
- `POST /api/v1/documents/upload` - Multipart only

**docs/tutorials/document-ingestion.md**:
- Fixed 5 curl examples to use `/documents/upload`

**docs/tutorials/pdf-ingestion.md**:
- Fixed all curl examples (bulk replace)

**docs/troubleshooting/common-issues.md**:
- Added new section #1 "Document Upload Errors"
- Documented both error messages from the issue
- Table showing which endpoint/content-type to use
- ✅ CORRECT vs ❌ WRONG examples
- Fixed ~20 other examples throughout the doc

### 2. New Quick Reference Guide

Created **docs/api-reference/document-upload-quick-reference.md** with:

- **Decision Tree**: Helps users choose the right endpoint
- **4 Upload Methods Documented**:
  1. JSON text upload (`/api/v1/documents`)
  2. Single file upload (`/api/v1/documents/upload`)
  3. Batch file upload (`/api/v1/documents/upload/batch`)
  4. Directory scan (`/api/v1/documents/scan`)
- **Common Errors**: All errors from the issue with fixes
- **API Summary Table**: Quick reference

### 3. Tests Added

Added two tests to **edgequake/crates/edgequake-api/tests/e2e_documents.rs**:

```rust
#[tokio::test]
async fn test_upload_document_rejects_multipart() {
    // Documents that /api/v1/documents returns 415 for multipart
    // This prevents future regressions
}

#[tokio::test]
async fn test_upload_endpoint_accepts_multipart() {
    // Documents that /api/v1/documents/upload accepts multipart
    // This serves as usage documentation
}
```

## Verification

### Correct Usage Examples

**Upload Text/JSON**:
```bash
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Your document text here...",
    "title": "Document Title"
  }'
```

**Upload File (PDF, TXT, MD)**:
```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@document.pdf" \
  -F "title=My Document"
```

### Expected Behavior

| Endpoint                        | Content-Type         | Method      | Expected Result      |
| ------------------------------- | -------------------- | ----------- | -------------------- |
| `/api/v1/documents`             | `application/json`   | `-d '{...}'`| ✅ 201 CREATED       |
| `/api/v1/documents`             | `multipart/form-data`| `-F "file="`| ❌ 415 UNSUPPORTED   |
| `/api/v1/documents/upload`      | `multipart/form-data`| `-F "file="`| ✅ 201 CREATED       |
| `/api/v1/documents/upload`      | `application/json`   | `-d '{...}'`| ❌ 400 BAD REQUEST   |

## Impact

**Before**: Users couldn't upload documents and received confusing error messages

**After**: 
- Clear documentation showing correct endpoint for each upload type
- Troubleshooting guide with exact error messages and solutions
- Quick reference guide for easy API lookup
- Tests preventing future documentation/code divergence

## References

- **Issue**: Can't import documents
- **PR**: Fix document upload API documentation
- **Files Changed**: 7 files (6 docs + 1 test)
- **Lines Added**: ~500 lines of documentation
- **New Guide**: document-upload-quick-reference.md (310 lines)

## Lessons Learned

1. **Documentation must match implementation** - Inconsistent docs cause user confusion
2. **API design matters** - Having clear, distinct endpoints prevents ambiguity
3. **Examples are critical** - Users copy-paste examples, so they must be correct
4. **Troubleshooting guides save time** - Documenting exact error messages helps users self-solve
5. **Tests as documentation** - Tests that demonstrate correct usage prevent regressions
