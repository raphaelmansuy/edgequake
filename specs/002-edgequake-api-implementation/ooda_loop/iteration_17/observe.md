# Iteration 17: Observe — TypeScript & Python SDK Fix

## Observations

- TypeScript SDK chat types used OpenAI-format `messages` array instead of EdgeQuake `message` string
- Python SDK chat.complete() used `messages` parameter; needs `message` (singular)
- Both SDKs had conversation/folder E2E tests that were skipped due to missing tenant/user defaults
- Default migration tenant ID: `00000000-0000-0000-0000-000000000002`
- Default migration user ID: `00000000-0000-0000-0000-000000000001`
- EdgeQuake chat response: `{conversation_id, content, sources, stats}` — not OpenAI choices format
