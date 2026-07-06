# SPEC-043 — Provider Expansion (models.toml)

## New provider blocks

Add to `edgequake/models.toml` with `enabled = true` and priority ordering:

| Provider | type | api_key_env | default chat | default embed |
| -------- | ---- | ----------- | ------------ | ------------- |
| nvidia | nvidia | NVIDIA_API_KEY | meta/llama-3.1-8b-instruct | nv-embedqa-e5-v2 |
| huggingface | huggingface | HF_TOKEN | meta-llama/Meta-Llama-3-8B-Instruct | — |
| jina | jina | JINA_API_KEY | — | jina-embeddings-v3 |
| cohere | cohere | COHERE_API_KEY | command-r-plus-08-2024 | embed-v4.0 |
| vscode-copilot | vscode-copilot | — | auto | — |
| bedrock | bedrock | (AWS chain) | amazon.nova-lite-v1:0 | amazon.titan-embed-text-v2:0 |

Bedrock: `enabled = false` by default unless `EDGEQUAKE_ENABLE_BEDROCK=1`.

---

## Credential checks

Extend `providers/credentials.rs`:

```rust
"nvidia" => env_nonempty("NVIDIA_API_KEY"),
"cohere" => env_nonempty("COHERE_API_KEY"),
"jina" => env_nonempty("JINA_API_KEY"),
"huggingface" => env_nonempty("HF_TOKEN") || env_nonempty("HUGGINGFACE_TOKEN"),
"vscode-copilot" => vscode_copilot_auth_available(),
"bedrock" => aws_credentials_available(),
```

---

## UI display names

Centralize in `edgequake_webui/src/lib/provider-display.ts`:

```typescript
export const PROVIDER_DISPLAY_NAMES: Record<string, string> = {
  nvidia: "NVIDIA NIM",
  cohere: "Cohere",
  jina: "Jina AI",
  huggingface: "HuggingFace",
  "vscode-copilot": "GitHub Copilot",
  bedrock: "AWS Bedrock",
  vertexai: "Google Vertex AI",
  // ...existing
};
```

---

## Factory alias map

`ProviderCatalog::resolve_id` handles: `hf` → `huggingface`, `copilot` → `vscode-copilot`, `aws-bedrock` → `bedrock`.

EdgeQuake API validation uses `ProviderCatalog::resolve_id` before `create_llm_provider_with_context`.
