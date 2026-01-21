# OODA Iteration 91: Observe

## Bug Report: Workspace LLM Provider Not Used

### Symptom

User reports that when creating a Tenant/Workspace with OpenAI selected, the system shows error:
> "Cannot use provider 'openai': Configuration error: OPENAI_API_KEY is empty or invalid"

### Evidence Gathered

1. **Workspace Configuration Check**:
   ```json
   {
     "id": "9757a55a-1490-458c-9a35-d9e82c833e67",
     "llm_provider": "openai",
     "llm_model": "gpt-4.1-mini"
   }
   ```
   - Workspace IS correctly configured with OpenAI

2. **Backend Health Check**:
   ```json
   {
     "status": "healthy",
     "llm_provider_name": "ollama"
   }
   ```
   - Server default is Ollama, not OpenAI

3. **API Request Without Provider**:
   ```bash
   curl -X POST "/api/v1/chat/completions" \
     -H "X-Workspace-Id: 9757a55a-1490-458c-9a35-d9e82c833e67" \
     -d '{"message": "test", "stream": false}'
   ```
   - Response: `{"llm_provider": null, "llm_model": null}`

### Root Cause Hypothesis

The chat handler only uses provider from:
1. Request parameters (`request.provider`, `request.model`)
2. Server default

**Missing**: Workspace-configured provider as fallback
