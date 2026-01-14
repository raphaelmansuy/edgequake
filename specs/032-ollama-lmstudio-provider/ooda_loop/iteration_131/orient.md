# Iteration 131 – Orient

## Analysis

### Provider/Model Selector Component

Found at [provider-model-selector.tsx](edgequake_webui/src/components/query/provider-model-selector.tsx) (296 lines):

| Feature | Status | Lines |
|---------|--------|-------|
| Search/filter | ✅ CommandInput | 212 |
| Provider grouping | ✅ CommandGroup per provider | 230-281 |
| Provider icons | ✅ OpenAI (green), Ollama (blue), LM Studio (purple) | 68-80 |
| Capability badges | ✅ Vision (eye), Streaming (zap) | 264-272 |
| Context length display | ✅ Shows "128K ctx" format | 275 |
| Default selection | ✅ "Server Default" option | 217-229 |
| Check mark indicator | ✅ Shows on selected item | 227, 279 |
| Unavailable providers | ✅ "(Not configured)" label, disabled | 241, 260 |

### Default Selection Behavior

```tsx
// Use special value for "server default" since empty string is not allowed
const SERVER_DEFAULT = '__server_default__';

// Default shown when no value selected
{!value && <Check className="h-4 w-4 ml-auto" />}
```

### Visual Indicators

| Indicator | Icon | Color |
|-----------|------|-------|
| OpenAI | Cloud | Green |
| Ollama | Cpu | Blue |
| LM Studio | Brain | Purple |
| Mock | FlaskConical | Gray |
| Vision capability | Eye | Blue |
| Streaming | Zap | Yellow |

## Conclusion

**Item 14 (Model/Provider selection UI): VERIFIED COMPLETE**

- Searchable dropdown for easy model discovery
- Visual grouping by provider with icons
- Default model clearly indicated
- Capability badges for quick assessment
