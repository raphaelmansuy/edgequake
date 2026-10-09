# 04 — Current architecture (pre-change snapshot)

Provider construction: `ProviderFactory::from_env()` at boot. Workspace metadata holds provider/model names. `models.toml` is descriptive; `api_base` is ignored.

Roles: extract/query/keyword/summary/vlm via `llm_roles`; embedding and vision as sibling fields; reranker and decision env-only.

Secrets: env only. No encryption. No SSRF. Quickstart: `DEV_MODE=true`, auth off, `0.0.0.0:8080`.
