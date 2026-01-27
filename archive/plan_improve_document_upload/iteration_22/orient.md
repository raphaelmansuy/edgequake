# Iteration 22: Document List Quick Actions - Orient

## Analysis

### Current Actions Layout
- Eye button for preview (existing)
- Dropdown menu for all other actions

### Enhancement Approach
Add contextual quick action buttons based on document status:

| Status | Quick Actions |
|--------|---------------|
| Completed/Indexed | Preview, View in Graph |
| Failed | Preview, Retry |
| Processing/Pending | Preview only |

### Button Design
- Ghost variant for minimal visual weight
- Tooltips for accessibility
- 8x8 size matching existing button
- Color coding for special actions (orange for retry)

### Implementation Details
1. Add Tooltip component imports
2. Wrap existing preview button with tooltip
3. Add conditional "View in Graph" button
4. Add conditional "Retry" button for failed docs
5. Keep dropdown for less common actions

## Risk Assessment
- Low risk: Additive UI changes
- No logic changes to existing actions
- Consistent with existing button style
