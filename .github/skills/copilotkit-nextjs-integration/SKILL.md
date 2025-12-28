---
name: copilotkit-nextjs-integration
description: Integrate CopilotKit AI components into Next.js frontend for building agentic UIs. Enables context-aware AI agents that can read app state and trigger tools/actions. Supports custom adapters for self-hosted LLMs and multiple provider integrations.
license: Proprietary (repository internal)
compatibility: Requires Next.js (13+), React (18+), TypeScript (5.0+)
metadata:
  repo: raphaelmansuy/edgequake
  area: frontend-ai-integration
  languages:
    - TypeScript
    - JavaScript
  frameworks:
    - Next.js
    - React
  patterns:
    - Agentic UI
    - Custom LLM adapters
    - Context-aware AI
---

# CopilotKit for Next.js Integration

Integrate CopilotKit AI components into your Next.js application to build intelligent, context-aware interfaces where AI agents can read application state and trigger actionable tools.

## When to use

Use this skill when you need to:

- Build AI-powered chat interfaces in your Next.js app
- Create context-aware AI agents that understand current UI state
- Enable AI to trigger actions and mutate app state
- Integrate custom or self-hosted LLM endpoints
- Implement human-in-the-loop AI workflows
- Add AI copilots to SaaS dashboards or productivity tools
- Support multiple LLM providers with custom adapters
- Build agentic UIs with sidebar, popup, or headless chat

## Core concepts

### Mental Model

CopilotKit is a **bridge** between your Next.js frontend and an LLM runtime endpoint:

```
[Next.js Frontend] 
    ↕ state/actions/messages
[Copilot Runtime Endpoint]
    ↕ provider protocol
[LLM Adapter] → [OpenAI/Claude/Groq/Custom LLM]
```

The runtime:
- Manages conversation history
- Calls your LLM provider through an adapter
- Executes tools/actions triggered by the model
- Syncs generated content back to your UI

### Key Primitives

- **Copilot Runtime**: Self-hosted endpoint managing message history, tool calls, and provider integration
- **Readable State**: App data exposed to the model via `useCopilotReadable` hooks for context
- **Actions/Tools**: Executable functions the model can trigger via `useCopilotAction`
- **UI Components**: `CopilotPopup`, `CopilotSidebar`, or custom headless chat
- **Adapters**: Provider integration layer (OpenAI, Groq, Google, custom self-hosted, etc.)
- **Generative UI**: Structured responses for interactive UI updates

## Quick start

### Basic Setup

```bash
npm install @copilotkit/react @copilotkit/sdk
```

### Minimal Example

```typescript
import { CopilotPopup } from "@copilotkit/react-ui";
import { CopilotKit } from "@copilotkit/react-core";

export default function App() {
  return (
    <CopilotKit runtimeUrl="http://localhost:8080/copilot">
      <YourApp />
      <CopilotPopup />
    </CopilotKit>
  );
}
```

### Share State with AI

```typescript
import { useCopilotReadable } from "@copilotkit/react-core";

export function TaskList({ tasks }) {
  useCopilotReadable({
    description: "List of tasks for the user",
    value: tasks
  });

  return (
    <ul>
      {tasks.map(task => <li key={task.id}>{task.title}</li>)}
    </ul>
  );
}
```

### Enable AI-Triggered Actions

```typescript
import { useCopilotAction } from "@copilotkit/react-core";

export function TaskPanel({ tasks, setTasks }) {
  useCopilotAction({
    name: "complete_task",
    description: "Mark a task as complete",
    parameters: [
      {
        name: "task_id",
        description: "The ID of the task to complete",
        type: "string"
      }
    ],
    handler: (taskId) => {
      setTasks(tasks.map(t => 
        t.id === taskId ? { ...t, completed: true } : t
      ));
    }
  });

  return (
    <div>
      {tasks.map(task => (
        <TaskCard key={task.id} task={task} />
      ))}
    </div>
  );
}
```

## Capabilities

### Context-Aware AI

- **Read App State**: Expose UI state, data, and context to the AI model
- **Conversation History**: Maintain conversation context across messages
- **Dynamic Context**: Update what the model sees as app state changes
- **Structured Responses**: Return structured data for interactive UI updates

### Tool & Action System

- **Define Custom Tools**: Any JavaScript function can become an AI-callable tool
- **Parameter Validation**: Define required and optional parameters with types
- **Error Handling**: Handle tool failures gracefully with retry logic
- **State Mutations**: Actions can update React state, trigger API calls, or run complex workflows
- **Human-in-the-Loop**: Get user confirmation before executing critical actions

### Provider Flexibility

- **OpenAI Integration**: Out-of-the-box support for GPT-4, GPT-4o, etc.
- **Alternative Providers**: Support for Groq, Google, Anthropic, etc.
- **Custom Adapters**: Implement adapters for proprietary or self-hosted models
- **Provider Agnostic**: Switch providers without changing your UI code
- **Self-Hosting**: Deploy your own LLM runtime for privacy and cost control

### UI Options

- **Popup UI**: Floating chat popup component
- **Sidebar UI**: Embedded sidebar copilot
- **Headless**: Build completely custom UI while using CopilotKit runtime
- **Customizable Styling**: Theme and style components to match your app
- **Accessibility**: Built-in keyboard navigation and screen reader support

## Workflow

When implementing CopilotKit in your Next.js app:

1. **Setup Runtime**: Deploy or configure Copilot Runtime endpoint
2. **Wrap App**: Wrap your Next.js app with `<CopilotKit>` provider
3. **Share State**: Use `useCopilotReadable` to expose relevant app data
4. **Define Actions**: Use `useCopilotAction` to enable AI-triggered operations
5. **Add UI**: Include `<CopilotPopup>`, `<CopilotSidebar>`, or custom chat UI
6. **Test & Refine**: Iterate on state exposure and action definitions

## Configuration Options

### Runtime Configuration

```typescript
<CopilotKit
  runtimeUrl="http://localhost:8080/copilot"  // Runtime endpoint
  credentials="include"                         // Include cookies
  headers={{                                    // Custom headers
    "Authorization": `Bearer ${token}`
  }}
>
```

### Readable State Configuration

```typescript
useCopilotReadable({
  description: "Clear description of this data",
  value: importantData,
  parentId: "parent-context",  // Nest within context
  categories: ["data", "state"] // Categorize for filtering
});
```

### Action Configuration

```typescript
useCopilotAction({
  name: "action_name",           // Unique identifier
  description: "What this does", // For AI model understanding
  parameters: [                   // Function parameters
    {
      name: "param_name",
      description: "Parameter description",
      type: "string",             // Type for validation
      required: true
    }
  ],
  handler: async (param) => {    // Async function to execute
    // Perform action, update state, call APIs
  }
});
```

## Best Practices

### State Sharing

- ✅ Expose only relevant state that the AI needs
- ✅ Use clear, descriptive names for readable state
- ✅ Update readables reactively as state changes
- ✅ Keep exposed state serializable
- ❌ Don't expose sensitive data (tokens, passwords)
- ❌ Don't expose extremely large objects

### Actions

- ✅ Keep actions focused and single-purpose
- ✅ Include clear parameter descriptions
- ✅ Validate parameters before execution
- ✅ Handle errors gracefully with user feedback
- ✅ Use async/await for long-running operations
- ❌ Don't expose dangerous operations without confirmation
- ❌ Don't rely on implicit context

### Security

- ✅ Validate all AI-triggered actions on the backend
- ✅ Implement proper authentication/authorization
- ✅ Log AI actions for auditing
- ✅ Use rate limiting on action endpoints
- ✅ Sanitize AI outputs before rendering
- ❌ Don't trust client-side validation alone
- ❌ Don't expose internal APIs directly

## Provider Integration

### OpenAI Adapter (Default)

```typescript
import { CopilotKit } from "@copilotkit/react-core";

<CopilotKit 
  runtimeUrl="http://localhost:8080/copilot"
  credentials="include"
/>
```

### Custom Provider Adapter

```typescript
// Implement custom adapter for proprietary LLM
class CustomLLMAdapter extends CopilotKitAdapter {
  async chat(messages, model) {
    // Call your custom LLM endpoint
    const response = await fetch("https://your-llm/chat", {
      method: "POST",
      body: JSON.stringify({ messages, model })
    });
    return response.json();
  }
}

// Use in runtime
const adapter = new CustomLLMAdapter({ apiKey: process.env.CUSTOM_API_KEY });
```

### Self-Hosted LLM

```typescript
// Point to self-hosted Ollama, LM Studio, or vLLM
<CopilotKit
  runtimeUrl="http://localhost:8080/copilot"
  // Runtime configured to use self-hosted endpoint
/>
```

## Real-World Patterns

### Pattern 1: Task Automation in SaaS Dashboard

```typescript
// Share CRM records and deal state
useCopilotReadable({
  description: "Current CRM records visible to user",
  value: crmRecords
});

// Enable bulk actions
useCopilotAction({
  name: "update_deal_status",
  description: "Update the status of one or more CRM deals",
  parameters: [/* ... */],
  handler: async (dealIds, newStatus) => {
    await updateDeals(dealIds, newStatus);
    refreshData();
  }
});
```

### Pattern 2: Productivity App Assistant

```typescript
// Expose task list and user preferences
useCopilotReadable({
  description: "User's tasks and productivity preferences",
  value: { tasks, preferences }
});

// Enable task manipulation
useCopilotAction({
  name: "suggest_task_reorganization",
  description: "Reorganize tasks based on priority",
  handler: async (strategy) => {
    const reorganized = reorganizeTasks(tasks, strategy);
    setTasks(reorganized);
  }
});
```

### Pattern 3: E-Commerce Copilot

```typescript
// Share cart state
useCopilotReadable({
  description: "Shopping cart contents and user profile",
  value: { cart, user }
});

// Enable checkout actions
useCopilotAction({
  name: "apply_promotion",
  description: "Apply a promotional code to the cart",
  handler: async (code) => {
    const discount = await validatePromo(code);
    updateCartDiscount(discount);
  }
});
```

## Troubleshooting

### Runtime Connection Issues

- ✓ Verify runtime endpoint is accessible
- ✓ Check CORS configuration
- ✓ Ensure credentials are correct
- ✓ Check browser console for connection errors

### Actions Not Triggering

- ✓ Verify action names are unique
- ✓ Check parameter types match definitions
- ✓ Ensure handler function is provided
- ✓ Check for JavaScript errors in handler

### State Not Updating

- ✓ Verify readables are wrapped with useCopilotReadable hook
- ✓ Ensure state is being updated in parent component
- ✓ Check that readables run on every render
- ✓ Verify state is serializable

## Related Skills

- **makefile-dev-workflow**: Development workflow for the full stack
- **playwright-ux-ui-capture**: UI capture automation
- **ux-ui-analyze-single-page**: Single page UX analysis

## Resources

- [CopilotKit Official Documentation](https://docs.copilotkit.ai/)
- [CopilotKit GitHub Repository](https://github.com/CopilotKit/CopilotKit)
- [Next.js Documentation](https://nextjs.org/docs)
- [React Hooks Documentation](https://react.dev/reference/react/hooks)

## Support

For issues or questions:

1. Check the [CopilotKit documentation](https://docs.copilotkit.ai/)
2. Review the examples in the skill directory
3. Check runtime logs for errors
4. File an issue with reproduction steps
