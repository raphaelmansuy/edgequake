# OODA-76: Dark Mode Support

**Date**: 2026-02-01
**Focus**: Theme Switching in Document Viewer

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Consistent theme across components
- PDF viewer theme integration

### Current Theme Implementation

**Theme Provider:**
```typescript
// From providers.tsx
import { ThemeProvider } from 'next-themes';

export function Providers({ children }) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
    >
      {children}
    </ThemeProvider>
  );
}
```

**Theme Toggle:**
```typescript
import { useTheme } from 'next-themes';

const ThemeToggle = () => {
  const { theme, setTheme } = useTheme();
  
  return (
    <Button onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>
      {theme === 'dark' ? <Sun /> : <Moon />}
    </Button>
  );
};
```

## ORIENT

### Theme Variables

| Variable | Light | Dark |
|----------|-------|------|
| --background | 0 0% 100% | 0 0% 3.9% |
| --foreground | 0 0% 3.9% | 0 0% 98% |
| --card | 0 0% 100% | 0 0% 3.9% |
| --muted | 0 0% 96.1% | 0 0% 14.9% |

### PDF Viewer Theme

PDF.js canvas rendering is not affected by CSS.
PDF content remains same in both themes.
Surrounding UI adapts via CSS variables.

## DECIDE

**Decision**: Theme system correctly integrated

The PDF viewer:
- Container adapts to theme
- PDF content is neutral (white pages)
- Controls use theme colors

## ACT

### PDF Viewer Container Styling

```typescript
// Dark mode: PDF on dark container
<div className="bg-muted rounded-lg p-4">
  <Document file={file}>
    {/* Pages render with white background (PDF default) */}
    <Page pageNumber={page} />
  </Document>
</div>
```

### Markdown Viewer Theme

```typescript
// prose-invert for dark mode
<div className="prose prose-sm dark:prose-invert max-w-none">
  <ReactMarkdown>{content}</ReactMarkdown>
</div>
```

### Status Badge Colors

```typescript
// Theme-aware badge variants
const badgeVariants = {
  success: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-800/20 dark:text-emerald-400',
  destructive: 'bg-red-100 text-red-800 dark:bg-red-800/20 dark:text-red-400',
  warning: 'bg-amber-100 text-amber-800 dark:bg-amber-800/20 dark:text-amber-400',
};
```

**Status**: ✅ VERIFIED - Dark mode complete
