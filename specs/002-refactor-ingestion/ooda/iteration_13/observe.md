# OODA-13: Observe

## Target: useFileUpload Hook Extraction

### Current Location

- **File**: `edgequake_webui/src/components/documents/document-manager.tsx`
- **Lines**: ~264-600 (~340 lines total)

### Functions to Extract

1. **handleFilesUpload** (~290 lines) - Main upload handler
2. **removeUploadingFile** (~3 lines) - Remove file from list
3. **handleUploadComplete** (~8 lines) - Mark upload complete
4. **handleUploadFailed** (~8 lines) - Mark upload failed

### State to Manage

- `uploadingFiles: UploadingFile[]` - List of files being uploaded
- `isUploading: boolean` - Upload in progress flag

### Dependencies

- `selectedTenantId`, `selectedWorkspaceId` - From tenant store
- `queryClient` - For cache updates
- `router` - For navigation after upload
- `t` - Translation function
- `uploadDocument`, `uploadPdfDocument` - API functions
- `setStatusFilter` - To switch filter during upload

### Props Required for Hook

```typescript
interface UseFileUploadOptions {
  tenantId?: string | null;
  workspaceId?: string | null;
  onStatusFilterChange?: (status: string) => void;
}
```

### Estimated Savings

- **Lines to extract**: ~340 lines
- **Expected reduction**: ~320 lines
- **Hook size**: ~350 lines (encapsulated logic)
