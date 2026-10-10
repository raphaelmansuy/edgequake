---
title: Configure LLM providers
description: Choose and connect a cloud or local model server to EdgeQuake, test it, and see how a workspace role turns into a working client.
---

EdgeQuake needs two models. A chat model extracts entities and writes answers. An embedding model turns text into vectors. This section shows how to point each one at a cloud API or a local server, and how to check the link before you ingest documents.

> Product release: v0.32.2. Connections, `edgequake doctor` and `POST /api/v1/providers/test` are new in the next release (v0.33.0, SPEC-163).

## Start in one of four ways

| Path | Command | Use it when |
|------|---------|-------------|
| Docker quickstart | `curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh \| sh` | First look |
| Docker, no prompts | `sh quickstart.sh --yes --provider ollama` | Scripts and CI |
| Source | `make dev` | You change the code |
| Helm | [Deployment](../operations/deployment.md) | Production |

`quickstart.sh` accepts `--provider ollama|openai|omlx|anthropic|lmstudio`, `--base-url URL`, `--model ID` and `--embed-model ID`. Without a terminal it needs `--yes`.

## Which provider should I use?

Answer two questions: may text leave your machine, and what hardware do you have? The chart gives a starting point. You can mix choices, for example a cloud chat model with local embeddings (see [Roles](roles.md)).

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["Start"] --> B{"May text leave your machine?"}
  B -- "No" --> C{"Want a desktop app?"}
  C -- "Yes" --> D["LM Studio"]
  C -- "No" --> E{"Apple Silicon Mac?"}
  E -- "Yes" --> F["Ollama, oMLX or MLX-LM"]
  E -- "No" --> G["Ollama or llama.cpp"]
  B -- "Yes" --> H{"Need Claude models?"}
  H -- "Yes" --> I["Anthropic plus OpenAI embeddings"]
  H -- "No" --> J["OpenAI or another catalog provider"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
class D,F,G,H,I,J eqLlm
```

Read it top to bottom. Answer each diamond and follow the labelled arrow to a box.

## How a workspace role finds its client

A workspace assigns a provider and model to each role. If a role names a saved Connection, EdgeQuake loads it from PostgreSQL, decrypts the key in memory and builds the client. If any step fails, EdgeQuake falls back to the next source of configuration. It does not return an error.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
  participant R as Request
  participant P as Resolver
  participant W as Workspace
  participant D as PostgreSQL
  participant S as Secrets
  participant U as Model server
  R->>P: Query in workspace
  P->>W: Read llm_roles.query
  W-->>P: Model and connection_id
  P->>D: Load provider_connections row
  D-->>P: URL, shape, encrypted key
  P->>S: Decrypt with EDGEQUAKE_SECRETS_KEY
  S-->>P: API key, memory only
  P->>U: Chat call with key
  Note over P,U: Any failure falls back to the next configuration source
```

Read it top to bottom. The key never leaves the server process, and API responses show only a fingerprint.

## Providers

Server defaults come from `EDGEQUAKE_LLM_PROVIDER` and `EDGEQUAKE_LLM_MODEL` for chat, and `EDGEQUAKE_EMBEDDING_PROVIDER` and `EDGEQUAKE_EMBEDDING_MODEL` for embeddings. The full variable list is in the [environment reference](../operations/env-reference.md).

| Provider id | Page | Auth | Default URL used by the client |
|-------------|------|------|-------------------------------|
| `openai` | [OpenAI](openai.md) | `OPENAI_API_KEY` | `https://api.openai.com/v1` |
| `anthropic` | [Anthropic](anthropic.md) | `ANTHROPIC_API_KEY` | `https://api.anthropic.com` |
| `ollama` | [Ollama](ollama.md) | none | `http://localhost:11434` |
| `lmstudio` | [LM Studio](lmstudio.md) | none | `http://localhost:1234` |
| `omlx` | [oMLX](omlx.md) | optional | `http://127.0.0.1:9050` |
| `mlx-lm` | [MLX-LM](mlx-lm.md) | optional | `http://127.0.0.1:8080` |
| `vllm-mlx` | [vLLM-MLX](vllm-mlx.md) | optional | `http://127.0.0.1:8000` |
| `llamacpp` | [llama.cpp](llamacpp.md) | optional | `http://127.0.0.1:8080` |
| `openai-compatible` | [Generic OpenAI shape](openai-compatible.md) | optional | none (you must set it) |

The catalog `edgequake/models.toml` also lists `mistral`, `gemini`, `xai`, `openrouter`, `minimax`, `nvidia`, `cohere`, `jina`, `huggingface`, `vertexai`, `vscode-copilot` and `mtplx`. Most of them read the key from the variable in their `api_key_env` field, for example `MISTRAL_API_KEY` or `GEMINI_API_KEY`. `azure`, `bedrock` and `mock` are in the catalog but disabled.

## Test a server

Run the probe before you save anything. It lists models, sends a short chat request and, for OpenAI-shaped servers, one embedding request. It returns `ok: true` or a `kind`: `unreachable`, `unauthorized`, `model_not_found`, `dim_mismatch`, `shape_mismatch`, `ssrf_denied` or `invalid_url`. The endpoint needs an admin credential. With auth off (dev mode) no header is needed.

```bash
curl -sS -X POST http://127.0.0.1:8080/api/v1/providers/test \
  -H 'Content-Type: application/json' \
  -d '{"shape":"openai_chat","base_url":"http://127.0.0.1:9050","allow_private_network":true}'
```

- In the web UI, open Settings, find **LLM connections** and press **Test connection**.
- From a shell, run `edgequake doctor` (add `--json` for machine output). It exits `0` when all checks pass, `1` when `DATABASE_URL` is missing, and `2` for any other failed check.

## Check the live server with /health

`GET /health` reports the state of the default provider. What it checks depends on the provider:

- **Local servers** (Ollama, LM Studio, oMLX, MLX-LM, vLLM-MLX, llama.cpp, MTPLX) get a live request. `llm_provider` is `false` and the status is `degraded` when they do not answer.
- **Cloud providers** (OpenAI, Anthropic and the others) only report whether the key variable is set. They make no network call.
- **openai-compatible** reports `llm_provider: true` without checking anything. Use the probe above to test it.

```bash
curl -sS http://127.0.0.1:8080/health
```

Look for `"llm_provider": true` under `components`.

## Pages

- [OpenAI](openai.md)
- [Anthropic and Anthropic-shaped servers](anthropic.md)
- [Ollama](ollama.md)
- [LM Studio](lmstudio.md)
- [oMLX](omlx.md)
- [MLX-LM](mlx-lm.md)
- [vLLM-MLX](vllm-mlx.md)
- [llama.cpp](llamacpp.md)
- [Generic OpenAI-shaped server](openai-compatible.md)
- [Workspace roles](roles.md)
- [Provider security](security.md)
