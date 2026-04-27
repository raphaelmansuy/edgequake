# EdgeQuake SDKs

Official HTTP clients for the EdgeQuake API. **Canonical routing** lives in `edgequake/crates/edgequake-api/src/routes.rs`; OpenAPI is the field-level truth for DTOs.

Use these docs for **copy-paste examples** and day-to-day integration. For honest gaps and parity, read [Brutal assessment](./BRUTAL-ASSESSMENT.md) and the spec tracker [SDK-API-COVERAGE.md](../../specs/009-skd-update/SDK-API-COVERAGE.md).

## By language

| Tier | SDK | Folder | Package / crate |
|------|-----|--------|-----------------|
| 1 | Rust | [rust](./rust/) | `sdks/rust` — `edgequake-sdk` |
| 1 | Python | [python](./python/) | `sdks/python` — `edgequake` |
| 1 | TypeScript / Node | [typescript](./typescript/) | `sdks/typescript` — `@edgequake/sdk` |
| 2 | Kotlin/JVM | [kotlin](./kotlin/) | `sdks/kotlin` |
| 2 | Swift | [swift](./swift/) | `sdks/swift` |
| 2 | Go | [go](./go/) | `sdks/go` |
| 2 | Java | [java](./java/) | `sdks/java` |
| 2 | C# / .NET | [csharp](./csharp/) | `sdks/csharp` |
| — | Ruby | [ruby](./ruby/) | `sdks/ruby` (layout incomplete — see assessment) |

## Headers and tenancy (all SDKs)

Most `/api/v1/*` calls expect workspace context:

- `Authorization: Bearer <jwt>` (or API key per server config)
- `X-Tenant-ID`, `X-User-ID`, `X-Workspace-ID` as required by your deployment

Configure these on the client **once**; every resource reuses the same transport.

## Quick actions

1. **Health** — `GET /health` (unversioned) before anything else.
2. **List documents** — `GET /api/v1/documents` with optional `page`, `page_size`, `date_from`, `date_to`, `document_pattern`.
3. **Conversations** — list uses cursor filters (`filter[folder_id]`, etc.); bulk delete body uses **`conversation_ids`**; response uses **`affected`**.

## See also

- [REST API overview](../api-reference/rest-api.md)
- [Multi-tenant tutorial](../tutorials/multi-tenant.md)
