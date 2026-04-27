# Python SDK — quickstart

## 1. Install

```bash
cd sdks/python && uv sync   # or pip install -e .
```

## 2. Environment

Export or pass explicitly:

- `EDGEQUAKE_BASE_URL` (default `http://localhost:8080`)
- API key or JWT + `tenant_id`, `user_id`, `workspace_id` as required by your server

## 3. List documents (canon query)

```python
from edgequake import EdgeQuake
from edgequake.types.documents import DocumentListParams

with EdgeQuake(base_url="http://localhost:8080", api_key="sk-…") as c:
    r = c.documents.list(
        params=DocumentListParams(
            page=1,
            page_size=50,
            date_from="2025-01-01T00:00:00Z",
            document_pattern="readme,guide",
        )
    )
    print(r.documents[0].id if r.documents else "empty")
```

Do **not** expect `status=` or `search=` on the wire — those were not API parameters. Use `document_pattern` and dates instead.

## 4. Conversations

```python
from edgequake.types.conversations import ConversationListParams

with EdgeQuake(...) as c:
    page = c.conversations.list(
        params=ConversationListParams(filter_folder_id="uuid", limit=30)
    )
```

## 5. Tests

```bash
cd sdks/python && uv run pytest -q
```
