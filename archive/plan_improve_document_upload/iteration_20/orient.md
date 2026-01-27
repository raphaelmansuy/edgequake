# Iteration 20: Loading States Enhancement - Orient

## Analysis

### Skeleton Design
Enhance skeleton to match table structure:
- Checkbox column: small square
- Title column: longer bar
- Status column: badge-sized bar
- Entities column: short centered bar
- Cost column: short centered bar
- Created column: medium bar
- Actions column: small circle

### Empty State Design
Make empty state more actionable:
- Keep existing icon and text
- Add explicit "Upload files" button
- Mention drag & drop capability
- Reference keyboard shortcut for power users

### Implementation Approach
Create a skeleton row component for better maintainability.

### Code Structure
```tsx
// Skeleton row that matches table structure
const SkeletonRow = () => (
  <div className="flex items-center gap-4 px-4 py-3 border-b">
    <Skeleton className="h-4 w-4" />
    <Skeleton className="h-4 w-48" />
    <Skeleton className="h-5 w-20 rounded-full" />
    <Skeleton className="h-4 w-8 mx-auto" />
    <Skeleton className="h-4 w-12 mx-auto" />
    <Skeleton className="h-4 w-24" />
    <Skeleton className="h-6 w-6 rounded-full" />
  </div>
);
```

## Risk Assessment
- Low risk: Visual-only changes
- No logic changes
- Easy to revert if needed
