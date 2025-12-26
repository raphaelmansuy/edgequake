# CopilotKit for Next.js Integration - Skill Overview

A production-grade skill for building **agentic UIs** in React and Next.js using CopilotKit: where the model can *see relevant UI/app state* and can *trigger tools/actions* that mutate that state or call backend APIs.

## Overview

This skill enables you to seamlessly integrate CopilotKit into Next.js applications, transforming static UIs into intelligent, context-aware interfaces. It covers everything from minimal setup to advanced production patterns.

## Languages Supported

- **TypeScript/JavaScript**: Full support for Next.js App Router, React hooks, and custom adapters

## What This Skill Solves

| Good Fit | Bad Fit |
| --- | --- |
| Integrating AI agents that read/write app state in real-time for interactive UIs. | Standalone backend AI with no UI/state synchronization (use pure backend frameworks). |
| Enabling user-guided AI decisions (human-in-the-loop checkpoints). | Enterprise-scale orchestration across many systems/teams. |
| Self-hosted LLM endpoints for privacy, custom adapters for non-standard providers. | Non-React / non-Next.js frontends. |

## Project Structure

```
.github/skills/copilotkit-nextjs-integration/
├── README.md                          # This file
├── metadata.yml                       # Skill metadata
├── overview.md                        # Complete CopilotKit guide
├── typescript/
│   ├── skill.md                      # TypeScript-specific patterns
│   ├── instructions.md               # Step-by-step setup guide
│   └── patterns/
│       ├── basic-setup.md            # Minimal working example
│       ├── state-sharing.md          # useCopilotReadable patterns
│       ├── actions-tools.md          # useCopilotAction patterns
│       ├── custom-adapters.md        # Custom LLM adapter implementation
│       └── production-security.md    # Auth, validation, logging
└── examples/
    ├── hello-copilot/                # Basic chat popup
    ├── user-list-assistant/          # State + chat example
    ├── task-automator/               # Actions & workflow example
    └── self-hosted-llm/              # Custom adapter pattern
```

## Core Concepts

### Mental Model

CopilotKit is a bridge between your **Next.js frontend** and an **LLM runtime endpoint**. The runtime calls an LLM provider through an **adapter**, and the frontend shares context and exposes tools.

```
[Next.js Frontend] ↔ [Copilot Runtime] ↔ [LLM Adapter] ↔ [LLM Service]
```

### Key Primitives

- **Copilot Runtime**: Self-hosted endpoint handling message history, tool calls, and provider integration.
- **Readable State**: App data exposed to the model for context-aware answers (`useCopilotReadable`).
- **Actions/Tools**: Executable functions the model can trigger (`useCopilotAction`).
- **UI Components**: `CopilotPopup`, `CopilotSidebar`, or headless chat for fully custom UI.
- **Adapters**: Provider integration layer (OpenAI, Groq, Google, LangChain, or custom).
- **Protocols (AG-UI / generative UI)**: Structured communication for syncing outputs into interactive UI.

## Real-World Scenarios

- **SaaS Dashboard**: A copilot automates outreach and updates CRM records based on tables the user is viewing.
- **Productivity App**: An agent suggests and executes task updates inside a to-do list.
- **E-commerce Tool**: A copilot analyzes cart state and suggests actions like promotions, refunds, or support workflows.

## The Survival Kit (Fastest Path to Proficiency)

### Prioritized Learning Path

**Day 0**: Basic Setup
- Install packages
- Create runtime endpoint (`/api/copilotkit`)
- Wrap app in `<CopilotKit>`
- Add `CopilotPopup` and confirm chat works

**Week 1**: Real Context
- Add one `useCopilotReadable` state source
- Add one `useCopilotAction` tool
- Use built-in adapter and build first automation

**Week 2**: Production Hardening
- Implement custom adapter for proprietary LLM
- Add auth to runtime endpoint
- Harden streaming, timeouts, retries, logging

### 20% Features for 80% Results

- `CopilotKit` provider + `runtimeUrl`
- `useCopilotReadable` for state sharing
- `useCopilotAction` for tool execution
- `CopilotPopup` / `CopilotSidebar` for UI
- Custom `CopilotServiceAdapter` for LLM flexibility

## Common Pitfalls & Solutions

| Pitfall | Solution |
| --- | --- |
| Serverless timeouts break streaming | Configure max duration (60s+) and use streaming-compatible adapters |
| API keys leaked to client | Keep keys server-only in runtime endpoint |
| Adapter mismatches | Ensure custom adapter implements required methods, test with known payloads |
| Model incompatibilities | Confirm OpenAI-like chat schema support or use LangChain adapter |
| State bloat slows model | Keep readable data minimal and JSON-serializable |
| Tool calls not validated | Validate parameters server-side before execution |

## Quick Cheat Sheet

**Install:**
```bash
npm i @copilotkit/react-core @copilotkit/runtime @copilotkit/react-ui
```

**Runtime Endpoint** (`/api/copilotkit`):
```typescript
new CopilotRuntime(); 
runtime.handleRequest(req, new OpenAIAdapter());
```

**Provider Wrapper:**
```typescript
<CopilotKit runtimeUrl="/api/copilotkit">{children}</CopilotKit>
```

**Readable State:**
```typescript
useCopilotReadable({ description: "...", value: data })
```

**Action/Tool:**
```typescript
useCopilotAction({ name: "...", parameters: [...], handler: async (...) => {} })
```

**UI Components:**
```typescript
<CopilotPopup />, <CopilotSidebar />, or headless chat
```

## If You Only Remember 5 Things

1. Wrap the app in `CopilotKit` with `runtimeUrl`
2. Share state with `useCopilotReadable`
3. Define tools with `useCopilotAction`
4. Use adapters in the runtime endpoint
5. Implement streaming correctly in custom adapters

## Related Technologies & Concepts

- **Alternatives**: Vercel AI SDK (simpler chat, less agentic UI-state tooling)
- **Complements**: LangGraph, CrewAI, LangChain (backend reasoning), used behind CopilotKit via adapters
- **Prerequisites**: Next.js 13+ (App Router), React 18+, TypeScript, basic LLM API familiarity

## How to Use This Skill

Simply describe what you want to build:

**For Basic Integration:**
```
Set up CopilotKit runtime endpoint and add a basic chat popup to my Next.js app
```

**For State-Aware Copilot:**
```
Add a copilot to my dashboard that understands the current user list and can send notifications
```

**For Custom LLM:**
```
Implement a CopilotKit adapter for my self-hosted LLM service
```

**For Production Deployment:**
```
Add authentication, validation, and streaming hardening to my CopilotKit runtime endpoint
```

The AI assistant will automatically:
1. Generate appropriately typed components and hooks
2. Validate adapter implementations
3. Provide working examples
4. Verify security patterns
5. Suggest performance optimizations

## References

- [CopilotKit Official Docs](https://docs.copilotkit.ai/)
- [Bring Your Own LLM](https://docs.copilotkit.ai/direct-to-llm/guides/bring-your-own-llm)
- [OpenAI Adapter Reference](https://docs.copilotkit.ai/reference/classes/llm-adapters/OpenAIAdapter)
- [GitHub Repository](https://github.com/CopilotKit/CopilotKit)
- [Task Automation Tutorial](https://www.telerik.com/blogs/task-automation-nextjs-using-copilotkit)
