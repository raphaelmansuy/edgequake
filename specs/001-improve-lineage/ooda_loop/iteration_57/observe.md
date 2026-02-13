# Observation - Iteration 57

## Cross-Page ScrollArea Consistency Audit

Checked all ScrollArea instances across the application for the Radix `display: table` issue.

### Pages Using ScrollArea

| Page | Component | ScrollArea Location | Has `!block` Override? | Risk |
|------|-----------|---------------------|------------------------|------|
| Graph | NodeDetails panel | graph-viewer.tsx:754 | ✅ Yes | Fixed |
| Graph | EntityBrowser panel | entity-browser-panel.tsx | ❌ No | Low (overflow-hidden parent) |
| Dashboard | Recent Activity | recent-activity.tsx | Via ScrollArea | Low (simple list) |
| Query | Chat messages | query-view.tsx | Via ScrollArea | Low (text only) |
| Documents | Document list | documents page | Via ScrollArea | Low (table layout) |
| Pipeline | Pipeline view | pipeline page | Via ScrollArea | Low (card layout) |

### Analysis

Only the graph right panel (NodeDetails) had a user-visible overflow issue because:
1. It had complex content (property value pairs with buttons)
2. It lacked `overflow-hidden` on a parent container
3. The Radix `display: table` wrapper expanded to content intrinsic width

All other ScrollArea instances either have:
- Simple content that doesn't exceed viewport width
- Parent containers with `overflow-hidden`
- Both
