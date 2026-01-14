# OODA Iteration 178 - OpenAI-Compatible API Format

## Observe

### Focus

Verify that OpenAI-compatible API format is used for all providers.

### Investigation

**API Compatibility**:

- Ollama exposes `/v1/chat/completions` endpoint
- LM Studio exposes `/v1/chat/completions` endpoint
- Both follow OpenAI API spec

**Request Format**:

```json
{
  "model": "gpt-4o",
  "messages": [
    { "role": "system", "content": "..." },
    { "role": "user", "content": "..." }
  ],
  "stream": true
}
```

## Orient

### Provider Endpoints

| Provider  | Endpoint               | Format        |
| --------- | ---------------------- | ------------- |
| OpenAI    | api.openai.com/v1/...  | OpenAI        |
| Ollama    | localhost:11434/v1/... | OpenAI-compat |
| LM Studio | localhost:1234/v1/...  | OpenAI-compat |

### Benefits of OpenAI Compatibility

1. **Single implementation**: Same client code
2. **Easy switching**: Just change base URL
3. **Ecosystem compatibility**: Works with tools expecting OpenAI

## Decide

**Status**: ✅ COMPLETE

OpenAI-compatible format used consistently.

## Act

### Verified

- All providers use OpenAI API format
- Same request/response structure
- Easy provider switching
- Consistent interface

---

_Commit: docs(OODA 178): Verify OpenAI-compatible API format_
