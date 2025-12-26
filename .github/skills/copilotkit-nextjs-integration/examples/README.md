# CopilotKit Examples

This directory contains working example projects for integrating CopilotKit into Next.js applications.

## Available Examples

### 1. Hello Copilot (`hello-copilot/`)
**Difficulty**: Beginner  
**Time to implement**: 5 minutes

Basic setup with a chat popup. Perfect starting point.

**Features:**
- Runtime endpoint setup
- CopilotKit provider configuration
- CopilotPopup component
- Basic conversation

**To implement:**
See [../typescript/patterns/basic-setup.md](../typescript/patterns/basic-setup.md)

---

### 2. User List Assistant (`user-list-assistant/`)
**Difficulty**: Beginner → Intermediate  
**Time to implement**: 15 minutes

Dashboard with a user list where the copilot can answer questions about the data.

**Features:**
- Readable state (`useCopilotReadable`)
- Context-aware responses
- Dynamic state updates
- Multiple state sources

**Key Code:**
```typescript
useCopilotReadable({
  description: "List of users in the system",
  value: users
});
```

**To implement:**
See [../typescript/patterns/state-sharing.md](../typescript/patterns/state-sharing.md)

**What you can ask:**
- "How many users are there?"
- "What is Alice's email?"
- "Who is the admin?"

---

### 3. Task Automator (`task-automator/`)
**Difficulty**: Intermediate  
**Time to implement**: 30 minutes

Task management app where the copilot can perform actions like adding/completing tasks.

**Features:**
- State sharing (`useCopilotReadable`)
- Tool execution (`useCopilotAction`)
- Real-time UI updates
- Parameter validation
- Human-in-the-loop patterns

**Key Code:**
```typescript
useCopilotAction({
  name: "addTask",
  description: "Add a new task",
  parameters: [{ name: "title", type: "string", required: true }],
  handler: async (params) => {
    setTasks(tasks => [...tasks, newTask(params.title)]);
    return "Task added";
  }
});
```

**To implement:**
See [../typescript/patterns/actions-tools.md](../typescript/patterns/actions-tools.md)

**What you can ask:**
- "Add a task to review the code"
- "Mark the first task as done"
- "What tasks are pending?"
- "Complete all high-priority tasks"

---

### 4. Self-Hosted LLM (`self-hosted-llm/`)
**Difficulty**: Advanced  
**Time to implement**: 45 minutes

Full production setup with:
- Custom LLM adapter for proprietary/self-hosted service
- Authentication on runtime endpoint
- Rate limiting
- Structured logging
- Error handling

**Features:**
- Custom `CopilotServiceAdapter`
- JWT authentication
- Request validation
- Detailed logging
- Timeout management
- Retry logic

**To implement:**
See [../typescript/patterns/custom-adapters.md](../typescript/patterns/custom-adapters.md)  
See [../typescript/patterns/production-security.md](../typescript/patterns/production-security.md)

**Key Code:**
```typescript
export class CustomLLMAdapter implements CopilotServiceAdapter {
  async streamChatCompletion(options: ChatCompletionOptions): Promise<ReadableStream> {
    // Your custom LLM integration
  }
}
```

---

## Quick Start Path

### Day 0: Basic Chat
1. Follow [hello-copilot](hello-copilot/) example
2. Read [../typescript/patterns/basic-setup.md](../typescript/patterns/basic-setup.md)
3. Get a working chat popup

### Week 1: Add Context
1. Follow [user-list-assistant](user-list-assistant/) example
2. Read [../typescript/patterns/state-sharing.md](../typescript/patterns/state-sharing.md)
3. Share app state with the copilot

### Week 1-2: Add Actions
1. Follow [task-automator](task-automator/) example
2. Read [../typescript/patterns/actions-tools.md](../typescript/patterns/actions-tools.md)
3. Let copilot trigger state changes

### Week 2-3: Production Hardening
1. Follow [self-hosted-llm](self-hosted-llm/) example
2. Read [../typescript/patterns/custom-adapters.md](../typescript/patterns/custom-adapters.md)
3. Read [../typescript/patterns/production-security.md](../typescript/patterns/production-security.md)
4. Deploy with auth, validation, logging

---

## Example Project Structure

Each example follows this structure:

```
example-name/
├── README.md              # Specific example documentation
├── app/
│   ├── api/
│   │   └── copilotkit/
│   │       └── route.ts   # Runtime endpoint
│   ├── layout.tsx         # CopilotKit provider
│   ├── page.tsx           # Main page with copilot
│   └── ...
├── lib/
│   ├── custom-adapter.ts  # (If custom LLM)
│   ├── auth.ts            # (If authenticated)
│   └── ...
├── .env.example           # Environment variables template
├── package.json
└── tsconfig.json
```

---

## Testing Examples Locally

```bash
# 1. Install dependencies
npm install

# 2. Copy environment template
cp .env.example .env.local

# 3. Add your LLM API key
# Edit .env.local with your OpenAI, Groq, or custom LLM credentials

# 4. Run development server
npm run dev

# 5. Visit http://localhost:3000
```

---

## Common Issues & Solutions

| Issue | Solution |
| --- | --- |
| "Copilot not connecting" | Check `.env.local` has API key, restart dev server, check Network tab |
| "Timeout on Vercel" | Increase `maxDuration` in `next.config.ts` or use faster adapter (Groq) |
| "State not updating" | Ensure you're using `useState` in action handlers and the component re-renders |
| "Tool parameters rejected" | Validate parameter types match; check CopilotKit version compatibility |

---

## Progression Difficulty

```
hello-copilot                   ⭐
    ↓
user-list-assistant            ⭐⭐
    ↓
task-automator                 ⭐⭐⭐
    ↓
self-hosted-llm                ⭐⭐⭐⭐⭐
```

---

## Next Steps After Examples

- Deploy to Vercel / other hosting
- Implement custom authentication
- Add analytics/monitoring
- Scale readable state with real databases
- Integrate with backend APIs
- Build admin dashboard for managing copilot behavior

---

## References

- [CopilotKit Official Docs](https://docs.copilotkit.ai/)
- [Patterns Documentation](../typescript/patterns/)
- [Instructions](../typescript/instructions.md)
