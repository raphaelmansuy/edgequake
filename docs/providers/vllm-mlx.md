---
title: vLLM-MLX
description: Connect EdgeQuake to a vLLM-MLX server through its OpenAI-shaped API, with the port caveat and the embedding setting.
---

vLLM-MLX is a vLLM-style server for Apple Silicon. EdgeQuake treats it as an OpenAI-shaped local server. This page shows how to connect it and why the port must match in three places.

## When to use

- You run MLX models with a vLLM-style server on a Mac with Apple Silicon.
- You want the same OpenAI-shaped API you use with vLLM elsewhere.

## Configure

```bash
export EDGEQUAKE_LLM_PROVIDER=vllm-mlx
export VLLM_MLX_HOST=http://127.0.0.1:8082
export VLLM_MLX_MODEL=default
```

| Variable | Purpose |
|----------|---------|
| `VLLM_MLX_HOST` | Server URL. Alias `VLLM_MLX_BASE_URL`. |
| `VLLM_MLX_MODEL` | Model name. `default` means the loaded model. |
| `VLLM_MLX_API_KEY` | Optional key. |
| `EDGEQUAKE_EMBEDDING_MODEL` | Embedding model, if offered. Use this variable. `VLLM_MLX_EMBEDDING_MODEL` is not read by the embedding path. |

## Models

- **Chat:** the model your server loaded, or `default`.
- **Embeddings:** set `EDGEQUAKE_EMBEDDING_PROVIDER=vllm-mlx` and `EDGEQUAKE_EMBEDDING_MODEL`. Check vector size with [Roles](roles.md#change-embeddings-with-care).

## Set the port explicitly

The three places that use a port disagree:

| Place | Address |
|-------|---------|
| Client default | `http://127.0.0.1:8000` |
| `/health` and the test probe | `http://127.0.0.1:8082` |
| Your `VLLM_MLX_HOST` | whatever you set |

Start your server on the port you put in `VLLM_MLX_HOST`, so all three agree.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["Start vLLM-MLX server"] --> B{"Server port matches VLLM_MLX_HOST?"}
  B -- "Yes" --> C["Client and health check reach it"]
  B -- "No" --> D["Health check or chat fails"]
  D --> E["Set VLLM_MLX_HOST to the real port"]
  E --> B
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
classDef eqBad fill:#FEE2E2,stroke:#EF4444,color:#7F1D1D
class A eqLlm
class C eqActor
class D eqBad
```

## Verify

```bash
curl "$VLLM_MLX_HOST/v1/models"
curl http://127.0.0.1:8080/health
```

`/health` reports `degraded` when the server does not answer on `/v1/models`. The probe (`POST /api/v1/providers/test` with `"shape":"openai_chat"`) runs the same model list check and a short chat request. See [Test a server](index.md#test-a-server).

## Troubleshoot

- **`/health` is degraded but `curl` works.** `VLLM_MLX_HOST` is not set, so the health check falls back to port 8082 while your server runs elsewhere. Set `VLLM_MLX_HOST` to the real port.
- **Model not found.** Run `curl "$VLLM_MLX_HOST/v1/models"` and copy the id into `VLLM_MLX_MODEL`.
- **Not reachable from Docker.** Replace `127.0.0.1` with `host.docker.internal`.

Related: [Providers overview](index.md), [llama.cpp](llamacpp.md), [MLX-LM](mlx-lm.md).
