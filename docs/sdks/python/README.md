# Python SDK

**Location:** `sdks/python`  
**Install:** `pip install edgequake` or `uv add edgequake` from the package root.

## 30-second example

```python
from edgequake import EdgeQuake
from edgequake.types.documents import DocumentListParams

client = EdgeQuake(
    base_url="http://localhost:8080",
    api_key="YOUR_KEY",
    tenant_id="…",
    user_id="…",
    workspace_id="…",
)

assert client.health().status == "healthy"

# Document list: only lawful query keys (see ListDocumentsRequest in the API)
page = client.documents.list(
    params=DocumentListParams(page=1, page_size=20, document_pattern="report")
)
print(len(page.documents), page.total, page.has_more)

client.close()
```

## Async

Use `AsyncEdgeQuake` the same way with `await client.documents.list(...)`.

## See also

- [Quickstart](./quickstart.md)
- In-repo reference: `sdks/python/README.md`, `sdks/python/docs/API.md`
