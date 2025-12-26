# Why CopilotKit for Next.js applications

CopilotKit is a framework for building **agentic UIs** in React and Next.js: the model can *see relevant UI/app state* and can *trigger tools/actions* that mutate that state or call backend APIs.

---

### Why it matters

- It gives you a *structured* way to embed an in-app copilot (chat, sidebar, popup, or headless UI) that is **context-aware** and **action-capable**.
- It replaces ad-hoc "raw OpenAI calls" with a consistent runtime + tool protocol, which makes it easier to implement **human-in-the-loop** and shared-state patterns.
- It complements backend agent frameworks (LangGraph, CrewAI, LangChain) by focusing on the missing piece: **frontend hooks** for readable state and actionable tools.
- It is especially valuable when you want **self-hosting**, **privacy control**, and **provider flexibility** (custom adapters).

---

### What problems it solves (and what it doesn't)

| Good fit | Bad fit |
| --- | --- |
| Integrating AI agents that read/write app state in real-time for interactive UIs (chat-based task automation, copilots in SaaS dashboards). | Standalone backend AI with no UI/state synchronization requirements (use a pure backend framework instead). |
| Enabling user-guided AI decisions (human-in-the-loop checkpoints for reliability). | Enterprise-scale orchestration across many systems/teams (CopilotKit is app-level, not a distributed workflow engine). |
| Self-hosted LLM endpoints for privacy, avoiding cloud costs/latency, with custom adapters for non-standard providers. | Non-React / non-Next.js frontends (it is optimized for the React ecosystem). |

### Real-world scenarios

- **SaaS dashboard**: a copilot automates outreach and updates CRM records based on tables the user is looking at.
- **Productivity app**: an agent suggests and executes task updates inside a to-do list.
- **E-commerce tool**: a copilot analyzes cart state and suggests actions like promotions, refunds, or support workflows.

---

### Mental model / key concepts (minimum to think correctly)

CopilotKit is a bridge between your **Next.js frontend** and an **LLM runtime endpoint**. The runtime calls an LLM provider through an **adapter**, and the frontend shares context and exposes tools.

### Core primitives

- **Copilot Runtime**: self-hosted endpoint that handles message history, tool/action calls, and provider integration.
- **Readable State**: app data exposed to the model for context-aware answers (via hooks like `useCopilotReadable`).
- **Actions/Tools**: executable functions the model can trigger (via `useCopilotAction`).
- **UI Components**: e.g. `CopilotPopup`, `CopilotSidebar`, or headless chat for fully custom UI.
- **Adapters**: provider integration layer (OpenAI, Groq, Google, LangChain, or custom for self-hosted/proprietary models).
- **Protocols (AG-UI / generative UI)**: structured communication for syncing outputs into interactive UI.

### Interaction flow

```
[Next.js Frontend] -- state/actions --> [Copilot Runtime] --> [LLM Adapter] --> [LLM Service]
[Next.js Frontend] <-- responses/actions -- [Copilot Runtime]
```

### Glossary

- **Agentic UI**: an interface where the model can read context and take actions.
- **Human-in-the-loop**: the user approves or guides decisions at checkpoints.
- **Generative UI**: the model produces UI-relevant outputs (structured, actionable, renderable).
- **Runtime Endpoint**: server route that processes copilot interactions.
- **Service Adapter**: translates CopilotKit requests into an LLM provider's API.

---

### The survival kit (fastest path to proficiency)

### Prioritized checklist

- **Day 0**
    - Install packages.
    - Create a runtime endpoint (`/api/copilotkit`).
    - Wrap your app in `<CopilotKit ...>`.
    - Add `CopilotPopup` and confirm a basic chat works.
- **Week 1**
    - Add one `useCopilotReadable` state source.
    - Add one `useCopilotAction` tool.
    - Use a built-in adapter (e.g. Groq) and build one "real" automation.
- **Week 2**
    - Implement a custom adapter for your own LLM service.
    - Add auth to the runtime endpoint.
    - Harden streaming, timeouts, retries, and logging.

### 20% of features for 80% results

- `CopilotKit` provider + `runtimeUrl`
- `useCopilotReadable` for state sharing
- `useCopilotAction` for tool execution
- `CopilotPopup` / `CopilotSidebar` for UI
- Custom `CopilotServiceAdapter` for LLM flexibility

### Common pitfalls (and how to avoid them)

- **Serverless timeouts break streaming**: configure max duration (e.g., 60s) and use streaming-compatible adapters.
- **API keys leaked to client**: keep keys server-only.
- **Adapter mismatches**: ensure custom adapter implements required methods and test with known-good payloads.
- **Model incompatibilities**: confirm OpenAI-like chat schema support, or route through a LangChain adapter.

### Debugging / observability tips

- Log runtime requests in the API route.
- Add logging inside adapter methods (especially custom adapters).
- Use browser dev tools to verify readable state updates.
- Unit-test custom adapters with mocked upstream LLM calls.

### Performance + security gotchas

- Keep readable data minimal and JSON-serializable.
- Validate tool/action parameters server-side.
- Protect runtime endpoint with auth middleware.
- Batch tool calls when possible to reduce LLM churn.

---

### Progressive complexity examples (minimal but real)

### Example 1: "Hello, core primitive"

Goal: add a basic chat popup that greets based on app context.

```tsx
// api/copilotkit/route.ts
import { CopilotRuntime, OpenAIAdapter } from "@copilotkit/runtime";

const runtime = new CopilotRuntime();

export const POST = (req: Request) =>
  runtime.handleRequest(req, new OpenAIAdapter());
```

```tsx
// app/page.tsx
"use client";

import { CopilotPopup } from "@copilotkit/react-ui";
import { useCopilotReadable } from "@copilotkit/react-core";

export default function Home() {
  useCopilotReadable({ description: "App name", value: "My Next.js App" });
  return <CopilotPopup instructions="Greet the user based on app context." />;
}
```

When to use: quick contextual chat.

Not for: complex workflows.

---

### Example 2: "Typical workflow" (switch adapter + share data)

Goal: let the model answer questions about a user list using a fast adapter.

```tsx
// api/copilotkit/route.ts
import { CopilotRuntime, GroqAdapter } from "@copilotkit/runtime";

const runtime = new CopilotRuntime();

export const POST = (req: Request) =>
  runtime.handleRequest(req, new GroqAdapter({ apiKey: process.env.GROQ_API_KEY! }));
```

```tsx
// app/page.tsx
"use client";

import { CopilotPopup } from "@copilotkit/react-ui";
import { useCopilotReadable } from "@copilotkit/react-core";

const users = [
  { name: "Alice", email: "alice@example.com" },
  { name: "Bob", email: "bob@example.com" }
];

export default function Page() {
  useCopilotReadable({ description: "User list", value: users });

  return <CopilotPopup instructions="Answer questions about the users list." />;
}
```

How to test quickly:

- Ask: "Which users do we have?"
- Ask: "What is Bob's email?"
- Confirm the responses stay grounded in the `users` readable state.

---

### Example 3: "Production-ish pattern" (actions/tools)

Goal: enable the model to trigger an action like sending an email.

```tsx
"use client";

import { useCopilotAction } from "@copilotkit/react-core";

useCopilotAction({
  name: "sendEmail",
  description: "Send an email",
  parameters: [
    { name: "to", type: "string", required: true },
    { name: "subject", type: "string", required: true },
    { name: "body", type: "string", required: true }
  ],
  handler: async ({ to, subject, body }) => {
    // call your backend email endpoint here
    return "Email sent";
  }
});
```

Pattern: user asks → model proposes action → (optionally) user approves → handler runs.

---

### Example 4: "Advanced but common" (custom adapter)

Goal: integrate a proprietary/self-hosted LLM service.

```tsx
// lib/custom-adapter.ts
import { CopilotServiceAdapter, ChatCompletionOptions } from "@copilotkit/runtime";

export class CustomLLMAdapter implements CopilotServiceAdapter {
  async process(request: any): Promise<Response> {
    // Proxy a chat request to your internal LLM API (SSE expected).
    const upstream = await fetch("https://custom-llm-service/api/chat", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request.body)
    });

    return new Response(upstream.body, {
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache"
      }
    });
  }

  async streamChatCompletion(options: ChatCompletionOptions): Promise<ReadableStream> {
    const upstream = await fetch("https://custom-llm-service/api/stream", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(options)
    });

    if (!upstream.body) throw new Error("Upstream did not return a stream");
    return upstream.body;
  }

  // Add other methods only if your CopilotKit runtime version requires them.
}
```

Security note: keep the custom LLM endpoint behind auth, and validate tool/action params server-side.

```tsx
// api/copilotkit/route.ts
import { CopilotRuntime } from "@copilotkit/runtime";
import { CustomLLMAdapter } from "@/lib/custom-adapter";

const runtime = new CopilotRuntime();

export const POST = (req: Request) =>
  runtime.handleRequest(req, new CustomLLMAdapter());
```

---

### Cheat sheet

- Install:
    - `npm i @copilotkit/react-core @copilotkit/runtime @copilotkit/react-ui`
- Runtime endpoint:
    - `new CopilotRuntime(); runtime.handleRequest(req, new OpenAIAdapter());`
- Provider:
    - `<CopilotKit runtimeUrl="/api/copilotkit">{children}</CopilotKit>`
- Readable state:
    - `useCopilotReadable({ description: "...", value: data })`
- Action/tool:
    - `useCopilotAction({ name: "...", parameters: [...], handler: async (...) => {} })`
- UI:
    - `CopilotPopup`, `CopilotSidebar`, or headless chat.
- Vercel timeouts:
    - set max duration (e.g., 60s) when streaming.

### If you only remember 5 things

- Wrap the app in `CopilotKit` with `runtimeUrl`.
- Share state with `useCopilotReadable`.
- Define tools with `useCopilotAction`.
- Use adapters in the runtime endpoint.
- Implement streaming correctly in custom adapters.

---

### Related technologies & concepts (map of the neighborhood)

- **Alternatives**: Vercel AI SDK (simpler chat, less agentic UI-state tooling).
- **Complements**: LangGraph/CrewAI/LangChain (backend reasoning and orchestration), used behind CopilotKit via adapters.
- **Prereqs**: Next.js App Router, React hooks, TypeScript, and basic LLM API familiarity.

---

### References

- Official docs: https://docs.copilotkit.ai/
- Bring your own LLM: https://docs.copilotkit.ai/direct-to-llm/guides/bring-your-own-llm
- OpenAI adapter reference: https://docs.copilotkit.ai/reference/classes/llm-adapters/OpenAIAdapter
- GitHub repo: https://github.com/CopilotKit/CopilotKit
- Tutorial (task automation): https://www.telerik.com/blogs/task-automation-nextjs-using-copilotkit
