# Authentication Guide — @edgequake/sdk

## API Key Authentication

The simplest way to authenticate. Recommended for server-side applications.

```typescript
import { EdgeQuake } from "@edgequake/sdk";

const client = new EdgeQuake({
  baseUrl: "http://localhost:8080",
  apiKey: process.env.EDGEQUAKE_API_KEY,
});
```

The API key is sent as a `Bearer` token in the `Authorization` header:

```
Authorization: Bearer your-api-key
```

## JWT Authentication

For browser-based or user-session applications.

```typescript
// 1. Login to get JWT
const client = new EdgeQuake({ baseUrl: "http://localhost:8080" });

const { access_token, refresh_token } = await client.auth.login({
  username: "user@example.com",
  password: "secure-password",
});

// 2. Create authenticated client
const authedClient = new EdgeQuake({
  baseUrl: "http://localhost:8080",
  jwt: access_token,
});

// 3. Refresh when expired
const { access_token: newToken } = await client.auth.refresh(refresh_token);
```

## Multi-Tenant Headers

For multi-tenant deployments. Adds `X-Tenant-Id` and `X-Workspace-Id` headers.

```typescript
const client = new EdgeQuake({
  baseUrl: "http://localhost:8080",
  apiKey: "key",
  tenantId: "tenant-123", // X-Tenant-Id header
  workspaceId: "workspace-456", // X-Workspace-Id header
});
```

## Custom Middleware

Add custom auth logic via middleware:

```typescript
import { EdgeQuake } from "@edgequake/sdk";
import type { Middleware } from "@edgequake/sdk";

const customAuth: Middleware = async (req, next) => {
  req.headers = {
    ...req.headers,
    "X-Custom-Token": await getToken(),
  };
  return next(req);
};

// Pass middleware through _transport injection (advanced)
```

## Security Best Practices

1. **Never hardcode API keys** — use environment variables
2. **Rotate keys regularly** — use `client.apiKeys.revoke()` for old keys
3. **Use tenant isolation** — always set `tenantId` in multi-tenant setups
4. **JWT expiry** — implement token refresh before expiry
5. **Rate limiting** — SDK auto-retries on 429 with exponential backoff
