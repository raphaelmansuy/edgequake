# Production Security & Hardening

Best practices for deploying CopilotKit in production.

## 1. API Key Security

### ✅ Keep Keys Server-Side Only

```typescript
// app/api/copilotkit/route.ts
import { CopilotRuntime, OpenAIAdapter } from "@copilotkit/runtime";

export const POST = async (req: Request) => {
  // API key is server-side only
  const apiKey = process.env.OPENAI_API_KEY;
  
  if (!apiKey) {
    return new Response("Missing API key", { status: 500 });
  }

  const adapter = new OpenAIAdapter({ apiKey });
  return new CopilotRuntime().handleRequest(req, adapter);
};
```

### ❌ Never Expose Keys to Client

```typescript
// ❌ BAD: Keys visible in browser
const CopilotKit = ({ children }) => (
  <CopilotKitProvider
    apiKey={process.env.NEXT_PUBLIC_OPENAI_API_KEY} // ❌ WRONG
  >
    {children}
  </CopilotKitProvider>
);
```

### Environment Variables

```bash
# .env.local (never commit this)
OPENAI_API_KEY=sk-...
GROQ_API_KEY=gsk_...
COPILOT_AUTH_TOKEN=secret-token-12345
```

```typescript
// lib/env.ts
export const getApiKey = (): string => {
  const key = process.env.OPENAI_API_KEY;
  if (!key) {
    throw new Error("OPENAI_API_KEY is required");
  }
  return key;
};
```

## 2. Endpoint Authentication

### JWT Authentication

```typescript
// lib/auth.ts
import jwt from "jsonwebtoken";

export function verifyAuthToken(token: string): { userId: string } | null {
  try {
    const decoded = jwt.verify(token, process.env.AUTH_SECRET!);
    return decoded as { userId: string };
  } catch {
    return null;
  }
}
```

```typescript
// app/api/copilotkit/route.ts
import { verifyAuthToken } from "@/lib/auth";

export const POST = async (req: Request) => {
  // Check authorization header
  const authHeader = req.headers.get("authorization");
  const token = authHeader?.replace("Bearer ", "");

  if (!token) {
    return new Response("Missing authorization", { status: 401 });
  }

  const auth = verifyAuthToken(token);
  if (!auth) {
    return new Response("Invalid token", { status: 401 });
  }

  // Proceed with authenticated request
  const runtime = new CopilotRuntime();
  return runtime.handleRequest(req, new OpenAIAdapter());
};
```

### Custom Token Header

```typescript
export const POST = async (req: Request) => {
  const apiKey = req.headers.get("x-copilot-api-key");

  if (apiKey !== process.env.COPILOT_API_KEY) {
    return new Response("Invalid API key", { status: 401 });
  }

  // Proceed
};
```

## 3. Input Validation

### Validate Tool Parameters

```typescript
useCopilotAction({
  name: "updateUser",
  parameters: [
    { name: "userId", type: "string", required: true },
    { name: "email", type: "string", required: false }
  ],
  handler: async (params: { userId: string; email?: string }) => {
    // Validate userId
    if (!params.userId || typeof params.userId !== "string") {
      throw new Error("Invalid userId");
    }

    if (params.userId.length < 3 || params.userId.length > 50) {
      throw new Error("userId must be 3-50 characters");
    }

    // Validate email format
    if (params.email && !isValidEmail(params.email)) {
      throw new Error("Invalid email format");
    }

    // Execute only after validation
    return updateUser(params.userId, params.email);
  }
});

function isValidEmail(email: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
}
```

### Sanitize Messages (XSS Prevention)

```typescript
// lib/sanitize.ts
export function sanitizeMessage(msg: string): string {
  // Remove potentially dangerous HTML
  return msg
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#x27;")
    .replace(/\//g, "&#x2F;");
}
```

## 4. Rate Limiting

### Simple Rate Limiting

```typescript
// lib/rate-limit.ts
const requestCounts = new Map<string, { count: number; resetAt: number }>();

export function isRateLimited(clientId: string, limit: number = 10, windowMs: number = 60000): boolean {
  const now = Date.now();
  const record = requestCounts.get(clientId);

  if (!record || now > record.resetAt) {
    requestCounts.set(clientId, { count: 1, resetAt: now + windowMs });
    return false;
  }

  record.count++;
  if (record.count > limit) {
    return true;
  }

  return false;
}
```

```typescript
// app/api/copilotkit/route.ts
import { isRateLimited } from "@/lib/rate-limit";

export const POST = async (req: Request) => {
  const clientId = req.headers.get("x-forwarded-for") || "unknown";

  if (isRateLimited(clientId, 100, 60000)) { // 100 requests per minute
    return new Response("Too many requests", { status: 429 });
  }

  // Proceed
};
```

### Using Upstash Redis

```typescript
// lib/upstash-rate-limit.ts
import { Ratelimit } from "@upstash/ratelimit";
import { Redis } from "@upstash/redis";

export const ratelimit = new Ratelimit({
  redis: Redis.fromEnv(),
  limiter: Ratelimit.slidingWindow(100, "60 s") // 100 requests per 60 seconds
});
```

```typescript
// app/api/copilotkit/route.ts
import { ratelimit } from "@/lib/upstash-rate-limit";

export const POST = async (req: Request) => {
  const clientId = req.headers.get("x-forwarded-for") || "unknown";
  const { success } = await ratelimit.limit(clientId);

  if (!success) {
    return new Response("Rate limited", { status: 429 });
  }

  // Proceed
};
```

## 5. Logging & Monitoring

### Structured Logging

```typescript
// lib/logger.ts
interface LogEntry {
  timestamp: string;
  level: "info" | "warn" | "error";
  action: string;
  userId?: string;
  status?: number;
  duration?: number;
  error?: string;
  details?: Record<string, unknown>;
}

export function logCopilotAction(entry: LogEntry) {
  const logEntry = {
    ...entry,
    timestamp: new Date().toISOString()
  };

  console.log(JSON.stringify(logEntry));
  
  // Send to external logger if needed
  // await sendToLogging(logEntry);
}
```

```typescript
// app/api/copilotkit/route.ts
import { logCopilotAction } from "@/lib/logger";

export const POST = async (req: Request) => {
  const startTime = Date.now();

  try {
    const runtime = new CopilotRuntime();
    const response = await runtime.handleRequest(req, new OpenAIAdapter());

    logCopilotAction({
      action: "chat_completion",
      status: 200,
      duration: Date.now() - startTime
    });

    return response;
  } catch (error) {
    logCopilotAction({
      action: "chat_completion",
      level: "error",
      error: error instanceof Error ? error.message : String(error),
      duration: Date.now() - startTime
    });

    return new Response("Internal server error", { status: 500 });
  }
};
```

## 6. Streaming Timeout Handling

### Vercel Deployment

```typescript
// next.config.ts
/** @type {import('next').NextConfig} */
const nextConfig = {
  maxDuration: 60 // 60 seconds max
};

module.exports = nextConfig;
```

```typescript
// app/api/copilotkit/route.ts
export const maxDuration = 60;

export const POST = async (req: Request) => {
  // Must complete within 60 seconds
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 55000); // 55s timeout

  try {
    // Streaming implementation
    const runtime = new CopilotRuntime();
    return runtime.handleRequest(req, new OpenAIAdapter());
  } finally {
    clearTimeout(timeout);
  }
};
```

## 7. CORS Configuration

### Allow Trusted Origins Only

```typescript
// lib/cors.ts
const ALLOWED_ORIGINS = [
  "https://myapp.com",
  "https://app.myapp.com",
  process.env.NEXT_PUBLIC_APP_URL
].filter(Boolean);

export function isCorsAllowed(origin: string | null): boolean {
  return origin ? ALLOWED_ORIGINS.includes(origin) : false;
}
```

```typescript
// app/api/copilotkit/route.ts
import { isCorsAllowed } from "@/lib/cors";

export const POST = async (req: Request) => {
  const origin = req.headers.get("origin");

  if (!isCorsAllowed(origin)) {
    return new Response("CORS not allowed", { status: 403 });
  }

  // Add CORS headers
  const response = await runtime.handleRequest(req, new OpenAIAdapter());
  response.headers.set("Access-Control-Allow-Origin", origin);
  
  return response;
};
```

## 8. Error Handling

### Don't Expose Internal Errors

```typescript
// ❌ BAD: Leaks internal error details
catch (error) {
  return new Response(error.message, { status: 500 });
}
```

```typescript
// ✅ GOOD: Generic error to client, detailed log for debugging
catch (error) {
  logCopilotAction({
    level: "error",
    action: "chat_completion",
    error: error instanceof Error ? error.message : String(error),
    details: { stack: error instanceof Error ? error.stack : undefined }
  });

  return new Response("Service unavailable", { status: 500 });
}
```

## 9. Complete Hardened Endpoint

```typescript
// app/api/copilotkit/route.ts
import { CopilotRuntime, OpenAIAdapter } from "@copilotkit/runtime";
import type { NextRequest } from "next/server";
import { verifyAuthToken } from "@/lib/auth";
import { isRateLimited } from "@/lib/rate-limit";
import { logCopilotAction } from "@/lib/logger";

export const maxDuration = 60;

export const POST = async (req: NextRequest) => {
  const startTime = Date.now();
  const clientId = req.headers.get("x-forwarded-for") || "unknown";

  try {
    // 1. Check rate limit
    if (isRateLimited(clientId)) {
      return new Response("Rate limited", { status: 429 });
    }

    // 2. Verify authentication
    const authHeader = req.headers.get("authorization");
    const token = authHeader?.replace("Bearer ", "");

    if (!token) {
      logCopilotAction({
        action: "auth_failure",
        level: "warn",
        error: "Missing token"
      });
      return new Response("Unauthorized", { status: 401 });
    }

    const auth = verifyAuthToken(token);
    if (!auth) {
      logCopilotAction({
        action: "auth_failure",
        level: "warn",
        error: "Invalid token"
      });
      return new Response("Unauthorized", { status: 401 });
    }

    // 3. Process copilot request
    const runtime = new CopilotRuntime();
    const response = await runtime.handleRequest(
      req,
      new OpenAIAdapter({ apiKey: process.env.OPENAI_API_KEY! })
    );

    // 4. Log success
    logCopilotAction({
      action: "chat_completion",
      userId: auth.userId,
      status: 200,
      duration: Date.now() - startTime
    });

    return response;
  } catch (error) {
    // 5. Log error
    logCopilotAction({
      action: "chat_completion",
      level: "error",
      error: error instanceof Error ? error.message : String(error),
      duration: Date.now() - startTime
    });

    // 6. Return generic error
    return new Response("Service error", { status: 500 });
  }
};
```

## 10. Security Checklist

- [ ] API keys stored in environment variables only
- [ ] Runtime endpoint requires authentication
- [ ] Rate limiting implemented
- [ ] Input validation on all tool parameters
- [ ] Error messages don't leak internal details
- [ ] Request/response logging configured
- [ ] CORS headers properly configured
- [ ] Streaming timeout set appropriately
- [ ] XSS prevention (sanitize user input)
- [ ] CSRF protection if needed
- [ ] Secrets never committed to git
- [ ] Regular security updates for dependencies

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Node.js Security Best Practices](https://nodejs.org/en/docs/guides/security/)
- [Next.js Security Best Practices](https://nextjs.org/docs/basic-features/environment-variables#bundling-environment-variables-for-the-browser)
