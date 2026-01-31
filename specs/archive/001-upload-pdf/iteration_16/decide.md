# Iteration 16: Decide

## Decision

We will implement retry functionality from the UploadHistory component and then polish the UX for mobile responsiveness.

## Rationale

1. The retry TODO is explicitly noted in the code
2. Backend retry endpoint already exists (`/api/v1/documents/pdf/:id/retry`)
3. Small change with high impact on user experience
4. Aligns with mission success criteria: "retry button"

## Action Items

1. [x] Fix backend compilation errors (Done in this session)
2. [ ] Wire up retry functionality in UploadHistory
3. [ ] Ensure responsive design on progress panel
4. [ ] Commit changes with OODA-16 reference

## Implementation Plan

### Step 1: Wire up UploadHistory retry

The `onRetry` callback is already passed to UploadHistory but currently just logs to console. We need to:

1. Track the PDF/document ID alongside track ID in history
2. Call the reprocess endpoint when retry is clicked
3. Show toast feedback

### Step 2: Responsive improvements

1. Progress panel should stack vertically on mobile
2. Stage indicators should wrap properly
3. Touch-friendly tap targets

## Success Metrics

- [ ] Retry button in history actually reprocesses document
- [ ] Mobile viewport shows all information without horizontal scroll
- [ ] Tests pass

## Testing Strategy

- Unit tests: N/A for wiring changes
- Integration tests: Existing frontend tests should pass
- Manual verification: Click retry in history, verify document reprocesses
