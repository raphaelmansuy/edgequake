# OODA Iteration 159 - Function Calling Support

## Observe

### Focus

Verify that function calling is available for structured tool use.

### Investigation

**Function Calling Capability** (from `models.toml`):

```toml
[providers.models.capabilities]
supports_function_calling = true
```

### Support by Model

| Model       | Function Calling |
| ----------- | ---------------- |
| gpt-4o      | ✅               |
| gpt-4o-mini | ✅               |
| gpt-4.1     | ✅               |
| llama3.2    | ✅               |
| gemma3:12b  | ✅               |

## Orient

### Function Calling Use Cases

1. **Tool Integration**: Allow LLM to call external tools
2. **Structured Output**: Force specific response format
3. **Agent Behavior**: Enable autonomous tool use

### Future Potential

- RAG with tool augmentation
- Graph query functions
- Document analysis tools

## Decide

**Status**: ✅ COMPLETE

Function calling is properly configured in model cards.

## Act

### Verified

- `supports_function_calling` flag defined
- Most modern models support it
- Ready for future agent features
- Consistent with OpenAI API spec

---

_Commit: docs(OODA 159): Verify function calling support_
