# OODA-10: Observe

## Target: QuickActionButtons Component Extraction

### Current Location
- **File**: `edgequake_webui/src/components/documents/document-manager.tsx`
- **Lines**: 1341-1430 (~90 lines)

### Code Analysis

Quick action buttons per document row:
1. **View Details** - ExternalLink icon → `handleViewDetails(doc)`
2. **Preview** - Eye icon → `handleDocumentClick(doc)` (side panel)
3. **View in Graph** - Sparkles icon → `handleViewInGraph(doc)` (conditional: completed/indexed)
4. **Retry** - RefreshCw icon → `reprocessMutation.mutate(doc.id)` (conditional: failed/partial_failure)
5. **Actions Dropdown** - Already extracted to `DocumentActionsMenu`

### Current Line Count
- DocumentManager: **1519 lines** (target: <300)
- This extraction: **~85 lines** potential savings

### Dependencies Identified
- `Button` from `@/components/ui/button`
- `Tooltip, TooltipContent, TooltipProvider, TooltipTrigger` from tooltip
- `ExternalLink, Eye, Sparkles, RefreshCw` from lucide-react
- Document type and status checks
- Mutation state (`reprocessMutation.isPending`)

### Props Required
```typescript
interface QuickActionButtonsProps {
  doc: DocumentResponse;
  onViewDetails: (doc: DocumentResponse) => void;
  onPreview: (doc: DocumentResponse) => void;
  onViewInGraph: (doc: DocumentResponse) => void;
  onRetry: (id: string) => void;
  isRetrying: boolean;
  children?: React.ReactNode; // For DocumentActionsMenu slot
}
```

### Status Constants
- COMPLETED_STATUSES = ['completed', 'indexed']
- FAILED_STATUSES = ['failed', 'partial_failure']
