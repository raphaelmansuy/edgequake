# TypeScript / Node SDK

**Location:** `sdks/typescript`

## Install

```bash
cd sdks/typescript && npm install   # or bun install
```

## Example

```typescript
import { EdgeQuakeClient } from "@edgequake/sdk";

const client = new EdgeQuakeClient({
  baseUrl: "http://localhost:8080",
  apiKey: process.env.EDGEQUAKE_API_KEY!,
  tenantId: process.env.EDGEQUAKE_TENANT_ID,
  userId: process.env.EDGEQUAKE_USER_ID,
  workspaceId: process.env.EDGEQUAKE_WORKSPACE_ID,
});

const health = await client.health.check();
console.log(health.status);

const docs = await client.documents.list({
  page: 1,
  page_size: 20,
  document_pattern: "quarterly",
});
console.log(docs.documents.length, docs.status_counts);
```

## Lawful document list filters

`ListDocumentsQuery` supports: `page`, `page_size`, `date_from`, `date_to`, `document_pattern` — matching the Rust `ListDocumentsRequest`.

## See also

- [Quickstart](./quickstart.md)
- `sdks/typescript/README.md`
