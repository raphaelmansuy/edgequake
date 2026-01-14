# OODA Iteration 158 - System Message Support

## Observe

### Focus

Verify that system messages are supported for context setting.

### Investigation

**System Message Capability** (from `models.toml`):

```toml
[providers.models.capabilities]
supports_system_message = true
```

### Usage in EdgeQuake

System messages used for:

- Entity extraction prompts
- Query context setting
- RAG instructions

## Orient

### System Message Architecture

```
Request
├── system: "You are a knowledge graph assistant..."
├── user: "What is the relationship between X and Y?"
└── assistant: (previous response if any)
```

### Support by Provider

| Provider  | System Message |
| --------- | -------------- |
| OpenAI    | ✅ Native      |
| Ollama    | ✅ Native      |
| LM Studio | ✅ Native      |
| Anthropic | ✅ Native      |

## Decide

**Status**: ✅ COMPLETE

System messages are supported by all LLM providers.

## Act

### Verified

- `supports_system_message` flag defined
- All LLM models support system messages
- Used in extraction and query prompts
- Consistent behavior across providers

---

_Commit: docs(OODA 158): Verify system message support_
