# LENS — Model

Parent: [04 Gap matrix](../04-gap-matrix.md) · Probe: [07](../07-derisk-results.md)

## The path that worked

`POST https://<workspace>/ai-gateway/mlflow/v1/chat/completions`  
`POST https://<workspace>/ai-gateway/mlflow/v1/embeddings`

That is the same shape `edgequake-llm` already posts, when `base` is `https://<workspace>/ai-gateway/mlflow/v1`.

| Call | Result |
|------|--------|
| `system.ai.llama-4-maverick` chat | 200, `content` string, body contained `pong` |
| `system.ai.qwen3-next-80b-a3b-instruct` chat | 200, `content` string |
| `system.ai.gpt-oss-20b` chat | 200, `content` is a list of `reasoning` parts |
| `system.ai.gte-large-en` embeddings | 200, 1024 floats |
| `system.ai.gte_large_en_v1_5` | 404 |
| Legacy endpoint name on the Gateway path | 403, use a Unity Catalog model service |
| `POST /serving-endpoints/databricks-gemma-3-12b/invocations` | 200, string `content` |

## What the client will do

`MessageContent.content` is `Option<String>`. Llama and Qwen responses can deserialize. The `gpt-oss-20b` response cannot. The default Databricks profile uses a model that returns a string. `gpt-oss` waits until the struct accepts a list and a string.

Embeddings must be pinned to `system.ai.gte-large-en` (or another model measured at the same dimension) and `EDGEQUAKE_EMBEDDING_DIMENSION=1024`. GTE large is documented as not normalized. Cosine distance still runs. Quality versus a normalized 1536 model is out of this study.

The bearer is a Databricks token, refreshed on the same clock as the database credential. A static `OPENAI_API_KEY` in `app.yaml` violates LAW-165-2.

## Gaps

G7, G8, G9 in [04](../04-gap-matrix.md).
