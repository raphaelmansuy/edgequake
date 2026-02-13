# Decision - Iteration 52

## Changes to Make

1. **recent-activity.tsx:94** — Add padding to dashboard scroll content
   - `<div className="space-y-2">` → `<div className="space-y-2 py-1">`
   - Adds 4px top/bottom padding

2. **entity-browser-panel.tsx:769** — Increase entity browser vertical padding
   - `<div className="p-1.5 space-y-0.5">` → `<div className="py-2 px-1.5 space-y-0.5">`
   - Increases vertical from 6px to 8px

## Priority

1. Dashboard recent-activity (critical — 0px padding is a UX defect)
2. Entity browser (minor — improves shadow indicator clearance)

## Expected Outcome

- Dashboard: first/last items have 4px buffer from scroll boundaries
- Entity browser: cleaner separation from shadow indicators
- No visual regression in other areas
