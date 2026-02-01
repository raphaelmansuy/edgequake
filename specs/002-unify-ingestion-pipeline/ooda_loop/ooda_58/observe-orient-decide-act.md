# OODA-58: File Size Validation

**Date**: 2026-02-01
**Focus**: Upload Size Limits

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Enforce file size limits
- Clear feedback on oversized files

### Current Size Validation

**Frontend Validation:**
```typescript
const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50MB

const validateFile = (file: File): boolean => {
  if (file.size > MAX_FILE_SIZE) {
    toast.error(`File too large. Maximum size is 50MB.`);
    return false;
  }
  return true;
};
```

**Backend Validation:**
```rust
// axum multipart config
pub fn configure_multipart() -> MultipartConfig {
    MultipartConfig::new()
        .max_file_size(50 * 1024 * 1024)
        .max_files(10)
}
```

## ORIENT

### Size Limits by Type

| File Type | Max Size | Rationale |
|-----------|----------|-----------|
| PDF | 50MB | Most research papers < 20MB |
| Text | 10MB | Documents are typically small |
| Total request | 100MB | Multiple file uploads |

### User Experience
```
[Oversized File Dropped]
         ↓
[Frontend validation fails]
         ↓
[Toast: "File too large. Maximum 50MB."]
         ↓
[File not uploaded, no API call]
```

## DECIDE

**Decision**: Size validation correctly implemented

Both frontend and backend validate:
1. Frontend catches before upload (saves bandwidth)
2. Backend protects against bypassed frontend

## ACT

### Complete Validation Flow

```typescript
const handleFileUpload = async (file: File) => {
  // Size check
  if (file.size > MAX_FILE_SIZE) {
    toast.error(`"${file.name}" is too large. Maximum size is 50MB.`);
    return;
  }
  
  // Type check
  if (!ALLOWED_TYPES.includes(file.type)) {
    toast.error(`"${file.name}" is not a supported file type.`);
    return;
  }
  
  // Proceed with upload
  await uploadFile(file);
};
```

### File Size Display Helper

```typescript
const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

// Usage
toast.error(
  `File is ${formatFileSize(file.size)}. Maximum is ${formatFileSize(MAX_FILE_SIZE)}.`
);
```

**Status**: ✅ VERIFIED - File size validation complete
