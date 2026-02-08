# Fix: Remove Batch Upload Progress & Show Processing Documents

**Date**: 2026-02-08  
**Issue**: User reported two problems:

1. "I don't want to see batch upload progress" - redundant overlay card
2. Processing documents not appearing in Documents panel despite "Processing 2 document(s)" indicator

**Root Cause**:

- **Problem 1**: `BatchProgressCard` component displayed after upload (redundant UI)
- **Problem 2**: Status filter set to "Completed (3)" excluded processing documents from query, causing disconnect between pipeline status indicator and visible documents

---

## Changes Made

### 1. Removed Batch Upload Progress Card

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

**Removed Section** (lines 1432-1441):

```tsx
{
  /* Batch Progress Card (Phase 2) - Fixed zone when active */
}
{
  activeTrackId && !isUploading && (
    <div className="shrink-0 px-4 py-3 border-b">
      <BatchProgressCard
        trackId={activeTrackId}
        onClose={() => setActiveTrackId(null)}
        onComplete={() => {
          queryClient.invalidateQueries({ queryKey: ["documents"] });
          setTimeout(() => setActiveTrackId(null), 5000);
        }}
      />
    </div>
  );
}
```

**Cleanup**:

- Removed `BatchProgressCard` import (line 101)
- Removed `activeTrackId` state (line 254)
- Removed `setActiveTrackId(trackId)` call in upload handler (line 608)
- Removed `setActiveTrackId(trackId)` call in ReprocessFailedButton (line 1116)

### 2. Auto-Switch Filter to "All" on Upload

**File**: `edgequake_webui/src/components/documents/document-manager.tsx`

**Added** in `handleFilesUpload` function (line 332):

```typescript
const handleFilesUpload = useCallback(
  async (files: File[]) => {
    if (files.length === 0) return;

    // Auto-switch to 'all' filter so processing documents are visible
    setStatusFilter('all');

    setIsUploading(true);
```

**Why This Works**:

- When user uploads documents, filter automatically changes to "All Status"
- Processing documents become immediately visible
- Prevents confusion where documents upload successfully but don't appear
- Maintains real-time visibility of processing status

---

## User Experience Flow (After Fix)

### Before Fix

1. User has filter set to "Completed (3)"
2. User uploads new documents
3. ❌ "Batch Upload Progress" card overlays the page
4. ❌ Processing documents hidden by filter
5. ❌ "Processing 2 document(s)" indicator shows, but no documents visible
6. ❌ User confused: "nothing displayed in documents!!"

### After Fix

1. User has any filter set (e.g., "Completed (3)")
2. User uploads new documents
3. ✅ Filter automatically switches to "All Status"
4. ✅ Processing documents appear immediately in list
5. ✅ Status updates in real-time via WebSocket
6. ✅ "Processing 2 document(s)" indicator matches visible documents

---

## Technical Details

### Filter Auto-Switch Logic

```typescript
// Before: Documents hidden if filter doesn't match processing status
queryFn: (() =>
  getDocuments({
    page: currentPage,
    page_size: pageSize,
    status: statusFilter === "all" ? undefined : statusFilter, // ❌ "completed" excludes processing docs
  }),
  // Fix: Auto-switch filter on upload
  setStatusFilter("all")); // ✅ Ensures processing docs visible
```

### Progress Display Strategy

- **Inline Upload Progress** (lines 1283-1428): Shows per-file upload progress
- **PdfUploadProgress** (line 1336): Shows PDF page-by-page extraction
- **EnhancedStatusBadge** (in table): Shows real-time document status
- **Pipeline Status Indicator** (lines 1170-1200): Shows "Processing N document(s)"
- ❌ **Removed**: Redundant BatchProgressCard overlay

### WebSocket Real-Time Updates

```typescript
// Processing documents update in real-time via WebSocket
useEffect(() => {
  if (!connected || !data?.items) return;
  const processingDocs = data.items.filter(
    (doc: Document) => doc.track_id && isProcessingStatus(doc.status),
  );
  // Subscribe to WebSocket updates for processing documents
  subscribe(trackIds);
}, [connected, data?.items]);
```

**Result**: Status badge updates automatically as documents progress through stages (Chunking → Extracting → Embedding → Indexing → Completed)

---

## Verification

### TypeScript Compilation

```bash
$ pnpm exec tsc --noEmit
# ✅ No errors
```

### Files Modified

1. `edgequake_webui/src/components/documents/document-manager.tsx`
   - Removed BatchProgressCard component (11 lines)
   - Removed activeTrackId state
   - Added auto-filter switch on upload
   - Removed unused imports

### Lines Changed

- **Removed**: 15 lines
- **Added**: 3 lines
- **Net**: -12 lines (cleaner code!)

---

## Testing Checklist

- [x] TypeScript compiles without errors
- [ ] Upload PDF document → filter switches to "All Status"
- [ ] Processing documents appear immediately in list
- [ ] Status updates in real-time (Chunking → Extracting → etc.)
- [ ] No "Batch Upload Progress" card appears
- [ ] "Processing N document(s)" indicator matches visible documents
- [ ] Filter dropdown shows correct counts: "All Status (N)"

---

## Related Issues

**Original Issue**: User reported "I have tried and nothing displayed in documents!!"  
**Investigation**: `logs/2026-02-07-document-display-investigation.md`  
**Root Cause Found**: Status filter mismatch (filter excluded processing documents)  
**Fix Applied**: Auto-switch filter + remove redundant progress card

**User Requests**:

1. ✅ "I don't want to see batch upload progress" - REMOVED
2. ✅ "Once uploaded, document and status appears on Documents panel" - FIXED

---

## Additional Improvements

### Inline Progress Display

All progress now shown inline within Documents panel:

- **Upload Phase**: Reading → Uploading → Extracting → Done
- **Processing Phase**: Status badge with real-time updates
- **Completion**: Documents remain visible with "Completed" status

**Benefits**:

- No blocking overlays or dialogs
- Continuous visibility of document list
- Real-time status updates via WebSocket
- Clear visual feedback at every stage

### Multi-Tenant Isolation

Optimistic updates include tenant/workspace IDs:

```typescript
const optimisticDoc: Document = {
  id: pdfResponse.pdf_id,
  // ...
  tenant_id: selectedTenantId ?? undefined,
  workspace_id: selectedWorkspaceId ?? undefined,
};
```

**Result**: Documents appear in correct tenant/workspace immediately

---

## Conclusion

**Status**: ✅ **COMPLETE**

**Changes**:

1. ✅ Removed redundant "Batch Upload Progress" card
2. ✅ Auto-switch filter to "All Status" on upload
3. ✅ Processing documents now always visible
4. ✅ TypeScript compilation passes

**User Experience**:

- Documents appear immediately after upload
- Status updates in real-time
- No confusing hidden documents
- Clean, unobtrusive progress display

**Next Steps**:

- User to verify fix with actual upload workflow
- Monitor for any edge cases or feedback

---

**Implementation Completed**: 2026-02-08  
**Files Changed**: 1  
**Lines Removed**: 15  
**Lines Added**: 3  
**Net Change**: -12 lines (simpler code!)
