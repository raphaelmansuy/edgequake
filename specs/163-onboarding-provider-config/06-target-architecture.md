# 06 — Target architecture

```text
Connection { slug, api_shape, base_url, auth_scheme, encrypted_key, locality }
     │
     ├─ openai_chat / openai_responses  → OpenAIProvider::compatible
     ├─ anthropic_messages              → AnthropicProvider::new + with_base_url
     ├─ ollama                          → OllamaProvider::builder
     └─ native kinds                    → factory create(name, model)

Workspace llm_roles.{extract,query,keyword,summary,vlm} + embedding + vision
     + connection_id (optional; else env/name resolution)
```

Resolution: upload → workspace role → tenant → **connection** → server defaults → env.
