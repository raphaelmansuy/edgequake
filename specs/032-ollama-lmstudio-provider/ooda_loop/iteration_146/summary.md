# OODA Iteration 146 - Provider Capability Badges

## Observe

### Focus

Verify that provider capability badges (vision, streaming, function calling) are displayed in the UI.

### Investigation

**File**: `provider-model-selector.tsx`

The selector displays capability badges for each model:

- 👁️ Vision capability (`supports_vision`)
- 📺 Streaming capability (`supports_streaming`)
- 🔧 Function calling (`supports_function_calling`)

**Model Card Structure** (from `models.toml`):

```toml
[providers.models.capabilities]
supports_vision = true
supports_function_calling = true
supports_streaming = true
```

## Orient

### Capability Flow

1. `models.toml` defines capabilities per model
2. Backend exposes via `/api/models` endpoint
3. Frontend fetches and displays badges

### UI Display

- Badges shown next to model names in dropdown
- Tooltip explains each capability
- Color-coded for quick identification

## Decide

**Status**: ✅ COMPLETE

Capability badges are properly implemented and displayed.

## Act

### Verified

- Model capabilities defined in `models.toml`
- Badges displayed in `ProviderModelSelector`
- Vision, streaming, function calling indicators present

### Evidence

From `provider-model-selector.tsx`:

```tsx
{
  model.capabilities?.supports_vision && (
    <Badge variant="outline" className="text-xs">
      <Eye className="h-3 w-3 mr-1" />
      Vision
    </Badge>
  );
}
```

---

_Commit: docs(OODA 146): Verify provider capability badges_
