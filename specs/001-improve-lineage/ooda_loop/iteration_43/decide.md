# Decision - Iteration 43

## Changes to Make

1. **`quick-action-buttons.tsx`**: Add `aria-label={label}` to `ActionButton`'s `<Button>` component
   - Impact: Fixes 37 buttons (3-4 per row × 12 rows)

2. **`document-actions-menu.tsx`**: Add `aria-label="More actions"` to DropdownMenuTrigger `<Button>`
   - Impact: Fixes 12 buttons (1 per row × 12 rows)

3. **`document-search-bar.tsx`**: Add `aria-label="Search documents"` to `<Input>`
   - Impact: Fixes 1 input

4. **`pagination-controls.tsx`**: Add `aria-label` to 4 pagination buttons
   - Labels: "First page", "Previous page", "Next page", "Last page"
   - Impact: Fixes 4 buttons

5. **`document-table-section.tsx`**: Add `aria-label="Documents list"` to `<Table>`, `scope="col"` to all `<TableHead>`, `<span className="sr-only">Actions</span>` to empty actions header
   - Impact: Fixes table semantics

## Priority

1. **High impact**: ActionButton aria-labels (37 buttons)
2. **High impact**: MoreVertical menu labels (12 buttons)
3. **Medium impact**: Pagination labels (4 buttons)
4. **Medium impact**: Table semantics (7 headers)
5. **Low impact**: Search input label (1 input)

## Expected Outcome

- All 52+ unnamed buttons gain accessible names
- Table structure fully identified by assistive technology
- Search input properly labeled
- WCAG 2.1 Level A compliance for all identified violations
