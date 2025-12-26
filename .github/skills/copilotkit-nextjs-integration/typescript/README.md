# CopilotKit TypeScript/Next.js Patterns

This directory contains TypeScript-specific patterns, instructions, and best practices for integrating CopilotKit into Next.js applications.

## Files

- **skill.md** - TypeScript-specific skill description with type examples
- **instructions.md** - Step-by-step setup and integration guide
- **patterns/** - Reusable patterns for common scenarios:
  - `basic-setup.md` - Minimal working runtime endpoint and app setup
  - `state-sharing.md` - Using `useCopilotReadable` for context
  - `actions-tools.md` - Defining and executing `useCopilotAction` tools
  - `custom-adapters.md` - Building custom LLM adapters
  - `production-security.md` - Auth, validation, logging, and hardening

## Quick Navigation

**Getting Started:**
1. Read [skill.md](skill.md) for TypeScript patterns overview
2. Follow [instructions.md](instructions.md) for step-by-step setup
3. Implement [patterns/basic-setup.md](patterns/basic-setup.md) first

**Adding Features:**
- State: See [patterns/state-sharing.md](patterns/state-sharing.md)
- Actions: See [patterns/actions-tools.md](patterns/actions-tools.md)
- Custom LLM: See [patterns/custom-adapters.md](patterns/custom-adapters.md)

**Deployment:**
- See [patterns/production-security.md](patterns/production-security.md) for hardening

## Key Type Definitions

```typescript
// Readable state shape
interface ReadableState {
  description: string;
  value: any; // Must be JSON-serializable
}

// Action/Tool definition
interface CopilotAction {
  name: string;
  description: string;
  parameters: Parameter[];
  handler: (params: Record<string, any>) => Promise<string>;
}

interface Parameter {
  name: string;
  type: "string" | "number" | "boolean";
  required: boolean;
  description?: string;
}

// Adapter interface (simplified)
interface CopilotServiceAdapter {
  streamChatCompletion(options: ChatCompletionOptions): Promise<ReadableStream>;
  // Additional methods depend on CopilotKit version
}
```

## Testing TypeScript Code

All code examples can be tested with:
```bash
npm run type-check     # TypeScript compiler
npm run lint           # ESLint with copilotkit rules
npm run build          # Next.js build
npm run dev            # Development server
```

## Related Files

- [../README.md](../README.md) - Overview and high-level concepts
- [../overview.md](../overview.md) - CopilotKit complete guide
- [../examples/](../examples/) - Full working project examples
