# OODA-13: Orient

## Analysis

### SRP Assessment

The file upload logic represents a coherent unit of functionality:

- Manages upload state (files, progress, errors)
- Handles sequential file processing
- Manages optimistic cache updates
- Handles success/error toasts

**Verdict**: Strong SRP candidate - upload orchestration is a distinct responsibility.

### Complexity Considerations

This is the largest extraction yet (~340 lines). Considerations:

- Many dependencies need to be passed as options
- DOM-related callbacks (onDrop) stay in component
- QueryClient operations stay in hook
- Translation function passed as dependency

### Hook Boundary

**Include in hook**:

- uploadingFiles state
- isUploading state
- handleFilesUpload
- removeUploadingFile
- handleUploadComplete
- handleUploadFailed

**Keep in component**:

- onDrop (tied to useDropzone)
- MAX_FILE_SIZE constant

### Testing Benefit

Extracting to a hook allows testing upload logic independently:

- Mock API calls
- Test state transitions
- Test error handling
- Test cache updates
