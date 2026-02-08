# OODA Iteration 08 - OBSERVE

## Target: DocumentDropzone UI

The dropzone UI section (lines 1196-1226) is ~30 lines:

```tsx
{/* Compact Upload Zone - Inline dropzone, no card wrapper */}
<div
  {...getRootProps()}
  className={cn(
    "border-2 border-dashed rounded-lg cursor-pointer transition-all duration-200",
    "flex items-center gap-4 px-4 py-3",
    isDragActive
      ? 'border-primary bg-primary/5 ring-2 ring-primary/20 animate-pulse'
      : 'border-muted-foreground/20 hover:border-primary/50 hover:bg-muted/30'
  )}
>
  <input {...getInputProps()} />
  <div className={cn(...)}>
    <Upload className={cn(...)} />
  </div>
  <div className="flex-1 min-w-0">
    {isDragActive ? (...) : (...)}
  </div>
</div>
```

## Dependencies

- getRootProps: from useDropzone
- getInputProps: from useDropzone
- isDragActive: from useDropzone

## Component Design

```typescript
interface DocumentDropzoneProps {
  getRootProps: () => DropzoneRootProps;
  getInputProps: () => DropzoneInputProps;
  isDragActive: boolean;
}

function DocumentDropzone({
  getRootProps,
  getInputProps,
  isDragActive,
}: DocumentDropzoneProps): JSX.Element;
```

## Lines to Extract: ~30
