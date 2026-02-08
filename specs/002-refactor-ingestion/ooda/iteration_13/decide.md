# OODA-13: Decide

## Action Plan

1. **Create `use-file-upload.ts`** hook in `@/hooks/`
   - Typed options interface
   - All upload state management
   - All upload callbacks
   - Return object with state and handlers

2. **Update `document-manager.tsx`**
   - Import and use the hook
   - Remove inline upload logic
   - Keep onDrop wiring to useDropzone

3. **Verify**
   - TypeScript check
   - Upload functionality works properly

## Expected Outcome
- **Lines saved**: ~320 lines from DocumentManager
- **New hook**: ~360 lines (complete upload logic)
- **Target**: DocumentManager 1373 → ~1050 lines

## Implementation Notes
- Hook returns: `{ uploadingFiles, isUploading, handleFilesUpload, removeUploadingFile, handleUploadComplete, handleUploadFailed }`
- Dependencies passed via options object
