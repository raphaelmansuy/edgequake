# CopilotKit Next.js Integration - Step-by-Step Setup

Complete instructions for integrating CopilotKit into a Next.js application.

## Prerequisites

- Node.js 18+
- Next.js 13+ (App Router)
- npm or yarn
- An LLM API key (OpenAI, Groq, Google) or self-hosted endpoint

## Step 1: Create or Update Your Next.js Project

```bash
# If starting fresh
npx create-next-app@latest my-copilot-app --typescript --app
cd my-copilot-app

# If using existing project
cd my-existing-nextjs-app
```

Ensure `next.config.ts` supports streaming:
```typescript
/** @type {import('next').NextConfig} */
const nextConfig = {
  maxDuration: 60 // Vercel: set timeout for streaming API routes
};

module.exports = nextConfig;
```

## Step 2: Install Dependencies

```bash
npm install @copilotkit/react-core @copilotkit/runtime @copilotkit/react-ui
```

Optional: for specific adapters
```bash
npm install @copilotkit/adapters-groq    # for Groq
npm install @copilotkit/adapters-google  # for Google Gemini
```

## Step 3: Set Up Environment Variables

Create `.env.local`:
```bash
# Choose one LLM provider
OPENAI_API_KEY=sk-...
# OR
GROQ_API_KEY=gsk_...
# OR
GOOGLE_API_KEY=...

# Optional: runtime URL (defaults to /api/copilotkit)
COPILOT_RUNTIME_URL=/api/copilotkit
```

## Step 4: Create the Runtime Endpoint

Create `app/api/copilotkit/route.ts`:

```typescript
import { CopilotRuntime, OpenAIAdapter } from "@copilotkit/runtime";

const runtime = new CopilotRuntime();

export const POST = async (req: Request) => {
  return runtime.handleRequest(
    req,
    new OpenAIAdapter({
      apiKey: process.env.OPENAI_API_KEY!
    })
  );
};

export const maxDuration = 60; // Vercel: max function duration
```

**For Groq** (faster, cheaper):
```typescript
import { CopilotRuntime, GroqAdapter } from "@copilotkit/runtime";

export const POST = async (req: Request) => {
  return runtime.handleRequest(
    req,
    new GroqAdapter({
      apiKey: process.env.GROQ_API_KEY!
    })
  );
};
```

**For Google Gemini**:
```typescript
import { CopilotRuntime, GoogleAdapter } from "@copilotkit/runtime";

export const POST = async (req: Request) => {
  return runtime.handleRequest(
    req,
    new GoogleAdapter({
      apiKey: process.env.GOOGLE_API_KEY!
    })
  );
};
```

## Step 5: Wrap Your App in CopilotKit Provider

Update `app/layout.tsx`:

```typescript
import type { Metadata } from "next";
import { CopilotKit } from "@copilotkit/react-core";
import "@copilotkit/react-ui/styles.css"; // Required CSS

export const metadata: Metadata = {
  title: "My Copilot App",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>
        <CopilotKit runtimeUrl="/api/copilotkit">
          {children}
        </CopilotKit>
      </body>
    </html>
  );
}
```

## Step 6: Add CopilotPopup to a Page

Update `app/page.tsx`:

```typescript
"use client";

import { CopilotPopup } from "@copilotkit/react-ui";
import { useCopilotReadable } from "@copilotkit/react-core";

export default function Home() {
  // Share app context with the model
  useCopilotReadable({
    description: "The current page",
    value: "Home page - User is exploring the app"
  });

  return (
    <main>
      <h1>Welcome to My Copilot App</h1>
      <p>The copilot is ready to help. Click the widget in the bottom-right corner.</p>
      
      {/* CopilotPopup renders a chat icon */}
      <CopilotPopup instructions="Be helpful and friendly." />
    </main>
  );
}
```

Test it:
```bash
npm run dev
# Visit http://localhost:3000
# Click the copilot icon in the bottom-right
```

## Step 7: Add Readable State

Share dynamic app state with the model:

```typescript
"use client";

import { useState } from "react";
import { CopilotPopup } from "@copilotkit/react-ui";
import { useCopilotReadable } from "@copilotkit/react-core";

const users = [
  { id: "1", name: "Alice", email: "alice@example.com" },
  { id: "2", name: "Bob", email: "bob@example.com" },
];

export default function DashboardPage() {
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null);

  // Share the user list and current selection
  useCopilotReadable({
    description: "Available users and current selection",
    value: {
      users,
      selectedUserId,
      userCount: users.length
    }
  });

  return (
    <div>
      <h2>Dashboard</h2>
      <ul>
        {users.map(user => (
          <li key={user.id}>
            {user.name} ({user.email})
          </li>
        ))}
      </ul>
      <CopilotPopup instructions="Answer questions about the user list." />
    </div>
  );
}
```

Test by asking: "How many users are there?" or "What is Alice's email?"

## Step 8: Add Actions (Tools)

Let the model trigger app state changes:

```typescript
"use client";

import { useState } from "react";
import { CopilotPopup } from "@copilotkit/react-ui";
import { useCopilotReadable, useCopilotAction } from "@copilotkit/react-core";

export default function TaskPage() {
  const [tasks, setTasks] = useState<Array<{ id: string; title: string; done: boolean }>>([
    { id: "1", title: "Read documentation", done: false }
  ]);

  useCopilotReadable({
    description: "Current task list",
    value: { tasks, pendingCount: tasks.filter(t => !t.done).length }
  });

  // Action: Mark a task as done
  useCopilotAction({
    name: "markTaskDone",
    description: "Mark a task as completed",
    parameters: [
      { name: "taskId", type: "string", required: true, description: "ID of the task" }
    ],
    handler: async (params: { taskId: string }) => {
      setTasks(tasks =>
        tasks.map(t =>
          t.id === params.taskId ? { ...t, done: true } : t
        )
      );
      return `Task marked as done`;
    }
  });

  // Action: Add a new task
  useCopilotAction({
    name: "addTask",
    description: "Add a new task",
    parameters: [
      { name: "title", type: "string", required: true, description: "Task title" }
    ],
    handler: async (params: { title: string }) => {
      const newTask = { id: Date.now().toString(), title: params.title, done: false };
      setTasks(tasks => [...tasks, newTask]);
      return `Task added: ${params.title}`;
    }
  });

  return (
    <div>
      <h2>Tasks</h2>
      {tasks.map(task => (
        <div key={task.id}>
          <input type="checkbox" checked={task.done} disabled />
          {task.title}
        </div>
      ))}
      <CopilotPopup instructions="Help manage the task list. You can mark tasks done or add new ones." />
    </div>
  );
}
```

Test by asking: "Mark the first task as done" or "Add a task to review code"

## Step 9: Add Backend API Integration

Call your backend from actions:

```typescript
"use client";

import { useCopilotAction } from "@copilotkit/react-core";

useCopilotAction({
  name: "sendEmail",
  description: "Send an email to a user",
  parameters: [
    { name: "to", type: "string", required: true },
    { name: "subject", type: "string", required: true },
    { name: "body", type: "string", required: true }
  ],
  handler: async (params) => {
    const response = await fetch("/api/send-email", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(params)
    });

    if (!response.ok) {
      throw new Error(`Email failed: ${response.statusText}`);
    }

    return "Email sent successfully";
  }
});
```

## Step 10: Production Hardening

### Add Authentication

```typescript
// app/api/copilotkit/route.ts
import { CopilotRuntime, OpenAIAdapter } from "@copilotkit/runtime";
import type { NextRequest } from "next/server";

const runtime = new CopilotRuntime();

export const POST = async (req: NextRequest) => {
  // Validate auth
  const token = req.headers.get("authorization")?.replace("Bearer ", "");
  if (!token || !validateToken(token)) {
    return new Response("Unauthorized", { status: 401 });
  }

  return runtime.handleRequest(req, new OpenAIAdapter());
};

function validateToken(token: string): boolean {
  // Your auth logic
  return token === process.env.COPILOT_AUTH_TOKEN;
}
```

### Add Logging

```typescript
export const POST = async (req: NextRequest) => {
  console.log(`[Copilot] Request from ${req.headers.get("user-agent")}`);
  
  try {
    return runtime.handleRequest(req, new OpenAIAdapter());
  } catch (error) {
    console.error("[Copilot] Error:", error);
    return new Response("Internal server error", { status: 500 });
  }
};
```

### Validate Tool Parameters

```typescript
useCopilotAction({
  name: "deleteUser",
  parameters: [{ name: "userId", type: "string", required: true }],
  handler: async (params) => {
    // Validate
    if (!params.userId || typeof params.userId !== "string") {
      throw new Error("Invalid userId");
    }
    if (params.userId.length < 3) {
      throw new Error("userId too short");
    }
    
    // Execute
    await deleteUserFromDatabase(params.userId);
    return "User deleted";
  }
});
```

## Troubleshooting

| Issue | Solution |
| --- | --- |
| "Copilot not connecting" | Check runtime URL in browser dev tools Network tab. Ensure `/api/copilotkit` is accessible. |
| "Streaming times out" | Increase `maxDuration` in `next.config.ts`. Use faster adapter (Groq). |
| "TypeError: fetch is not defined" | Ensure action handlers use `async` and await properly. |
| "API key not found" | Check `.env.local` matches code (`OPENAI_API_KEY`, etc.). Restart `npm run dev`. |
| "State not updating in UI" | Ensure you use `setState` in action handlers. React state updates trigger re-renders. |

## Next Steps

1. **Customize UI**: Replace `CopilotPopup` with `CopilotSidebar` or build headless chat.
2. **Custom Adapter**: Implement `CopilotServiceAdapter` for your own LLM.
3. **Advanced Patterns**: See `patterns/` directory for more examples.
4. **Deploy**: Test on Vercel with correct environment variables.

## References

- [CopilotKit Docs](https://docs.copilotkit.ai/)
- [Examples](../examples/)
- [Patterns](patterns/)
