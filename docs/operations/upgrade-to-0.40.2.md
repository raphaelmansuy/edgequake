---
title: "Upgrade to EdgeQuake v0.40.2"
description: "Upgrade from v0.40.1 to v0.40.2. Schema stays 227."
---

# Upgrade to EdgeQuake v0.40.2

> **From:** v0.40.1 (schema **227**) · **To:** v0.40.2 (schema **227**)

This patch adds no migration. `edgequake migrate` is a no-op on a database that is already at schema 227. Roll the images. Do not change the schema.

## Order of operations

```bash
# 1. Roll the API replicas to 0.40.2.
# 2. Roll the WebUI to 0.40.2.
```

You do not need a backup for a schema change. This cut does not write schema. Take a backup if your own policy requires one before an image roll.

## What changes for an operator

**Vertex AI can hold its own credential.** Deployment › Connections › Vertex AI now offers three choices:

| Credential | What it means |
| ---------- | ------------- |
| This server's Google Cloud identity | Nothing is stored. Calls use ADC, the attached service account, or `gcloud`. This is what v0.40.1 did. |
| Service account JSON | Paste the key file for an account such as `vertex-express@PROJECT.iam.gserviceaccount.com`. It is encrypted at rest and never returned; chat mints an access token from it. |
| API key | A Vertex Express mode key, or a Google Cloud API key bound to a service account. |

A service-account file makes a connection independent of the machine's ADC, which matters when `gcloud auth activate-service-account` and `application_default_credentials.json` belong to different principals. Set **Location** (`us-central1`, `global`) on the connection. The project comes from the file's `project_id`.

**Three provider endpoints moved.** If you stored a base URL by hand for any of these, replace it:

| Provider | Replace | With |
| -------- | ------- | ---- |
| Hugging Face | `https://api-inference.huggingface.co` | `https://router.huggingface.co/v1` (the old host returns 410 Gone) |
| Cohere | `https://api.cohere.com/v1` | `https://api.cohere.ai/compatibility/v1` (the `/v1` route is Cohere's native shape) |
| Azure OpenAI | `https://RESOURCE.openai.azure.com` | `https://RESOURCE.openai.azure.com/openai/v1` (Foundry v1; Azure normalises a missing or doubled suffix) |

An existing stored row keeps its stored value. Nothing rewrites your database.

**Test connection no longer lists models where none are listed.** Cohere's compatibility surface has no `GET /models`, and Azure's model list is control-plane. Both were reported as failing keys; they now test over the chat path.

**Base URL is read-only for Gemini, Vertex AI, Bedrock and the Copilot proxy.** Those clients build the endpoint themselves, so the field no longer invited an edit that did nothing.

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/packs/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
