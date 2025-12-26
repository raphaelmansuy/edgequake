# CopilotKit TypeScript Skill

Complete TypeScript/Next.js patterns and best practices for integrating CopilotKit.

## TypeScript-Specific Features

### Type Safety

CopilotKit provides full TypeScript support with strong typing for:

- **Readable State**: Typed via generics
  ```typescript
  interface UserListState {
    users: Array<{ id: string; name: string; email: string }>;
  }
  
  useCopilotReadable<UserListState>({
    description: "Current user list",
    value: users
  });
  ```

- **Action Parameters**: Typed handler functions
  ```typescript
  useCopilotAction({
    name: "updateUser",
    parameters: [
      { name: "userId", type: "string", required: true },
      { name: "newName", type: "string", required: true }
    ],
    handler: async (params: { userId: string; newName: string }) => {
      // TypeScript knows params shape
      return `Updated ${params.newName}`;
    }
  });
  ```

### React Hook Patterns

All CopilotKit functionality uses React hooks for composition:

```typescript
"use client"; // Next.js Server Component boundary

import { useCopilotReadable, useCopilotAction } from "@copilotkit/react-core";

export function MyComponent() {
  // Share state with model
  useCopilotReadable({ description: "...", value: data });
  
  // Define actions
  useCopilotAction({
    name: "doSomething",
    parameters: [...],
    handler: async (params) => { ... }
  });
  
  return <div>...</div>;
}
```

### Server-Side Runtime Pattern

The runtime endpoint is a Next.js API route with full TypeScript support:

```typescript
// app/api/copilotkit/route.ts
import { CopilotRuntime, OpenAIAdapter } from "@copilotkit/runtime";
import type { NextRequest } from "next/server";

const runtime = new CopilotRuntime();

export const POST = async (req: NextRequest) => {
  return runtime.handleRequest(
    req,
    new OpenAIAdapter({
      apiKey: process.env.OPENAI_API_KEY!
    })
  );
};

export const maxDuration = 60; // Vercel: 60s max
```

### Custom Adapter Types

```typescript
import type { CopilotServiceAdapter } from "@copilotkit/runtime";

class MyCustomAdapter implements CopilotServiceAdapter {
  async streamChatCompletion(
    options: ChatCompletionOptions
  ): Promise<ReadableStream> {
    // Implementation
  }
}
```

## Next.js App Router Integration

CopilotKit works seamlessly with Next.js App Router:

- **Layout Wrapper**: Wrap provider in root layout
  ```typescript
  // app/layout.tsx
  import { CopilotKit } from "@copilotkit/react-core";
  
  export default function RootLayout({
    children
  }: {
    children: React.ReactNode;
  }) {
    return (
      <html>
        <body>
          <CopilotKit runtimeUrl="/api/copilotkit">
            {children}
          </CopilotKit>
        </body>
      </html>
    );
  }
  ```

- **Page Components**: Use hooks naturally
  ```typescript
  // app/dashboard/page.tsx
  "use client";
  
  import { CopilotPopup } from "@copilotkit/react-ui";
  import { useCopilotReadable } from "@copilotkit/react-core";
  
  export default function DashboardPage() {
    // Share component state
    useCopilotReadable({ ... });
    
    return (
      <>
        <DashboardContent />
        <CopilotPopup />
      </>
    );
  }
  ```

## Environment Configuration

TypeScript enables better environment variable handling:

```typescript
// lib/env.ts
const getEnvVar = (name: string): string => {
  const value = process.env[name];
  if (!value) throw new Error(`Missing environment variable: ${name}`);
  return value;
};

export const LLM_API_KEY = getEnvVar("OPENAI_API_KEY");
export const COPILOT_RUNTIME_URL = process.env.COPILOT_RUNTIME_URL ?? "/api/copilotkit";
```

## Common Type Patterns

### Readable State with Complex Objects

```typescript
interface DashboardState {
  users: Array<{ id: string; name: string }>;
  metrics: { activeCount: number; totalCount: number };
  isLoading: boolean;
}

const [state, setState] = useState<DashboardState>({...});

useCopilotReadable({
  description: "Dashboard state with users and metrics",
  value: {
    users: state.users,
    metrics: state.metrics,
    loadingStatus: state.isLoading ? "loading" : "ready"
  }
});
```

### Actions with Validation

```typescript
useCopilotAction({
  name: "sendNotification",
  parameters: [
    {
      name: "userId",
      type: "string",
      required: true,
      description: "User ID to notify"
    },
    {
      name: "message",
      type: "string",
      required: true,
      description: "Message to send"
    }
  ],
  handler: async (params) => {
    if (!params.userId || !params.message) {
      throw new Error("userId and message are required");
    }
    
    const response = await fetch("/api/notifications", {
      method: "POST",
      body: JSON.stringify(params)
    });
    
    if (!response.ok) {
      throw new Error(`Failed: ${response.statusText}`);
    }
    
    return "Notification sent";
  }
});
```

## Linting & Type Checking

```bash
# Type check
npx tsc --noEmit

# ESLint with React/React Hooks rules
npx eslint app/

# Next.js build with full type checking
npm run build
```

## Testing Patterns

Use TypeScript with your test framework:

```typescript
// __tests__/copilot-action.test.ts
import { renderHook } from "@testing-library/react";
import { useCopilotAction } from "@copilotkit/react-core";

describe("sendEmail action", () => {
  it("should validate required parameters", async () => {
    const { result } = renderHook(() =>
      useCopilotAction({
        name: "sendEmail",
        parameters: [
          { name: "to", type: "string", required: true }
        ],
        handler: async (params) => {
          expect(params.to).toBeDefined();
          return "sent";
        }
      })
    );

    // Test implementation
  });
});
```

## Production Patterns

### Typed Middleware

```typescript
// lib/copilot-middleware.ts
import type { NextRequest, NextResponse } from "next/server";

export async function validateCopilotRequest(
  req: NextRequest
): Promise<{ valid: boolean; error?: string }> {
  // Validation logic
  return { valid: true };
}
```

### Logging with Types

```typescript
// lib/logger.ts
interface CopilotLogEntry {
  timestamp: Date;
  action: string;
  userId?: string;
  status: "success" | "error";
  details?: Record<string, unknown>;
}

export function logCopilotAction(entry: CopilotLogEntry) {
  console.log(JSON.stringify(entry));
}
```

## References

- [CopilotKit TypeScript Types](https://github.com/CopilotKit/CopilotKit/tree/main/packages/runtime)
- [Next.js TypeScript Documentation](https://nextjs.org/docs/app/building-your-application/configuring/typescript)
- [React Hooks TypeScript Guide](https://react-typescript-cheatsheet.netlify.app/docs/basic/hooks)
