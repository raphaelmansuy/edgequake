# OODA Iteration 06 - OBSERVE

## Analysis

Looking at document-manager.tsx upload-related code:

### Upload State (lines 250-252)

```typescript
const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
const [isUploading, setIsUploading] = useState(false);
```

### handleFilesUpload (lines 287-575) - ~290 LINES!

This is the largest function in the file. It handles:

- File type detection (PDF vs text)
- Progress tracking through phases
- Optimistic cache updates
- Toast notifications
- Error handling
- Query invalidation

### removeUploadingFile (lines 579-581)

```typescript
const removeUploadingFile = useCallback((index: number) => {
  setUploadingFiles((prev) => prev.filter((_, i) => i !== index));
}, []);
```

### MAX_FILE_SIZE (line 584)

```typescript
const MAX_FILE_SIZE = 10 * 1024 * 1024;
```

### onDrop (lines 586-614)

Handles file rejections and calls handleFilesUpload.

### useDropzone (lines 702-712)

Configuration for react-dropzone.

## Total Upload Code: ~340 lines

## Dependencies

- queryClient (react-query)
- t (i18n)
- router (next/navigation)
- selectedTenantId, selectedWorkspaceId (tenant store)
- setStatusFilter (state setter)

## Extraction Strategy

Create `useFileUpload` hook with:

1. All upload state
2. handleFilesUpload logic
3. onDrop handler
4. useDropzone configuration

Hook returns:

- uploadingFiles
- isUploading
- getRootProps
- getInputProps
- isDragActive
- openFileDialog
- removeUploadingFile

This removes ~340 lines from DocumentManager.
