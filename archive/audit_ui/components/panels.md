# Panel Architecture Audit

**Components Reviewed:**

- Right Panel (`src/components/layout/right-panel.tsx`)
- Entity Browser Panel (Graph screen)
- Conversation History Panel (Query screen)
- Mobile Sheet Panels
- Collapsible Panels

**Cross-cutting Concerns:** Collapsibility, Persistence, Animations, Sizing, Mobile Adaptation

---

## Panel Inventory

| Panel                | Screen    | Position | Collapsible | Persistent | Mobile |
| -------------------- | --------- | -------- | ----------- | ---------- | ------ |
| Right Panel          | Multiple  | Right    | ✅          | ✅         | Sheet  |
| Entity Browser       | Graph     | Left     | ✅          | ⚠️         | Hidden |
| Conversation History | Query     | Left     | ❌          | N/A        | Hidden |
| Document Details     | Documents | Right    | ✅          | ✅         | Sheet  |

---

## Panel Structure Analysis

### Standard Right Panel Pattern

```
┌─────────────────────────────────────┐
│ Panel Header           [─] [×]      │  ← 48-56px height
├─────────────────────────────────────┤
│                                     │
│ Panel Content                       │  ← flex-1, overflow-auto
│ (scrollable)                        │
│                                     │
├─────────────────────────────────────┤
│ Panel Footer (optional)             │  ← 48-56px height
└─────────────────────────────────────┘
```

---

## Slickness Score

| Criterion          | Score (1–5) | Notes                                  |
| ------------------ | ----------- | -------------------------------------- |
| Visual consistency | 3.8         | Some variation in header styles        |
| Animation quality  | 4.0         | Smooth collapse, could use spring      |
| State persistence  | 3.5         | Some panels forget state               |
| Mobile adaptation  | 4.0         | Sheet pattern works well               |
| **Overall**        | **3.8**     | Good foundation, needs standardization |

---

## Issues

### 🟠 Major

#### Inconsistent Panel Header Heights

- **Severity:** 🟠 Major
- **Location:** All panels
- **Current behavior:** Headers vary between 48px, 56px, 64px
- **Expected behavior:** Standardize at 48px or 56px

#### Panel State Not Persisted

- **Severity:** 🟠 Major
- **Location:** Entity Browser, Right Panel
- **Current behavior:** Collapse state may reset on navigation
- **Expected behavior:** Remember open/closed state per panel per screen

#### No Panel Resize

- **Severity:** 🟠 Major
- **Location:** All fixed-width panels
- **Current behavior:** Fixed widths (256px, 320px, 400px)
- **Expected behavior:** Draggable resize with min/max constraints

---

### 🟡 Minor

#### Collapse Animation Timing

- **Severity:** 🟡 Minor
- **Location:** All collapsible panels
- **Current behavior:** Linear or basic ease
- **Expected behavior:** Consistent spring animation (ease-out-cubic)

#### Close Button Inconsistent

- **Severity:** 🟡 Minor
- **Location:** Panel headers
- **Current behavior:** Some have [×], some have [−]
- **Expected behavior:** Collapse vs Close should be visually distinct

#### Panel Shadow on Light Theme

- **Severity:** 🟡 Minor
- **Location:** Right panels
- **Current behavior:** May lack shadow separation
- **Expected behavior:** Subtle shadow for depth

---

## Recommendations

### 1. Create Unified Panel Component

**Change:** Extract reusable Panel component

**Specifications:**

```tsx
// src/components/ui/panel.tsx
interface PanelProps {
  id: string;
  title: string;
  icon?: React.ReactNode;
  position: "left" | "right";
  defaultWidth?: number;
  minWidth?: number;
  maxWidth?: number;
  collapsible?: boolean;
  defaultCollapsed?: boolean;
  persistState?: boolean;
  onCollapse?: (collapsed: boolean) => void;
  children: React.ReactNode;
  footer?: React.ReactNode;
}

export function Panel({
  id,
  title,
  icon,
  position,
  defaultWidth = 320,
  minWidth = 200,
  maxWidth = 600,
  collapsible = true,
  defaultCollapsed = false,
  persistState = true,
  onCollapse,
  children,
  footer,
}: PanelProps) {
  const [isCollapsed, setIsCollapsed] = usePanelState(id, defaultCollapsed);
  const [width, setWidth] = usePanelWidth(id, defaultWidth);

  return (
    <motion.aside
      className={cn(
        "flex flex-col border-l bg-background",
        position === "left" && "border-r border-l-0"
      )}
      animate={{ width: isCollapsed ? 0 : width }}
      transition={{ type: "spring", stiffness: 300, damping: 30 }}
    >
      {/* Header */}
      <div className="flex items-center h-12 px-4 border-b shrink-0">
        {icon && <span className="mr-2">{icon}</span>}
        <h3 className="font-medium text-sm flex-1 truncate">{title}</h3>
        {collapsible && (
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setIsCollapsed(!isCollapsed)}
            aria-label={isCollapsed ? "Expand panel" : "Collapse panel"}
          >
            {isCollapsed ? <ChevronRight /> : <ChevronLeft />}
          </Button>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">{children}</div>

      {/* Footer */}
      {footer && <div className="shrink-0 border-t p-3">{footer}</div>}

      {/* Resize Handle */}
      {!isCollapsed && (
        <ResizeHandle
          onResize={setWidth}
          minWidth={minWidth}
          maxWidth={maxWidth}
          position={position}
        />
      )}
    </motion.aside>
  );
}
```

**Acceptance Criteria:**

- [ ] Single component for all panels
- [ ] Consistent header height (48px)
- [ ] State persistence in localStorage
- [ ] Resize capability
- [ ] Mobile sheet fallback

---

### 2. Implement Panel State Persistence

**Change:** Use Zustand store for panel states

**Specifications:**

```tsx
// src/stores/panel-store.ts
interface PanelState {
  panels: Record<
    string,
    {
      collapsed: boolean;
      width: number;
    }
  >;
  setPanelCollapsed: (id: string, collapsed: boolean) => void;
  setPanelWidth: (id: string, width: number) => void;
}

export const usePanelStore = create<PanelState>()(
  persist(
    (set) => ({
      panels: {},
      setPanelCollapsed: (id, collapsed) =>
        set((state) => ({
          panels: {
            ...state.panels,
            [id]: { ...state.panels[id], collapsed },
          },
        })),
      setPanelWidth: (id, width) =>
        set((state) => ({
          panels: {
            ...state.panels,
            [id]: { ...state.panels[id], width },
          },
        })),
    }),
    {
      name: "panel-state",
    }
  )
);

// Custom hooks
export function usePanelState(id: string, defaultCollapsed: boolean) {
  const panel = usePanelStore((s) => s.panels[id]);
  const setCollapsed = usePanelStore((s) => s.setPanelCollapsed);

  return [
    panel?.collapsed ?? defaultCollapsed,
    (collapsed: boolean) => setCollapsed(id, collapsed),
  ] as const;
}
```

**Acceptance Criteria:**

- [ ] Panel state persists across sessions
- [ ] Each panel has unique ID
- [ ] Width and collapse state saved
- [ ] Works with SSR (next-safe)

---

### 3. Add Resize Handle Component

**Change:** Draggable resize for panels

**Specifications:**

```tsx
// src/components/ui/resize-handle.tsx
interface ResizeHandleProps {
  onResize: (width: number) => void;
  minWidth: number;
  maxWidth: number;
  position: "left" | "right";
}

export function ResizeHandle({
  onResize,
  minWidth,
  maxWidth,
  position,
}: ResizeHandleProps) {
  const [isDragging, setIsDragging] = useState(false);

  return (
    <div
      className={cn(
        "absolute top-0 bottom-0 w-1 cursor-col-resize",
        "hover:bg-primary/20 active:bg-primary/30",
        "transition-colors duration-150",
        position === "left" ? "right-0" : "left-0",
        isDragging && "bg-primary/30"
      )}
      onMouseDown={handleMouseDown}
    />
  );
}
```

**Acceptance Criteria:**

- [ ] Visual indicator on hover
- [ ] Smooth drag experience
- [ ] Min/max constraints enforced
- [ ] Cursor changes on hover

---

### 4. Standardize Mobile Sheet Pattern

**Change:** Consistent sheet for all panels on mobile

**Specifications:**

```tsx
// src/components/ui/responsive-panel.tsx
interface ResponsivePanelProps extends PanelProps {
  mobileBreakpoint?: number; // default 768
  mobileTitle?: string;
  mobileTrigger?: React.ReactNode;
}

export function ResponsivePanel({
  mobileBreakpoint = 768,
  mobileTitle,
  mobileTrigger,
  ...props
}: ResponsivePanelProps) {
  const isMobile = useMediaQuery(`(max-width: ${mobileBreakpoint}px)`);

  if (isMobile) {
    return (
      <Sheet>
        <SheetTrigger asChild>{mobileTrigger}</SheetTrigger>
        <SheetContent side={props.position === "left" ? "left" : "right"}>
          <SheetHeader>
            <SheetTitle>{mobileTitle || props.title}</SheetTitle>
          </SheetHeader>
          <div className="mt-4 overflow-auto flex-1">{props.children}</div>
          {props.footer && (
            <div className="mt-auto pt-4 border-t">{props.footer}</div>
          )}
        </SheetContent>
      </Sheet>
    );
  }

  return <Panel {...props} />;
}
```

**Acceptance Criteria:**

- [ ] Sheet on mobile < 768px
- [ ] Proper accessibility (SheetTitle present)
- [ ] Same content renders in both modes
- [ ] Trigger button on mobile

---

## Panel Specifications

### Dimensions

| Property       | Value | Notes                   |
| -------------- | ----- | ----------------------- |
| Header height  | 48px  | Consistent across all   |
| Min width      | 200px | Collapsed shows nothing |
| Default width  | 320px | Good for most content   |
| Max width      | 600px | Prevent over-expansion  |
| Collapse width | 0px   | Fully hidden            |
| Footer height  | 48px  | When present            |

### Animation

| Property         | Value                        |
| ---------------- | ---------------------------- |
| Duration         | 250ms                        |
| Easing           | cubic-bezier(0.4, 0, 0.2, 1) |
| Spring stiffness | 300                          |
| Spring damping   | 30                           |

### Colors

| Element        | Token                       |
| -------------- | --------------------------- |
| Background     | `--background`              |
| Border         | `--border`                  |
| Header text    | `--foreground`              |
| Shadow (light) | `0 0 20px rgba(0,0,0,0.05)` |
| Shadow (dark)  | `0 0 20px rgba(0,0,0,0.2)`  |

---

## Panel Usage by Screen

### Dashboard

- No panels (single content area)

### Documents

- Right Panel: Document details
  - Width: 400px
  - Collapsible: Yes
  - Content: Document info, actions

### Query

- Left Panel (optional): Conversation history
  - Width: 280px
  - Collapsible: Could be
- Right Panel (potential): Related documents
  - Not implemented

### Graph

- Left Panel: Entity browser
  - Width: 256px
  - Collapsible: Yes
  - Content: Entity list, search, filters
- Right Panel (potential): Entity details
  - Triggered by selection

### Settings

- No panels (tabs in main content)

### API Explorer

- Two-column layout (not panels)

---

## Accessibility Requirements

| Requirement                        | Status            |
| ---------------------------------- | ----------------- |
| `aria-expanded` on collapse toggle | ⚠️ Check          |
| `aria-label` on buttons            | ⚠️ Check          |
| Focus trap in mobile sheets        | ✅ shadcn handles |
| Keyboard close (Escape)            | ✅ shadcn handles |
| Screen reader announcement         | ⚠️ Add aria-live  |

---

## Implementation Priority

1. **High:** Create unified Panel component
2. **High:** Implement state persistence store
3. **Medium:** Add resize handles
4. **Medium:** Standardize mobile sheets
5. **Low:** Add spring animations

---

_Last updated: December 25, 2025_
