# Iteration 19: Keyboard Shortcuts - Decide

## Decision

### Implement Core Shortcuts
Focus on essential shortcuts that improve workflow:

1. **Escape** - Clear selection or close preview
2. **Ctrl/Cmd + A** - Select all documents  
3. **R** - Refresh document list

### Skip for Now
- Delete shortcut (too dangerous without confirmation modal)
- Help dialog (lower priority)

### Code Structure
Add new `useEffect` in document-manager.tsx after line 140 (after existing hooks).

### Implementation
```tsx
// OODA-19: Keyboard shortcuts for power users
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    // Skip if in input field
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') {
      return;
    }

    // Escape: Clear selection or close preview
    if (e.key === 'Escape') {
      if (previewPanelOpen) {
        handlePreviewClose();
      } else if (selectedIds.size > 0) {
        setSelectedIds(new Set());
      }
      return;
    }

    // Ctrl/Cmd + A: Select all
    if ((e.metaKey || e.ctrlKey) && e.key === 'a') {
      e.preventDefault();
      handleSelectAll(true);
      return;
    }

    // R: Refresh (when not in input)
    if (e.key === 'r' && !e.metaKey && !e.ctrlKey) {
      refetch();
      return;
    }
  };

  document.addEventListener('keydown', handleKeyDown);
  return () => document.removeEventListener('keydown', handleKeyDown);
}, [previewPanelOpen, selectedIds.size, handlePreviewClose, handleSelectAll, refetch]);
```

### Keyboard Hint
Add subtle hint in bulk action bar after clear button.
