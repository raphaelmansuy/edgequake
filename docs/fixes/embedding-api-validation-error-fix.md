---
title: 'Fix: Embedding API Validation Error'
---

# Fix: Embedding API Validation Error

**Date**: 2026-02-10  
**Issue**: Pipeline processing failed: Embedding error: API error: '$.input' is invalid  
**Status**: ✅ Fixed  
**Commit**: 5b6bcd6a

## Problem

When processing documents, the pipeline occasionally failed with:

```
Pipeline processing failed: Embedding error: API error: '$.input' is invalid. Please check the AP...
```

This error occurred when embedding providers (OpenAI, Ollama, etc.) received arrays containing empty or whitespace-only strings. API validation rejected these invalid inputs.

## Root Cause

The embedding pipeline was passing all text strings to the API without filtering, including:

- Empty strings (`""`)
- Whitespace-only strings (`"   "`, `"\n"`, `"\t"`)
- Strings that became empty after `.trim()`

External APIs (OpenAI, Gemini, Jina, etc.) validate input and reject empty strings in the input array.

## Solution

All embedding providers now:

1. **Filter invalid inputs** before API calls:

   ```rust
   let valid_texts: Vec<(usize, &String)> = texts
       .iter()
       .enumerate()
       .filter(|(_, text)| !text.trim().is_empty())
       .collect();
   ```

2. **Handle all-empty case gracefully**:

   ```rust
   if valid_texts.is_empty() {
       return Ok(vec![vec![0.0; self.embedding_dimension]; texts.len()]);
   }
   ```

3. **Map results back to original indices**:
   ```rust
   let mut result = vec![vec![0.0; self.embedding_dimension]; texts.len()];
   for ((orig_idx, _), embedding) in valid_texts.iter().zip(api_embeddings) {
       result[*orig_idx] = embedding;
   }
   ```

## Affected Providers

All embedding providers were updated:

- ✅ OpenAI (`openai.rs`)
- ✅ Ollama (`ollama.rs`)
- ✅ Gemini (`gemini.rs`)
- ✅ Jina (`jina.rs`)
- ✅ Azure OpenAI (`azure_openai.rs`)
- ✅ LM Studio (`lmstudio.rs`)
- ✅ Mock Provider (`mock.rs`)

## Testing

### Unit Tests

All 201 tests pass:

```bash
cd edgequake
cargo test --package edgequake-llm --lib
# Result: ok. 201 passed; 0 failed; 0 ignored
```

### Manual Testing

1. Start the backend:

   ```bash
   make dev
   ```

2. Upload a problematic PDF document

3. Verify the document processes successfully without embedding errors

4. Check backend logs:
   ```bash
   tail -f /tmp/edgequake-backend.log
   ```

Expected: No "Embedding error" messages, document status shows "Completed"

## Edge Cases Handled

| Input Case          | Behavior                                                  |
| ------------------- | --------------------------------------------------------- |
| All strings valid   | Normal processing, all strings embedded                   |
| Some strings empty  | Empty strings get zero vectors, others processed normally |
| All strings empty   | Return array of zero vectors (dimension-matched)          |
| Whitespace-only     | Treated as empty, receives zero vector                    |
| Mixed valid/invalid | Valid strings embedded, invalid get zero vectors          |

## Performance Impact

- **Negligible overhead**: One additional `filter()` pass over input array
- **API call reduction**: Fewer strings sent to API when some are empty
- **Consistency**: Output array size always matches input array size

## Code Quality

✅ **Clippy**: No warnings  
✅ **Tests**: All 201 tests pass  
✅ **Consistency**: All providers use same pattern

## Future Improvements

Consider:

1. Log warning when many empty strings are filtered (potential data quality issue)
2. Add telemetry to track how often filtering occurs
3. Upstream validation in chunking/extraction to prevent empty strings earlier

## Related Issues

This fix prevents:

- OpenAI API errors: `$.input is invalid`
- Ollama API errors: `invalid input`
- Gemini API errors: `empty text not allowed`

## Verification Checklist

- [x] All providers filter empty strings
- [x] Results mapped back to correct indices
- [x] Zero vectors returned for empty inputs
- [x] Array size consistency maintained
- [x] Tests pass
- [x] Clippy clean
- [x] Documentation updated
