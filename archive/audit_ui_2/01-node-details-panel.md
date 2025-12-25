# UI Audit: Node Details Panel

**Screen:** Entity/Node Details Side Panel  
**Date:** 2025-12-25  
**Priority:** High - Core interaction component

---

## Screenshot Analysis

The node details panel displays entity information including name, type, description, properties, metadata, and relationships.

---

## Issues Identified

### Critical Issues

| ID     | Issue                                                                                                                 | Location           | Severity    |
| ------ | --------------------------------------------------------------------------------------------------------------------- | ------------------ | ----------- |
| NDP-01 | **Truncated property values** - `source_ids`, `tenant_id`, `workspace_id` are cut off with "..." making them unusable | Properties section | 🔴 Critical |
| NDP-02 | **No copy button for truncated values** - Users cannot access full UUID values                                        | Properties section | 🔴 Critical |

### High Priority Issues

| ID     | Issue                                                                                                            | Location      | Severity |
| ------ | ---------------------------------------------------------------------------------------------------------------- | ------------- | -------- |
| NDP-03 | **Inconsistent section spacing** - DESCRIPTION, PROPERTIES, METADATA, RELATIONSHIPS have different vertical gaps | All sections  | 🟠 High  |
| NDP-04 | **Entity type badge too small** - "PRODUCT" badge has minimal padding, hard to read                              | Header        | 🟠 High  |
| NDP-05 | **Relationship arrows lack clarity** - Left/right arrows (← →) meaning is unclear without legend                 | Relationships | 🟠 High  |
| NDP-06 | **Description box inconsistent** - Has info icon and box styling, but Properties/Metadata don't                  | DESCRIPTION   | 🟠 High  |

### Medium Priority Issues

| ID     | Issue                                                                                          | Location                  | Severity  |
| ------ | ---------------------------------------------------------------------------------------------- | ------------------------- | --------- |
| NDP-07 | **Property labels not aligned** - Labels like "description", "entity_type" have varying widths | Properties                | 🟡 Medium |
| NDP-08 | **ID field copyable but not obvious** - Copy icon exists but small and low contrast            | Metadata                  | 🟡 Medium |
| NDP-09 | **Connections badge style differs from Relationships badge** - Inconsistent badge styling      | Metadata vs Relationships | 🟡 Medium |
| NDP-10 | **Action buttons cramped** - Edit, Merge, Delete buttons at bottom have minimal breathing room | Footer                    | 🟡 Medium |
| NDP-11 | **Delete button red text on white** - Destructive action not visually prominent enough         | Footer                    | 🟡 Medium |

### Low Priority Issues

| ID     | Issue                                                                               | Location | Severity |
| ------ | ----------------------------------------------------------------------------------- | -------- | -------- |
| NDP-12 | **Entity name could be larger** - "Nemotron 3" should be more prominent as h1       | Header   | 🟢 Low   |
| NDP-13 | **Dot indicator color not explained** - Blue dot before entity name meaning unclear | Header   | 🟢 Low   |

---

## Improvement Plan

### Phase 1: Critical Fixes (Week 1)

#### 1.1 Expandable Property Values

```
Current:  source_ids  c52657ee-fd49-473...
Improved: source_ids  c52657ee-fd49-4737-89de-6f473786edea [📋]
                      ↳ Click to expand/collapse long values
```

**Implementation:**

- Add "show full" toggle for truncated values
- Add copy button on hover for each property value
- Consider tooltip on hover showing full value

#### 1.2 Copy All Functionality

- Add "Copy All Properties" button in Properties section header
- Individual copy buttons for each UUID field
- Toast notification on successful copy

### Phase 2: Visual Consistency (Week 2)

#### 2.1 Section Spacing Standardization

```css
.section-header {
  margin-top: 24px; /* --space-6 */
  margin-bottom: 12px; /* --space-3 */
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.section-content {
  padding: 16px; /* --space-4 */
  gap: 12px; /* --space-3 */
}
```

#### 2.2 Entity Type Badge Enhancement

```
Current:  [PRODUCT] (small, minimal padding)
Improved: [● PRODUCT] (pill badge with dot, px-3 py-1.5)
```

**Specifications:**

- Background: `bg-primary/10`
- Text: `text-primary font-medium`
- Padding: `px-3 py-1.5`
- Border radius: `rounded-full`

#### 2.3 Relationship Direction Clarity

```
Current:  ← ● Mixture-of-Experts (MoE)
Improved: ⟵ INCOMING  ● Mixture-of-Experts (MoE)
          ⟶ OUTGOING  ● Qwen3-30B
```

**Or use labels:**

- "Related FROM" (incoming)
- "Related TO" (outgoing)

### Phase 3: Property Display Improvements (Week 2)

#### 3.1 Property Grid Layout

```
┌─────────────────────────────────────────────────┐
│ ✦ PROPERTIES                                    │
├─────────────────────────────────────────────────┤
│ description    AI model architecture designed  │
│                by NVIDIA to improve efficiency │
│                in handling long-context tasks. │
│                                                 │
│ entity_type    PRODUCT                    [📋] │
│ importance     0.5          ████░░░░░░   [📋] │
│ source_ids     c52657ee-... ⤢ expand     [📋] │
│ tenant_id      7f8d921e-... ⤢ expand     [📋] │
│ workspace_id   d3d520d1-... ⤢ expand     [📋] │
└─────────────────────────────────────────────────┘
```

**Features:**

- Fixed-width labels (120px)
- Right-aligned values
- Copy button on each row
- Expand toggle for UUIDs
- Progress bar for numeric values like "importance"

### Phase 4: Footer Actions (Week 3)

#### 4.1 Action Button Improvements

```
Current:  [✎ Edit] [⚗ Merge] [🗑 Delete]
Improved:
┌─────────────────────────────────────────────────┐
│   [✎ Edit]   [⚗ Merge]   │   [🗑 Delete]       │
│   ─────────────────────────────────────────────  │
│   Primary actions         │   Destructive        │
└─────────────────────────────────────────────────┘
```

**Specifications:**

- Separate destructive action with divider or gap
- Delete button: `variant="destructive"` (red background)
- Add confirmation dialog for Delete
- Buttons: `min-h-[44px]` for touch targets

---

## Design Tokens Required

```css
/* Node Details Panel */
--panel-header-padding: 20px;
--section-gap: 24px;
--property-label-width: 120px;
--property-row-gap: 8px;
--badge-padding-x: 12px;
--badge-padding-y: 6px;
```

---

## Accessibility Improvements

1. **Screen Reader Support:**

   - Add `aria-label="Node details for Nemotron 3"` to panel
   - Section headings should be `role="heading" aria-level="2"`

2. **Keyboard Navigation:**

   - Tab through all interactive elements
   - Enter/Space to expand truncated values
   - Focus visible states on all buttons

3. **Color Contrast:**
   - Ensure muted text meets 4.5:1 ratio
   - Relationship arrow colors need higher contrast

---

## Success Metrics

| Metric                       | Current     | Target               |
| ---------------------------- | ----------- | -------------------- |
| Property value accessibility | 60% visible | 100% accessible      |
| Copy action discoverability  | Low         | High (visible icons) |
| Section spacing consistency  | Varies      | Uniform 24px         |
| Touch target compliance      | ~36px       | 44px minimum         |
