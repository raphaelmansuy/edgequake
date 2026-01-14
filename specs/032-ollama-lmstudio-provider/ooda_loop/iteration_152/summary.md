# OODA Iteration 152 - Deprecated Model Handling

## Observe

### Focus

Verify that deprecated models are properly marked and handled.

### Investigation

**Deprecated Model Example** (from `models.toml`):

```toml
[[providers.models]]
name = "text-embedding-ada-002"
display_name = "Text Embedding Ada 002"
model_type = "embedding"
description = "Legacy embedding model, use text-embedding-3-small instead"
deprecated = true
tags = ["legacy"]
```

### Deprecated Models

| Model                  | Reason                             |
| ---------------------- | ---------------------------------- |
| text-embedding-ada-002 | Replaced by text-embedding-3-small |

## Orient

### Deprecation Handling

1. **Visual Indicator**: Deprecated models shown with warning
2. **Description**: Suggests replacement model
3. **Filtering**: Can be excluded from default lists
4. **Compatibility**: Still functional for existing workspaces

### UI Behavior

- Deprecated models shown at bottom of list
- Warning icon displayed
- Description explains replacement

## Decide

**Status**: ✅ COMPLETE

Deprecated models are properly marked with `deprecated = true`.

## Act

### Verified

- `deprecated` field available in model config
- Legacy tag accompanies deprecated models
- Description suggests replacement
- Still selectable for backward compatibility

---

_Commit: docs(OODA 152): Verify deprecated model handling_
