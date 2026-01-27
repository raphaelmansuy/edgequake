# Iteration 19: Keyboard Shortcuts - Orient

## Keyboard Shortcut Plan

### Core Shortcuts
| Key | Action | Condition |
|-----|--------|-----------|
| `Escape` | Clear selection | Has selection |
| `Escape` | Close preview panel | Panel open |
| `Ctrl/Cmd + A` | Select all | Not in input |
| `R` | Refresh documents | Not in input |
| `Delete` | Delete selected | Has selection |

### Implementation Details

1. **useEffect Hook**
   - Add keyboard listener on mount
   - Cleanup on unmount
   - Check `document.activeElement` to avoid input conflicts

2. **Modifier Keys**
   - Use `event.metaKey` for macOS
   - Use `event.ctrlKey` for Windows/Linux
   - Combine with `||` for cross-platform

3. **Input Conflict Prevention**
   - Skip shortcuts when focus is in input/textarea
   - Check `tagName === 'INPUT' || tagName === 'TEXTAREA'`

### Visual Feedback
- Add keyboard hint in bulk action bar: "Press Esc to clear"
- Consider future: help dialog with all shortcuts

## Risk Assessment
- Low risk: Standard keyboard handling
- No visual changes except hints
- Easy to disable if issues arise
