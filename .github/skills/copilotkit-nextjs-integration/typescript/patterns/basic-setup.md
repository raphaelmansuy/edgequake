# Basic Setup Pattern

Minimal working example: runtime endpoint + app wrapper + basic chat popup.

## Files Structure

```
app/
├── api/
│   └── copilotkit/
│       └── route.ts          # Runtime endpoint
├── layout.tsx                # CopilotKit provider
└── page.tsx                  # Chat popup
```

## Implementation

### 1. Runtime Endpoint (`app/api/copilotkit/route.ts`)

```typescript
import { CopilotRuntime, OpenAIAdapter } from "@copilotkit/runtime";

const runtime = new CopilotRuntime();

export const POST = async (req: Request) => {
  return runtime.handleRequest(
    req,
    new OpenAIAdapter({ apiKey: process.env.OPENAI_API_KEY! })
  );
};

export const maxDuration = 60; // Vercel: streaming timeout
```

### 2. Root Layout (`app/layout.tsx`)

```typescript
import type { Metadata } from "next";
import { CopilotKit } from "@copilotkit/react-core";
import "@copilotkit/react-ui/styles.css";

export const metadata: Metadata = {
  title: "CopilotKit App",
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

### 3. Home Page (`app/page.tsx`)

```typescript
"use client";

import { CopilotPopup } from "@copilotkit/react-ui";

export default function Home() {
  return (
    <main>
      <h1>My Copilot App</h1>
      <p>Click the copilot icon in the bottom-right corner.</p>
      <CopilotPopup instructions="Be helpful and friendly." />
    </main>
  );
}
```

### 4. Environment Variables (`.env.local`)

```bash
OPENAI_API_KEY=sk-...
```

## Testing

```bash
npm install
npm run dev
# Visit http://localhost:3000
# Click the copilot icon and chat
```

## What's Happening

1. **Runtime Endpoint** (`/api/copilotkit`): Receives chat messages from frontend, calls OpenAI, streams responses back.
2. **CopilotKit Provider**: Initializes frontend context with `runtimeUrl`.
3. **CopilotPopup**: Renders a floating chat icon that opens a chat window.
4. **User Interaction**: Type message → sent to runtime → OpenAI → streamed back → displayed in chat.

## Common Variations

### Use Groq Instead (Faster & Cheaper)

```typescript
import { CopilotRuntime, GroqAdapter } from "@copilotkit/runtime";

export const POST = async (req: Request) => {
  return runtime.handleRequest(
    req,
    new GroqAdapter({ apiKey: process.env.GROQ_API_KEY! })
  );
};
```

### Use CopilotSidebar Instead

```typescript
"use client";

import { CopilotSidebar } from "@copilotkit/react-ui";

export default function Home() {
  return (
    <div className="flex">
      <CopilotSidebar />
      <main className="flex-1">
        {/* Your content */}
      </main>
    </div>
  );
}
```

### Custom Instructions

```typescript
<CopilotPopup
  instructions="You are a helpful assistant for a task management app. Be concise and friendly."
  defaultOpen={false}
/>
```

## Debugging

**Problem**: Popup not appearing
- Check browser console for errors
- Verify CSS is imported: `@copilotkit/react-ui/styles.css`

**Problem**: "Copilot not connecting"
- Check Network tab → `/api/copilotkit` is called
- Verify `.env.local` has API key
- Check server logs for errors

**Problem**: No response from LLM
- Verify API key is valid
- Check account has credit/quota
- Use simpler prompt to test

## Next Steps

- Add `useCopilotReadable` for context (see [state-sharing.md](state-sharing.md))
- Add `useCopilotAction` for actions (see [actions-tools.md](actions-tools.md))
- Customize UI (see CopilotKit docs)
