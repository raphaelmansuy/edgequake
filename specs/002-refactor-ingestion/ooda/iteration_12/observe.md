# OODA-12: Observe

## Target: DocumentTableStates Component Extraction

### Current Location
- **File**: `edgequake_webui/src/components/documents/document-manager.tsx`
- **Lines**: ~1170-1200 (~30 lines for loading + empty states)

### Code Analysis

Two conditional states to extract:
1. **Loading Skeleton** (~15 lines)
   - 5 skeleton rows matching table structure
   - Animated pulse effect

2. **Empty State** (~15 lines)
   - Centered icon and text
   - Description text
   - Upload button CTA

### Dependencies Identified
- `Skeleton` from UI components
- `Button` from UI components
- `FileText, Upload` icons
- `openFileDialog` function from dropzone

### Props Required
```typescript
interface DocumentTableStatesProps {
  isLoading: boolean;
  isEmpty: boolean;
  onUploadClick: () => void;
  rowCount?: number; // For skeleton rows (default 5)
}
```

### Estimated Savings
- **Lines to extract**: ~30 lines
- **Expected reduction**: ~25 lines (accounting for component usage)
