---
title: "Local extract reliability (Ollama / LM Studio)"
description: "Make local LLM extraction reliable under load."
---

# Local extract reliability (Ollama / LM Studio)

Quick ops runbook when KG extraction stalls with `Network error … /api/chat` or
`Local inference gate saturated` under a single-slot Ollama runner (`-np 1`).

## Recommended local profile

| Knob | Value | Why |
|------|-------|-----|
| `OLLAMA_CONTEXT_LENGTH` | `8192` | Avoid 128k runner cost on extract |
| `EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS` | `1` | Match Ollama serial slot |
| `EDGEQUAKE_PROVIDER_BUDGET` / `EDGEQUAKE_LOCAL_MAX_INFLIGHT` | `1` | Gate admits one in-flight chat |
| `EDGEQUAKE_EXTRACT_REASONING_EFFORT` | `none` | Forces `think:false` / `reasoning:"off"` |
| Extract model | `gemma4:latest` (or cloud) | Prefer over 35B for bulk PDFs |

`make dev` / `make backend-bg` export these defaults for ollama/lmstudio providers.

## Unblock a stuck document

1. Restart backend so think-off + admission clamps are live:
   ```bash
   make stop
   export OLLAMA_CONTEXT_LENGTH=8192
   export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=1
   export EDGEQUAKE_PROVIDER_BUDGET=1
   export EDGEQUAKE_EXTRACT_REASONING_EFFORT=none
   make backend-bg   # or make dev
   ```
2. Prefer a smaller extract model on the workspace (Settings → models) if still on `qwen3.6:35b*`.
3. Cancel the stuck track in Documents → Active run, then requeue / re-upload.
4. Confirm health: `curl -s http://localhost:8090/health | jq .providers`

## Success signals

- Near-zero `Network error` under single-doc local extract with healthy Ollama.
- Gate saturation becomes wait + heartbeat (`Extracting … in flight`), not connection storms.
- Outbound Ollama body includes `"think": false` and `options.num_ctx` ≤ configured ctx.
