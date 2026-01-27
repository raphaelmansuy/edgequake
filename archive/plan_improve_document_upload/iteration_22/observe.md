# Iteration 22: Document List Quick Actions - Observe

## Pivot from Toast Enhancements
Toast styling changes would require refactoring 20+ files. 
Pivoting to a higher-value enhancement.

## Current State Analysis

### Document Row Actions
Current implementation uses dropdown menu for all actions:
- View
- View in Graph
- Reprocess
- Delete

### User Pain Points
1. Too many clicks to perform common actions
2. Dropdown adds friction to frequent operations
3. Failed documents don't have prominent retry button

### Enhancement Opportunity
Add inline quick action buttons for common operations:
- Eye icon for quick preview
- Graph icon for view in graph
- Retry icon for failed documents (inline, no dropdown)

### Files to Modify
- src/components/documents/document-manager.tsx
  - Add inline action buttons in table row
  - Keep dropdown for less common actions
