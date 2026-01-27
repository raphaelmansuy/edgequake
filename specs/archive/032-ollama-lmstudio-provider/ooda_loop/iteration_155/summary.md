# OODA Iteration 155 - Server Default Selection

## Observe

### Focus

Verify that "Server Default" option works in model selection.

### Investigation

**Server Default UI** (from `provider-model-selector.tsx`):

The selector includes a "Server Default" option that uses workspace or server configuration.

### Default Configuration

```toml
[defaults]
llm_provider = "ollama"
llm_model = "gemma3:12b"
embedding_provider = "ollama"
embedding_model = "embeddinggemma"
```

## Orient

### Default Selection Flow

```
User opens query page
    │
    ▼
Model selector shows "Server Default"
    │
    ▼
Query uses workspace.llm_provider/model
    │
    ▼
Falls back to [defaults] from models.toml
```

### UI Behavior

1. "Server Default" shown at top of list
2. Shows current default in description
3. Selecting uses workspace config
4. Falls back to global defaults

## Decide

**Status**: ✅ COMPLETE

Server default selection is properly implemented.

## Act

### Verified

- "Server Default" option in dropdown
- Workspace configuration respected
- Falls back to models.toml defaults
- Clear indication of current default

### Default Resolution

| Priority | Source                  |
| -------- | ----------------------- |
| 1        | Workspace configuration |
| 2        | Tenant default          |
| 3        | models.toml [defaults]  |

---

_Commit: docs(OODA 155): Verify server default selection_
