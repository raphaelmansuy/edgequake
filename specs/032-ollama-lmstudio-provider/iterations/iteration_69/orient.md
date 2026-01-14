# OODA 69 - Orient: Provider Selector UI E2E Test Strategy

## Analysis

### Test Strategy for Query Interface Provider Selection

The ProviderModelSelector component needs E2E validation to ensure:

1. **Visibility**: Selector is present in query interface
2. **Interactivity**: Can open dropdown and see providers
3. **Data Population**: Providers from API appear in dropdown
4. **Selection Feedback**: Selected model is displayed correctly

### Component Structure (from code analysis)

```
ProviderModelSelector
├── Trigger button (shows current selection)
├── Dropdown content
│   └── Provider groups
│       └── Model options
└── Selection callback → parent state
```

### Test Scenarios

| Scenario | Priority | Complexity |
|----------|----------|------------|
| Selector is visible on query page | P0 | Low |
| Dropdown opens on click | P0 | Low |
| Shows provider names in dropdown | P1 | Medium |
| Shows model options | P1 | Medium |
| Selection updates display | P1 | Medium |

## Recommendation

Add 2 focused E2E tests:
1. "query page has provider model selector" - visibility check
2. "provider selector shows available providers" - dropdown functionality
