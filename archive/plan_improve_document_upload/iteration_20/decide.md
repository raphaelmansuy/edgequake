# Iteration 20: Loading States Enhancement - Decide

## Decision

### Implement Enhanced Skeleton

Replace simple bars with structured skeleton rows matching table columns.

### Implement Enhanced Empty State

Add upload button and improve messaging.

### Code Changes

#### 1. Enhanced Loading Skeleton

```tsx
{isLoading ? (
  <div className="border rounded-lg overflow-hidden">
    {[...Array(5)].map((_, i) => (
      <div key={i} className="flex items-center gap-4 px-4 py-3 border-b last:border-b-0">
        <Skeleton className="h-4 w-4 shrink-0" />
        <Skeleton className="h-4 w-48 shrink-0" />
        <Skeleton className="h-5 w-20 rounded-full shrink-0" />
        <Skeleton className="h-4 w-8 shrink-0" />
        <Skeleton className="h-4 w-12 shrink-0" />
        <Skeleton className="h-4 w-24 shrink-0" />
        <Skeleton className="h-6 w-6 rounded-full shrink-0 ml-auto" />
      </div>
    ))}
  </div>
)
```

#### 2. Enhanced Empty State

```tsx
<div className="text-center py-16 text-muted-foreground">
  <FileText className="h-12 w-12 mx-auto mb-4 opacity-40" />
  <p className="font-medium text-lg">No documents yet</p>
  <p className="text-sm mt-2 max-w-sm mx-auto">
    Drag & drop files above or click to upload. Build your knowledge graph from
    documents.
  </p>
  <Button
    variant="outline"
    className="mt-4"
    onClick={() => inputRef.current?.click()}
  >
    <Upload className="h-4 w-4 mr-2" />
    Upload Documents
  </Button>
</div>
```

### Note

Will need to check if `inputRef` is available or use dropzone's `open()` method.
