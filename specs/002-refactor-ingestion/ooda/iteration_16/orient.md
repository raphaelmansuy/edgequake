# OODA-16 Orient: useBulkSelection Hook Design

## Gap Analysis

### Current State
- Selection state (`selectedIds`) managed inline
- Bulk operation handlers defined inline
- Dependencies on queried documents and API functions

### Target State
- Selection state encapsulated in hook
- Bulk operations centralized
- Hook manages its own cache invalidation

## Design Decision: State Location

**Option A: Hook owns selectedIds state**
```typescript
const { selectedIds, handleSelectAll, ... } = useBulkSelection({ documents });
```
- ✅ Complete encapsulation
- ✅ Cleaner component code
- ❌ documents param needed for select all

**Option B: Hook receives selectedIds externally**
```typescript
const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
const { handleSelectAll, ... } = useBulkSelection({ selectedIds, setSelectedIds, documents });
```
- ❌ More verbose
- ✅ Parent controls state

**Decision: Option A** - Hook owns state for maximum encapsulation

## Hook Interface Design

```typescript
interface UseBulkSelectionOptions {
  documents: Document[];
}

interface UseBulkSelectionReturn {
  // State
  selectedIds: Set<string>;
  selectedCount: number;
  isAllSelected: boolean;
  
  // Handlers
  handleSelectAll: (checked: boolean) => void;
  handleSelectOne: (docId: string, checked: boolean) => void;
  handleClearSelection: () => void;
  handleBulkDelete: () => Promise<void>;
  handleBulkReprocess: () => Promise<void>;
  
  // Loading states
  isBulkDeleting: boolean;
  isBulkReprocessing: boolean;
}
```

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| State sync with filters | Clear selection on filter change |
| API calls in hook | Use react-query internally |
| Toast in hook | Toast is global, works fine |

## Next: Decide
