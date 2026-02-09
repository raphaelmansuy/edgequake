# OODA Iteration 04 - Decide

**Mission**: Optimize Knowledge Graph Display & UX
**Mission File**: `specs/001-optimize-graph-mission/mission.md`
**Date**: 2026-02-09

---

## Decision: Implement Screen Reader Accessibility

### Priority: HIGH

Screen reader support is a WCAG requirement and makes the graph usable for visually impaired users.

---

## Specific Changes

### 1. Create `graph-accessibility-announcer.tsx`

**Purpose**: Announce node selection changes to screen readers

**Features**:

- aria-live="polite" region
- Announces node label, type, and degree
- Visually hidden (sr-only)

### 2. Update `graph-viewer.tsx`

- Add role="application" to container
- Add aria-label="Knowledge Graph Visualization"
- Include GraphAccessibilityAnnouncer component

---

## Implementation Details

### Component Structure

```tsx
// graph-accessibility-announcer.tsx
export function GraphAccessibilityAnnouncer() {
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const [announcement, setAnnouncement] = useState("");

  useEffect(() => {
    if (!selectedNodeId) {
      setAnnouncement("No node selected");
      return;
    }

    const node = nodes.find((n) => n.id === selectedNodeId);
    if (!node) return;

    const degree = edges.filter(
      (e) => e.source === selectedNodeId || e.target === selectedNodeId,
    ).length;

    setAnnouncement(
      `Selected: ${node.label}, type ${node.node_type}, ${degree} connections`,
    );
  }, [selectedNodeId, nodes, edges]);

  return (
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className="sr-only"
    >
      {announcement}
    </div>
  );
}
```

### Integration Location

**File**: `graph-viewer.tsx:530` (inside graph container)

---

## Acceptance Criteria

- [ ] Screen reader announces node selection
- [ ] No visual impact (sr-only)
- [ ] Announcements are polite (don't interrupt)
- [ ] TypeScript compiles without errors
- [ ] Tests continue to pass

---

## Non-Goals (This Iteration)

- Edge descriptions
- Full graph description
- Color contrast validation (next iteration)
