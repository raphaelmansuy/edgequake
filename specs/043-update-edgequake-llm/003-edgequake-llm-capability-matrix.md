# SPEC-043 — edgequake-llm Capability Matrix (0.6.26 → 0.10.0)

## Version pins

| Location | Before | After |
| -------- | ------ | ----- |
| `edgequake/Cargo.toml` | `0.6.26` | `0.10.0` |
| `edgequake-pdf2md` (transitive) | `0.6.20` | unchanged until `0.9.3` |

---

## New in 0.10.0 (EdgeQuake must expose)

| Feature | Rust API | EdgeQuake surface |
| ------- | -------- | ----------------- |
| Application attribution | `ApplicationContext`, `create_llm_provider_with_context` | `/settings/attribution`, `/health` |
| Provider catalog | `ProviderCatalog::all()` | `/settings/provider-catalog` |
| Capability search | `find_static_models`, `CapabilityFilter` | `GET /models/search` |
| Name/fuzzy search | `search_static_models`, `ModelSearchQuery` | `GET /models/search?q=` |
| Cohere | `ProviderType::Cohere` | `models.toml` + UI icon |
| Ollama Cloud | `OllamaProvider::from_env_cloud()` | env docs + health |
| Bedrock (feature) | `features = ["bedrock"]` | optional `models.toml` |

---

## Provider coverage matrix

| Provider | 0.6.26 factory | 0.10.0 catalog | models.toml (before) | Action |
| -------- | -------------- | -------------- | -------------------- | ------ |
| openai | ✅ | ✅ Full attribution | ✅ | keep |
| anthropic | ✅ | ✅ Full | ✅ | keep |
| gemini | ✅ | ✅ Full | ✅ | keep |
| vertexai | ✅ | ✅ Full | ✅ | keep — **identity auth (ADC/SA), not API key** — see 011 |
| mistral | ✅ | ✅ Full | ✅ | keep |
| azure | ✅ | ✅ Full | ✅ | keep |
| xai | ✅ | ✅ Passthrough | ✅ | keep |
| openrouter | ✅ | ✅ Full | ✅ | keep |
| ollama | ✅ | ✅ Passthrough | ✅ | keep |
| lmstudio | ✅ | ✅ Passthrough | ✅ | keep |
| minimax | via compat | via compat | ✅ | keep |
| mock | ✅ | ✅ None | ✅ | keep |
| **nvidia** | ✅ | ✅ Full | ❌ | **add** |
| **huggingface** | ✅ | ✅ Passthrough | ❌ | **add** |
| **jina** | embed only | ✅ Passthrough | ❌ | **add** |
| **cohere** | ❌ | ✅ Full | ❌ | **add** |
| **vscode-copilot** | ✅ | ObservabilityOnly | ❌ | **add** |
| **bedrock** | feature | ✅ Full | ❌ | **add (optional)** |

---

## AttributionSupport → HTTP mapping (from edgequake-llm)

| Provider | Level | Key headers / body fields |
| -------- | ----- | ------------------------- |
| OpenAI | Full | `X-Client-Request-Id`, body `user` |
| Azure | Full | `application_name`, `end_user_id`, `x-ms-client-request-id` |
| Anthropic | Full | `x-request-id`, `x-application-id` |
| Gemini/Vertex | Full | `x-goog-api-client`, `x-request-id` |
| OpenRouter | Full | `HTTP-Referer`, `X-OpenRouter-Title` |
| Mistral/Cohere | Full | `X-Client-Name` |
| NVIDIA | Full | `X-Request-Id` |
| Bedrock | Full | body `app`, `tenant_id` |
| Ollama/LM Studio/xAI/HF | Passthrough | OpenAI-family headers |
| VS Code Copilot | ObservabilityOnly | span attrs only |

Ingress headers (EdgeQuake → context):

- `x-edgequake-app-id`, `x-edgequake-app-name`, `x-edgequake-app-url`
- `x-edgequake-tenant-id`, `x-edgequake-request-id`
- Env: `EDGEQUAKE_APP_ID`, `EDGEQUAKE_APP_NAME`, `EDGEQUAKE_APP_URL`
