# Plan: Dashboard Layout Improvements

**Addresses:** F-DB-01, F-DB-02, F-DB-04, F-DB-06  
**Files to change:** `src/components/dashboard/quick-actions.tsx`, `src/app/(dashboard)/page.tsx`

---

## Changes

### 1. Remove color tints from Quick Actions

Replace `bg-blue-500/10`, `bg-purple-500/10`, `bg-green-500/10` with neutral hover states.

```tsx
// Before:
className={cn(
  'flex flex-col items-center justify-center gap-2 rounded-lg border p-4',
  action.bgColor,  // ← bg-blue-500/10
  'hover:border-primary/50 hover:shadow-md hover:-translate-y-0.5'
)}

// After:
className={cn(
  'flex flex-col items-center justify-center gap-2 rounded-lg border p-4',
  'bg-card hover:bg-muted/40 hover:border-border',
  'transition-all duration-150 hover:shadow-sm'
)}
```

### 2. Improve contextual header

Replace generic welcome text with workspace context:

```tsx
// Before:
<p>Welcome to EdgeQuake - Your Knowledge Graph RAG Platform</p>

// After:  
<p className="text-sm text-muted-foreground">
  {selectedWorkspace?.name} · {documentValue} documents · {lastActivity}
</p>
```

### 3. Expand System Status card

Show full health breakdown instead of a single line.

### 4. Empty state for zero documents

Add hero call-to-action when `documentValue === 0`.

---

## Acceptance Criteria

- [ ] Quick Actions cards are visually neutral (no color tints)
- [ ] Page header shows workspace context, not generic marketing
- [ ] System status shows more than one line
- [ ] Zero-document state has a clear CTA
