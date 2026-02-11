# Q: How does workspace selection in EdgeQuake compare to LightRAG?

**Q:** In LightRAG, you can set a `workspace` parameter (or `POSTGRES_WORKSPACE` env var) to isolate data in PostgreSQL. I can't find an equivalent in EdgeQuake's configuration docs. Is this a missing feature, or is workspace selection handled differently?

**A:**

EdgeQuake supports workspaces, but the mechanism is more advanced and API-driven compared to LightRAG:

- **Workspaces are first-class entities**: Each workspace is a row in the `workspaces` table (with a UUID), belonging to a tenant. You can create, list, update, and delete workspaces via the REST API.
- **Workspace selection is per-request**: Instead of a global env var, you specify the workspace for each API call using the `X-Workspace-ID` HTTP header. Example:
  ```bash
  curl http://localhost:8080/api/v1/documents \
    -H "X-Workspace-ID: <workspace-uuid>"
  ```
- **Per-workspace config**: Each workspace can have its own LLM provider, embedding model, and settings.
- **Row-Level Security (RLS)**: PostgreSQL enforces workspace isolation at the database level.
- **Default workspace**: A default workspace is auto-created at startup for convenience.

**Summary:**
- There is no `POSTGRES_WORKSPACE` or `EDGEQUAKE_WORKSPACE` env var in EdgeQuake. Instead, workspace selection is handled at the API level via the `X-Workspace-ID` header, enabling true multi-tenant, multi-workspace operation.
- See `docs/cookbook.md` for usage examples.

---

**References:**
- [docs/cookbook.md](../docs/cookbook.md)
- [edgequake/crates/edgequake-api/src/middleware.rs](../edgequake/crates/edgequake-api/src/middleware.rs)
- [LightRAG README](https://github.com/HKUDS/LightRAG#data-isolation-between-lightrag-instances)
