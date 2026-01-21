# OODA 72 - Orient: API Error Handling Test Strategy

## Analysis

### API Error Response Patterns

From the codebase, API errors return structured JSON:

```json
{
  "error": "error_code",
  "message": "Human readable message"
}
```

### Error Codes

- 400: Bad Request (validation errors)
- 404: Not Found (invalid IDs)
- 500: Internal Server Error

### Test Cases for Hardening

1. **Invalid tenant ID**

   - GET `/api/v1/tenants/{invalid-uuid}`
   - Expected: 404 with "not found" message

2. **Invalid workspace ID**

   - GET `/api/v1/tenants/{tenant}/workspaces/{invalid-uuid}`
   - Expected: 404 with "not found" message

3. **Models API robustness**
   - GET `/api/v1/models/{unknown-provider}`
   - Expected: 404 or empty response

## Recommendation

Add 2 error handling tests:

1. "invalid tenant ID returns 404"
2. "invalid workspace ID returns 404"

These validate the API gracefully handles invalid inputs.
