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

## Vertex AI (identity auth — not API key)

**Do not** add `api_key_env` for `vertexai`. It uses OAuth2 bearer tokens from identity (ADC, service account, or explicit token).

| Auth mode | Env / mechanism | Use case |
| --------- | --------------- | -------- |
| ADC (local) | `gcloud auth application-default login` | Developer laptop |
| ADC (GCP) | Attached service account → metadata server | Cloud Run / GKE / GCE (**recommended prod**) |
| Service account | `GOOGLE_APPLICATION_CREDENTIALS=/path/to/sa.json` | Off-GCP prod; prefer WIF over raw keys |
| Explicit token | `GOOGLE_ACCESS_TOKEN` | CI / debug (~1 h TTL) |
| Routing | `GOOGLE_CLOUD_PROJECT` (required), `GOOGLE_CLOUD_REGION` (default `us-central1`) | All modes |

Credential gate (extend `providers/credentials.rs`):

```rust
"vertexai" => vertex_auth_configured(), // project + any auth source — see 011-vertexai-authentication.md
```

Factory: prefer `GeminiProvider::from_env_vertex_ai_adc().await` over sync `from_env_vertex_ai()`.

Health message must **not** say `"API key not configured"` — use identity-specific copy.

Full design: [011-vertexai-authentication.md](./011-vertexai-authentication.md).

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
