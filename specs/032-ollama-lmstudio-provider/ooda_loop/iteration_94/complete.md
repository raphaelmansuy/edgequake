# OODA Iteration 94: Test Request Override Priority

## Observe

Testing that request-level provider still takes priority over workspace:

```bash
curl -s -X POST "http://localhost:8080/api/v1/chat/completions" \
  -H "X-Workspace-Id: 9757a55a-1490-458c-9a35-d9e82c833e67" \
  -d '{"message": "test", "stream": false, "provider": "ollama", "model": "gemma3:12b"}'
```

## Orient

Even though workspace is configured for OpenAI, if request explicitly specifies Ollama,
that should be used instead.

## Decide

Run test with explicit provider override.

## Act

**Result**:
```json
{
  "llm_provider": "ollama",
  "llm_model": "gemma3:12b"
}
```

✅ **VERIFIED**: Request-level provider override works correctly

Priority order confirmed:
1. Request provider (ollama) - USED
2. Workspace provider (openai) - SKIPPED
3. Server default - SKIPPED
