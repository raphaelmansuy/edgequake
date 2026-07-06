# SPEC-043 — Vertex AI Authentication (First Principles)

**Date:** 2026-07-06  
**Status:** Implemented (P3.1–P3.4)  
**Symptom:** Provider Status Hub shows Vertex AI **offline** with *"API key not configured"* even when `GOOGLE_APPLICATION_CREDENTIALS` or ADC is set.

---

## First principles

### 1. Two different Google products, two auth models

| Product | Provider ID | Endpoint | Auth model | Env vars |
| ------- | ----------- | -------- | ---------- | -------- |
| **Gemini Developer API** | `google` / `gemini` | `generativelanguage.googleapis.com` | Static API key | `GOOGLE_API_KEY`, `GEMINI_API_KEY` |
| **Vertex AI (enterprise)** | `vertexai` | `{region}-aiplatform.googleapis.com` | **OAuth2 bearer token** (~1 h TTL) minted from **identity** | `GOOGLE_CLOUD_PROJECT`, token source (see ladder) |

**Axiom (SPEC-043 extension):** Never classify `vertexai` as an `api_key_env` provider. It is an **identity-based OAuth2** provider. Conflating it with Gemini Studio keys produces false negatives in health checks and misleading UI copy.

### 2. What “support key, principal, and ADC” actually means

| User term | Google mechanism | EdgeQuake env / path | Production fit |
| --------- | ---------------- | -------------------- | -------------- |
| **Key** | Short-lived `GOOGLE_ACCESS_TOKEN` (or bound API key — see note) | `GOOGLE_ACCESS_TOKEN` | CI/debug only; expires mid-session |
| **Principal** | Service account JSON, WIF config, or attached workload SA | `GOOGLE_APPLICATION_CREDENTIALS` → SA key / WIF JSON; GCE/GKE/Cloud Run metadata | **Preferred for prod** (attached SA > key file) |
| **ADC** | Application Default Credentials search strategy | `gcloud auth application-default login`; well-known ADC file; metadata server | **Preferred for local dev + GCP-hosted prod** |

**Note on “API keys” for Vertex:** Google documents [API keys bound to a service account](https://cloud.google.com/vertex-ai/docs/authentication) for `aiplatform.googleapis.com` only. Google explicitly **does not recommend** this for production — migrate to IAM + short-lived credentials. EdgeQuake should **not** add a `VERTEXAI_API_KEY` env as the primary path.

### 3. Token resolution ladder (edgequake-llm 0.10.1 — code is law)

Upstream `GeminiProvider` implements two constructors:

| Constructor | When to use | Resolution order |
| ----------- | ----------- | ---------------- |
| `from_env_vertex_ai()` (sync) | Legacy / tests | 1. `GOOGLE_ACCESS_TOKEN` → 2. `gcloud auth print-access-token` → 3. `gcloud auth application-default print-access-token` |
| `from_env_vertex_ai_adc()` (async) | **Production target** | 1. `GOOGLE_ACCESS_TOKEN` → 2. **GCE metadata server** (attached SA, auto-refresh) → 3. gcloud CLI fallback |

**Gap today:** `GOOGLE_APPLICATION_CREDENTIALS` (service account JSON) is detected but **not** used to mint tokens in the sync path — edgequake-llm logs a warning and still requires gcloud or metadata. EdgeQuake must not claim “configured” based only on `GOOGLE_APPLICATION_CREDENTIALS` unless runtime can mint a token (track upstream: native SA JWT in edgequake-llm).

**Recommended EdgeQuake factory change:** Route `vertexai` creation through `GeminiProvider::from_env_vertex_ai_adc().await` in resolver/safety_limits (not sync `from_env_vertex_ai()`).

### 4. Required routing env (non-secret)

| Variable | Required | Default | Purpose |
| -------- | -------- | ------- | ------- |
| `GOOGLE_CLOUD_PROJECT` | ✅ | — | GCP project ID |
| `GOOGLE_CLOUD_REGION` | — | `us-central1` | Regional endpoint (`{region}-aiplatform.googleapis.com`) |
| `GOOGLE_CLOUD_LOCATION` | — | alias of region | Some Google SDKs use this name |

IAM: principal needs `roles/aiplatform.user` (or narrower custom role) on the project; Vertex AI API enabled.

---

## Root cause of current bug (Provider Status Hub)

```
models.toml          vertexai.api_key_env = ""     (correct — no static key)
        │
        ▼
check_provider_health()  ──►  api_key_env empty  ──►  env_hint = "API key"
        │                      credentials.rs gate may pass ADC
        ▼
ProviderHealthResponse.error = "API key not configured"   ← WRONG COPY
```

**First-principles fix:** Introduce `CredentialKind::OAuth2Identity` for Vertex (and Bedrock). Health messages and config requirements must describe **identity prerequisites**, not API keys.

---

## Target architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Provider Status Hub / settings UI                                        │
│  vertexai: "Identity auth" — project + (ADC | SA | token)              │
│  gemini:   "API key" — GOOGLE_API_KEY                                   │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────────────┐
│ edgequake-api/providers/credentials.rs                                   │
│  vertex_auth_configured() — sync OR-ladder (project + any auth source)  │
│  vertex_auth_requirements() — structured ConfigRequirement[]            │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────────────┐
│ edgequake-api/handlers/models.rs                                         │
│  check_provider_health(vertexai) — provider-specific message + optional   │
│  async metadata probe for GCP-hosted deployments                        │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
┌───────────────────────────────▼─────────────────────────────────────────┐
│ edgequake-llm GeminiProvider::from_env_vertex_ai_adc()                   │
│  short-lived token + metadata auto-refresh on GCE/GKE/Cloud Run         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Credential satisfaction ladder (EdgeQuake gate)

**Configured** when `GOOGLE_CLOUD_PROJECT` is non-empty **AND** at least one auth source is present:

| Priority | Source | Detection (sync health) | Runtime mint |
| -------- | ------ | ----------------------- | ------------ |
| 1 | Explicit token | `GOOGLE_ACCESS_TOKEN` non-empty | Use as-is (~1 h) |
| 2 | Attached principal | Metadata probe `169.254.169.254` (async health only) | `from_env_vertex_ai_adc()` |
| 3 | ADC file | Well-known path exists (`~/.config/gcloud/application_default_credentials.json` or `$GOOGLE_APPLICATION_CREDENTIALS`) | gcloud ADC or future SA JWT |
| 4 | Service account key | `GOOGLE_APPLICATION_CREDENTIALS` points to existing `.json` | gcloud ADC today; SA JWT upstream |
| 5 | User ADC | `gcloud` on PATH + `application-default print-access-token` succeeds | gcloud CLI |

**Online** (health `available: true`) = configured **AND** (optional) live token probe succeeds.

For local dev without gcloud: show **configured** if project + SA JSON exist, but **offline** with actionable error: *"Service account key set; run `gcloud auth application-default login --impersonate-service-account=...` or deploy on GCP with attached SA"* until upstream SA JWT lands.

---

## Recommended deployment patterns

### A. Local development (human principal)

```bash
export GOOGLE_CLOUD_PROJECT=my-project
export GOOGLE_CLOUD_REGION=us-central1
gcloud auth application-default login
# Optional: impersonate prod SA without key file
gcloud auth application-default login --impersonate-service-account=eq-llm@my-project.iam.gserviceaccount.com
```

### B. CI / short-lived (explicit token)

```bash
export GOOGLE_CLOUD_PROJECT=my-project
export GOOGLE_ACCESS_TOKEN="$(gcloud auth print-access-token)"
```

### C. Production on GCP (attached principal — **best**)

- Cloud Run / GKE / GCE service account with `roles/aiplatform.user`
- **No** `GOOGLE_APPLICATION_CREDENTIALS` in env
- EdgeQuake uses `from_env_vertex_ai_adc()` → metadata server + auto-refresh

### D. Production off GCP (service account key or WIF)

```bash
export GOOGLE_CLOUD_PROJECT=my-project
export GOOGLE_APPLICATION_CREDENTIALS=/run/secrets/gcp-sa.json   # mount, never commit
```

Prefer **Workload Identity Federation** over long-lived SA keys when possible (`GOOGLE_APPLICATION_CREDENTIALS` → WIF config JSON).

### E. Gemini Studio only (not Vertex)

Use provider `gemini`, not `vertexai`:

```bash
export GOOGLE_API_KEY=...
export EDGEQUAKE_LLM_PROVIDER=gemini
```

---

## Implementation tasks

### P3.1 — Credential model (backend)

- [x] Add `CredentialKind` enum: `StaticApiKey | OAuth2Identity | LocalNoAuth | AwsChain`
- [x] `vertex_auth_configured()` in `credentials.rs` — OR-ladder above (replace current incomplete check)
- [x] `vertex_auth_requirements()` — return structured requirements (project required; auth one-of)
- [x] Unit tests: project-only → false; project+token → true; project+SA path → true; gemini still uses API key path

### P3.2 — Health + catalog (backend)

- [x] `check_provider_health`: branch for `vertexai` — never emit `"API key not configured"`
- [x] Error templates:
  - missing project: `"GOOGLE_CLOUD_PROJECT not set"`
  - missing auth: `"No Vertex identity configured (ADC, service account, or GOOGLE_ACCESS_TOKEN)"`
  - SA key without mint path: actionable gcloud/ADC message
- [x] `provider_catalog.rs` `build_config_requirements(vertexai)`: add `GOOGLE_CLOUD_REGION` optional; document ADC path; remove misleading `api_key_env` requirement block

### P3.3 — Runtime factory (backend)

- [x] Resolver/safety_limits: `ProviderType::VertexAI` → `from_env_vertex_ai_adc().await`
- [x] Token refresh: rely on edgequake-llm metadata `vertex_token` RwLock (already in adc path)

### P3.4 — UI (frontend)

- [x] Provider Status Hub: Vertex row shows **"Identity (ADC)"** badge, not "API key"
- [x] Expand panel lists satisfied/missing requirements from `config_requirements` API
- [x] Link to docs anchor `#vertex-ai-authentication`

### P3.5 — Upstream (edgequake-llm)

- [ ] Track: native service-account JWT from `GOOGLE_APPLICATION_CREDENTIALS` without gcloud CLI
- [ ] Track: optional `google-cloud-auth` / `yup-oauth2` for SA impersonation

---

## models.toml (no change to api_key_env)

Keep:

```toml
name = "vertexai"
api_key_env = ""   # intentional — OAuth2 identity, not static key
description = "Google Cloud Vertex AI Gemini (enterprise IAM; ADC or service account)"
```

Add optional metadata block (future):

```toml
[credential]
kind = "oauth2_identity"
required_env = ["GOOGLE_CLOUD_PROJECT"]
auth_sources = ["GOOGLE_ACCESS_TOKEN", "GOOGLE_APPLICATION_CREDENTIALS", "adc", "metadata"]
```

(Defer TOML schema until P3.1 lands — document intent here.)

---

## Verification

```bash
# Local ADC
export GOOGLE_CLOUD_PROJECT=my-project
gcloud auth application-default login
curl -s http://localhost:8080/api/v1/models/health | jq '.[] | select(.name=="vertexai")'

# Expect: available=true OR configured with clear auth error (not "API key")

# Attached SA (on GCE)
# unset GOOGLE_ACCESS_TOKEN GOOGLE_APPLICATION_CREDENTIALS
curl -s http://localhost:8080/api/v1/models/health | jq '.[] | select(.name=="vertexai").health'

# E2E
cd edgequake_webui && pnpm exec playwright test e2e/spec043-llm-model-picker.spec.ts -g vertexai
```

---

## Decision record

| Decision | Choice | Rationale |
| -------- | ------ | --------- |
| Primary prod auth | Attached SA + ADC metadata | Zero long-lived secrets; Google recommended |
| Local dev auth | `gcloud auth application-default login` | Same code path as ADC; no key files |
| Explicit token | Support but don't promote | CI/debug; 1 h TTL |
| Vertex API keys | Document only; do not implement as primary | Google discourages for production |
| Separate from `gemini` | Keep two provider IDs | Different endpoints, quotas, compliance |
| Factory constructor | `from_env_vertex_ai_adc()` | Metadata refresh; matches GCP deployments |
