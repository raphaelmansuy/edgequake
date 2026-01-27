# Iteration 24: Document Sort Persistence - Decide

## Decision

### Implement localStorage Persistence

Use lazy initialization and useEffect to persist sort preferences.

### Storage Key

`edgequake:documents:sort`

### Code Changes

1. **Lazy State Initialization**

```tsx
const [sortField, setSortField] = useState<SortField>(() => {
  if (typeof window === "undefined") return "created_at";
  try {
    const stored = localStorage.getItem("edgequake:documents:sort");
    return stored ? JSON.parse(stored).field : "created_at";
  } catch {
    return "created_at";
  }
});

const [sortDirection, setSortDirection] = useState<SortDirection>(() => {
  if (typeof window === "undefined") return "desc";
  try {
    const stored = localStorage.getItem("edgequake:documents:sort");
    return stored ? JSON.parse(stored).direction : "desc";
  } catch {
    return "desc";
  }
});
```

2. **Persist on Change**

```tsx
useEffect(() => {
  localStorage.setItem(
    "edgequake:documents:sort",
    JSON.stringify({
      field: sortField,
      direction: sortDirection,
    }),
  );
}, [sortField, sortDirection]);
```

### Rationale

- User preference persists across sessions
- Zero backend requirements
- Graceful degradation if localStorage unavailable
