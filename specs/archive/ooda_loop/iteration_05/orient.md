# Iteration 05 - ORIENT Phase

## Gap Analysis

### What Works

- Error message is displayed in document row
- Red color indicates error state
- AlertCircle icon provides visual cue
- Title attribute shows full error on hover

### What Needs Improvement

| Gap                                | Impact                            | Priority |
| ---------------------------------- | --------------------------------- | -------- |
| Cannot copy error message          | Users must manually select text   | P1       |
| Truncation hides important details | Debugging is difficult            | P1       |
| No error categorization            | Hard to identify error type       | P2       |
| No mobile-friendly error view      | Title hover doesn't work on touch | P2       |

## Strategic Options

### Option A: Inline Copy Button

Add small copy icon next to error that copies to clipboard.
**Pros**: Minimal UI change, fast to implement
**Cons**: Still truncated, limited info

### Option B: Expandable Error Popover

Click error to show popover with full details + copy button.
**Pros**: Rich information, preserves table layout
**Cons**: More complex implementation

### Option C: Error Details Dialog

Click error to open dialog with formatted error info.
**Pros**: Maximum space for details
**Cons**: Disruptive to workflow

## Recommended Approach: Option B (Popover)

Use Radix UI Popover with:

1. Click trigger on error text
2. Full error message in popover
3. Copy button with clipboard feedback
4. Error timestamp if available
5. "Retry" quick action

## Dependencies

- `@/components/ui/popover` - Already available via shadcn
- `navigator.clipboard.writeText()` - Browser API
- `toast` from sonner - For copy feedback
