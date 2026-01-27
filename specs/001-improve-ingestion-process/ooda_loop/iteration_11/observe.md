# Observe - Iteration 11: Document Processing Progress Indicator

## Current State
- Status badge shows sub-states (chunking, extracting, embedding, indexing)
- But no visual indication of overall progress through stages
- Users can't tell how far through the pipeline a document is

## Gap Analysis
1. **Stage Progress**: No visual progress bar for individual documents
2. **Stage Description**: Brief description of what's happening would help
3. **Time Estimate**: Per-stage ETA could help set expectations

## UI Location Options
1. In document table row (inline progress)
2. In a detail panel/drawer when selected
3. As tooltip on status badge
4. As expanded row details

## Recommended Approach
Option 3 (tooltip on status badge) - least disruptive to current UI:
- Hover over status badge to see stage details
- Show progress through stages (1/4, 2/4, etc)
- Brief description of current stage

## Data Requirements
- Current status from API
- Stage order (chunking → extracting → embedding → indexing)
- Stage descriptions
- Optional: started_at timestamp per stage

## Next Step
Update status-badge.tsx to include tooltip with stage progress
