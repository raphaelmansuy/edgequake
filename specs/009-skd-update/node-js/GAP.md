# Node.js / TypeScript SDK gap analysis (`sdks/typescript`)

**Canon:** [API-CANON.md](../API-CANON.md)  
**Package:** `@edgequake/sdk` — see `sdks/typescript/package.json`.

## Summary

| Area | Issue | Resolution |
|------|-------|------------|
| Admin | No resource for `/api/v1/admin/*` | Added `client.admin` (`AdminResource`) |
| Config | No `GET /config/effective` | Added `client.config.effective()` |
| Workspaces | Knowledge injection REST paths not wrapped | Added injection methods on `WorkspacesResource` |
| Health | `ready` / `live` may be plain text | Transport parses JSON or falls back to text wrapper |
| Costs | `pricing` / `estimate` pointed at non-routed `/costs/pricing`, `/costs/estimate` | Fixed to `/api/v1/pipeline/costs/pricing` and `…/estimate`; removed invented `GET /costs/workspace`; paths centralized in `src/constants/api-paths.ts` (shared with `PipelineResource`) |

## Already aligned (reference)

- `documents.upload` → `POST /api/v1/documents`
- `documents.uploadFile` → `POST /api/v1/documents/upload`
- PDF resource: download, retry, cancel, delete, content, progress
- `tenants.getWorkspaceBySlug` — correct tenant-scoped path
- Graph: `stream` SSE, `entities`, `relationships`, `getDegreesBatch`
- Ollama emulation: `client.ollama`

## Cross-reference

- Python: [../python-skd/GAP.md](../python-skd/GAP.md)  
- Rust: [../RUST-SDK-GAP.md](../RUST-SDK-GAP.md)
