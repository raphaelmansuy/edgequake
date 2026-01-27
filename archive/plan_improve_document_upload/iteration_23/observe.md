# Iteration 23: Processing Status Summary Card - Observe

## Current State Analysis

### Existing Pipeline Status
There's a pipeline status dialog accessible via button, but no at-a-glance summary.

### User Pain Points
1. Need to click to see pipeline status
2. No quick visibility of processing queue
3. Hard to know if documents are being processed

### Enhancement Opportunity
Add a compact status summary card that shows:
- Documents currently processing
- Queue depth
- Failed documents count
- Quick visual indicators

### Location Options
1. Above document list header
2. Next to "Documents (X)" header
3. In the filters area

### Design Considerations
- Compact: Should not take too much space
- Informative: Show key metrics at a glance
- Interactive: Click to open full pipeline dialog
