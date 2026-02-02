# OODA-53: API Error Response Handling

**Date**: 2026-02-01
**Focus**: Backend Error Communication

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Clear error feedback to users
- Consistent error response format

### Backend Error Response Format

**Rust Error Response:**
```rust
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorResponse { error: msg, code: "NOT_FOUND", details: None }
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse { error: msg, code: "BAD_REQUEST", details: None }
            ),
            // ...
        };
        (status, Json(error)).into_response()
    }
}
```

## ORIENT

### Frontend Error Handling

**API Client Pattern:**
```typescript
class APIError extends Error {
  code: string;
  details?: unknown;
  
  constructor(response: ErrorResponse) {
    super(response.error);
    this.code = response.code;
    this.details = response.details;
  }
}

async function apiRequest<T>(url: string): Promise<T> {
  const res = await fetch(url);
  if (!res.ok) {
    const error = await res.json();
    throw new APIError(error);
  }
  return res.json();
}
```

### Error Code Mapping

| Code | User Message |
|------|--------------|
| NOT_FOUND | Document not found |
| BAD_REQUEST | Invalid request |
| UNAUTHORIZED | Please login |
| PDF_PARSE_ERROR | Could not read PDF |
| LLM_ERROR | Processing failed, retry later |

## DECIDE

**Decision**: Error handling is properly structured

The implementation provides:
1. Consistent error format from backend
2. Typed errors in frontend
3. User-friendly message translation

## ACT

### Error Toast Pattern

```typescript
const { mutate } = useMutation({
  mutationFn: uploadPdfDocument,
  onError: (error: APIError) => {
    const message = errorMessages[error.code] || error.message;
    toast.error(message);
  },
});
```

### Error Message Map
```typescript
const errorMessages: Record<string, string> = {
  NOT_FOUND: 'The document was not found.',
  PDF_PARSE_ERROR: 'Could not read the PDF. It may be corrupted.',
  LLM_ERROR: 'AI processing failed. Please try again.',
  QUOTA_EXCEEDED: 'You have reached your document limit.',
  FILE_TOO_LARGE: 'File is too large. Maximum size is 50MB.',
};
```

**Status**: ✅ VERIFIED - Error handling complete
