# OODA-65: Toast Notifications

**Date**: 2026-02-01
**Focus**: User Feedback Notifications

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Clear feedback on user actions
- Non-blocking notifications

### Toast Implementation

**Provider Setup (layout.tsx):**
```typescript
import { Toaster } from 'sonner';

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>
        {children}
        <Toaster position="top-right" richColors />
      </body>
    </html>
  );
}
```

**Usage Pattern:**
```typescript
import { toast } from 'sonner';

// Success
toast.success('Document uploaded successfully');

// Error
toast.error('Failed to upload document');

// Loading
const toastId = toast.loading('Uploading...');
// Later:
toast.success('Upload complete', { id: toastId });

// With action
toast('Document uploaded', {
  action: {
    label: 'View',
    onClick: () => router.push(`/documents/${doc.id}`),
  },
});
```

## ORIENT

### Toast Types

| Type | Use Case | Duration |
|------|----------|----------|
| Success | Action completed | 4s |
| Error | Action failed | 6s |
| Info | Informational | 4s |
| Loading | In progress | Until dismissed |
| Warning | Caution needed | 5s |

### Toast Positioning
- Desktop: Top-right corner
- Mobile: Bottom-center
- Stacks up to 3 visible

## DECIDE

**Decision**: Toast implementation is correct

Sonner provides:
- Rich styling out of box
- Promise integration
- Action buttons
- Accessible announcements

## ACT

### Document Upload Toasts

```typescript
const handleUpload = async (file: File) => {
  const toastId = toast.loading(`Uploading ${file.name}...`);
  
  try {
    const result = await uploadDocument(file);
    
    toast.success('Document uploaded', {
      id: toastId,
      action: {
        label: 'View',
        onClick: () => handleViewDetails({ id: result.document_id }),
      },
    });
  } catch (error) {
    toast.error(`Failed to upload ${file.name}`, {
      id: toastId,
      description: error instanceof Error ? error.message : 'Unknown error',
    });
  }
};
```

### Delete Toast with Undo

```typescript
toast.success('Document deleted', {
  action: {
    label: 'Undo',
    onClick: () => restoreDocument(doc.id),
  },
  duration: 8000, // Longer for undo opportunity
});
```

**Status**: ✅ VERIFIED - Toast notifications complete
