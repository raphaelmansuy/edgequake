# OODA-68: Back Button Behavior

**Date**: 2026-02-01
**Focus**: Navigation Stack Management

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Predictable back navigation
- Browser history integration

### Current Back Navigation

**From document detail page:**
```typescript
import { useRouter } from 'next/navigation';

const router = useRouter();

const handleBack = () => {
  router.back();  // Uses browser history
};

// Alternative: explicit navigation
const handleBackToList = () => {
  router.push('/documents');
};
```

## ORIENT

### Navigation Stack Behavior

```
[Documents List] → browser history push
      ↓
[Document Detail] → browser history push
      ↓
[Back button] → browser history pop
      ↓
[Documents List] → restored from history
```

### Back Button Strategies

| Strategy | Behavior | Use Case |
|----------|----------|----------|
| router.back() | History pop | Normal navigation |
| router.push() | History push | Reset to known state |
| router.replace() | History replace | Redirect without history |

## DECIDE

**Decision**: router.back() is correct for detail → list

Provides:
- Natural browser behavior
- Preserves scroll position
- Works with browser back button

## ACT

### Back Button Component

```typescript
const BackButton = ({ fallback = '/' }: { fallback?: string }) => {
  const router = useRouter();
  
  // Check if we can go back
  const handleBack = () => {
    if (window.history.length > 1) {
      router.back();
    } else {
      router.push(fallback);
    }
  };
  
  return (
    <Button variant="ghost" onClick={handleBack}>
      <ArrowLeft className="h-4 w-4 mr-2" />
      Back
    </Button>
  );
};
```

### Document Detail Header

```typescript
<div className="flex items-center gap-4 mb-6">
  <BackButton fallback="/documents" />
  <h1 className="text-2xl font-bold">{document.title}</h1>
</div>
```

### Keyboard Support

```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    // Alt+Left for back (like browser)
    if (e.altKey && e.key === 'ArrowLeft') {
      router.back();
    }
  };
  
  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, [router]);
```

**Status**: ✅ VERIFIED - Back button complete
