# Dashboard/Home Screen Audit

## Screen: Dashboard (`/`)

**Screenshot References:**

- [`01-dashboard-full.png`](../audit_ui/screenshots/01-dashboard-full.png)
- [`01-dashboard-viewport.png`](../audit_ui/screenshots/01-dashboard-viewport.png)

**Component Files:**

- Layout: [`src/app/(dashboard)/layout.tsx`](<../edgequake_webui/src/app/(dashboard)/layout.tsx>)
- Page: [`src/app/(dashboard)/page.tsx`](<../edgequake_webui/src/app/(dashboard)/page.tsx>)
- Components: [`src/components/dashboard/`](../edgequake_webui/src/components/dashboard/)

---

## What I Reviewed

### UI Regions Analyzed:

1. **Sidebar** (left panel) - Primary navigation with tenant/workspace selector
2. **Header** - Connection status, theme switcher, user menu, mobile navigation
3. **Breadcrumb** - Navigation context below header
4. **Main Content Area** - Dashboard widgets including:
   - Page title and description
   - Stats cards (4-column grid)
   - Quick Actions section
   - Recent Activity section (2/3 width)
   - System Status panel (1/3 width)

### Measurements:

- Main content area: **1664px × 999px** (at 1920px viewport)
- Sidebar: **256px** width (expanded)
- Header: **64px** height
- Breadcrumb bar: ~**40px** height

---

## Issues

### 🔴 Critical

**C1. No Collapsible Left Panel**

- **Location:** Sidebar component ([`sidebar.tsx`](../edgequake_webui/src/components/layout/sidebar.tsx))
- **Issue:** Desktop sidebar has collapse functionality in code (`collapsed` prop, toggle button) but is not being used or exposed in the main layout
- **Evidence:** `showToggle` prop is not passed as `true` in the desktop implementation
- **Impact:** Wastes horizontal space for users who want more content area, especially on smaller screens (1366px, 1440px laptops)

**C2. No Right Panel at All on Dashboard**

- **Location:** Dashboard page layout
- **Issue:** Unlike the query page which has a right panel for context, the dashboard has no collapsible right panel for additional information or filters
- **Impact:** Inconsistent layout pattern across the application; missed opportunity for contextual help or advanced filters

### 🟡 Major

**M1. Weak Visual Hierarchy in Page Title**

- **Location:** Dashboard page header
- **Current:** `text-2xl font-bold` (24px) for "Dashboard" title
- **Issue:** Not sufficiently differentiated from card titles and other headings; feels small in the large content area
- **Evidence:** H1 uses same `text-2xl` as some h2/h3 elements

**M2. Inconsistent Spacing Between Sections**

- **Location:** Main content area (`space-y-6` = 24px)
- **Issue:** All sections use uniform 24px spacing, which doesn't create enough visual separation between functionally different areas (stats vs actions vs activity)
- **Visual hierarchy suffers** - everything feels equally important

**M3. Stats Cards Typography Too Generic**

- **Location:** StatsCard components
- **Issue:**
  - Value numbers don't stand out enough (should be larger, bolder)
  - Description text is same size/weight as title
  - Poor scanning order - eye doesn't naturally go title → value → description

**M4. Recent Activity Table Density Issues**

- **Location:** Recent Activity component
- **Issue:**
  - Too much whitespace in some columns, not enough in others
  - No hover states or visual feedback
  - Column widths not optimized (filename takes too much space)

**M5. Breadcrumb Bar Wastes Vertical Space**

- **Location:** Below header (`py-2` + `border-b`)
- **Issue:** Takes up a full row with border just for breadcrumbs when there's only 1-2 items
- **Evidence:** ~40px of vertical space that could be reclaimed or better utilized
- **Alternative:** Could be integrated into header or removed on simple pages

### 🟢 Minor

**m1. No Visual Loading States for Stats**

- **Location:** StatsCard components
- **Issue:** While `isLoading` prop exists, the skeleton/loading state is not visually distinct enough
- **Expected:** Shimmer effect or clear skeleton UI

**m2. Quick Actions Cards Lack Visual Interest**

- **Location:** QuickActions component
- **Issue:** Plain cards with icons that don't leverage color or visual design
- **Opportunity:** Use accent colors, gradients, or better iconography to make actions more inviting

**m3. System Status Panel Underutilized**

- **Location:** Right column (1/3 width)
- **Issue:** Takes up significant space but only shows basic connection status
- **Opportunity:** Could show more metrics (query latency, storage usage, recent errors, LLM stats)

**m4. Mobile Header Redundant**

- **Location:** Header component
- **Issue:** Shows "EdgeQuake" text on mobile when mobile sidebar already shows the logo
- **Code:** `<h1 className="text-lg font-semibold md:hidden">EdgeQuake</h1>`

**m5. Tenant/Workspace Selector Cramped When Collapsed**

- **Location:** Sidebar tenant selector
- **Issue:** Compact version not well optimized for collapsed sidebar
- **Evidence:** `compact={true}` prop but implementation may truncate important info

---

## Recommendations

### For Left Panel (Sidebar)

**R1. Enable Desktop Sidebar Collapse** ⭐ **PRIORITY**

```tsx
// In layout.tsx, add state management
const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

<Sidebar
  collapsed={sidebarCollapsed}
  onToggle={() => setSidebarCollapsed(!sidebarCollapsed)}
/>;
```

- **Collapsed width:** 72px (fits icon + padding)
- **Expanded width:** 256px (current)
- **Transition:** 200ms ease-in-out
- **Persistence:** Save state to localStorage

**R2. Default State Strategy**

- **≥ 1440px viewport:** Expanded by default
- **1024px - 1439px:** Collapsed by default
- **< 1024px:** Mobile drawer (current behavior)

**R3. Collapsed Sidebar Improvements**

- Show tooltips on hover for all nav items (already implemented ✓)
- Tenant/workspace selector should show just an icon/avatar in collapsed state
- Keep version info hidden when collapsed

### For Right Panel (Dashboard)

**R4. Add Optional Right Panel for Insights** ⭐ **PRIORITY**

```
┌─────────────┬──────────────────────┬──────────┐
│   Sidebar   │    Main Content      │  Right   │
│   (256px)   │     (flexible)       │ (320px)  │
└─────────────┴──────────────────────┴──────────┘
```

**Panel Width:** 320px (collapsible)
**Default State:** Collapsed on dashboard (optional feature)
**Content Ideas:**

- **Quick Filters:** Filter stats by date range
- **Recent Queries:** Last 5-10 queries run
- **System Health:** Detailed metrics
- **Tips & Onboarding:** Contextual help for new users
- **Activity Feed:** Real-time updates

**R5. Make Right Panel Consistent Across All Pages**

- Query page: Context sources (keep current)
- Documents page: Document preview/metadata
- Graph page: Node details/filters
- Settings page: Help documentation

### For Visual Hierarchy

**R6. Strengthen Typography System** ⭐ **PRIORITY**

**Page Titles (H1):**

```css
/* Current: text-2xl (24px) */
/* Recommended: text-3xl font-bold tracking-tight (30px) */
```

**Section Titles (H2):**

```css
/* Recommended: text-xl font-semibold (20px) */
```

**Card Titles (H3):**

```css
/* Recommended: text-base font-medium (16px) */
```

**R7. Improve Stats Card Visual Design**

```tsx
// Enhanced Stats Card Structure:
<Card>
  <CardHeader className="pb-2">
    <div className="flex items-center justify-between">
      <CardTitle className="text-sm font-medium text-muted-foreground">
        {title}
      </CardTitle>
      <Icon className="h-4 w-4 text-muted-foreground" />
    </div>
  </CardHeader>
  <CardContent>
    <div className="text-4xl font-bold tabular-nums">{value}</div>
    <p className="text-xs text-muted-foreground mt-1">{description}</p>
  </CardContent>
</Card>
```

**Key Changes:**

- Value: `text-4xl font-bold tabular-nums` (36px, monospace numbers)
- Title: `text-sm` to de-emphasize (12px)
- Description: `text-xs` (11px)
- Add icon at top-right for visual interest

### For Space Optimization

**R8. Implement Progressive Spacing Scale**

```tsx
// Replace uniform space-y-6 with:
<div className="p-6 space-y-8">
  {" "}
  {/* Container */}
  <div>{/* Header - title + description */}</div>
  <div className="space-y-4">
    {" "}
    {/* Stats section */}
    {/* Stats cards with gap-4 */}
  </div>
  <div className="mt-8">
    {" "}
    {/* Quick Actions - larger gap */}
    {/* Action cards */}
  </div>
  <div className="mt-8 space-y-6">
    {" "}
    {/* Activity section */}
    {/* Recent Activity + System Status */}
  </div>
</div>
```

**Spacing Scale:**

- Related items (within section): **16px** (gap-4)
- Between sections: **32px** (mt-8)
- Container padding: **24px** (p-6)

**R9. Optimize Breadcrumb Integration**

- **Option A:** Integrate into header (single 64px bar)
- **Option B:** Remove breadcrumb bar on dashboard (it's just "Dashboard")
- **Option C:** Only show breadcrumb when depth > 1 (e.g., Dashboard > Settings > API)

**R10. Improve Table Density**

```tsx
// Recent Activity Table
<Table>
  <TableHeader>
    <TableRow>
      <TableHead className="w-[40%]">Document</TableHead>
      <TableHead className="w-[20%]">Status</TableHead>
      <TableHead className="w-[20%]">Date</TableHead>
      <TableHead className="w-[20%] text-right">Actions</TableHead>
    </TableRow>
  </TableHeader>
</Table>
```

- Add `hover:bg-muted/50` to rows
- Add fixed column widths
- Truncate long filenames with tooltip
- Add subtle border-spacing

---

## Rationale

### Why Collapsible Panels Matter

- **User Control:** Power users want to maximize content area; new users may want persistent navigation
- **Modern Standard:** Gmail, Notion, Linear, GitHub - all have collapsible sidebars
- **Accessibility:** More content visible means less scrolling for screen reader users
- **Responsive:** Helps bridge the gap between desktop and tablet layouts

### Why Visual Hierarchy Matters

- **Cognitive Load:** Clear hierarchy helps users process information 50% faster
- **Scanning:** Users don't read, they scan - hierarchy guides their eyes
- **Professionalism:** Weak hierarchy looks amateurish and hurts brand perception

### Why Consistent Spacing Matters

- **Rhythm:** Creates visual rhythm that feels natural and reduces cognitive friction
- **Grouping:** Proximity indicates relationships between UI elements (Gestalt principles)
- **Breathing Room:** Prevents information overload while maintaining density

---

## Acceptance Criteria

### AC1: Collapsible Left Sidebar (Desktop)

- [ ] Sidebar can be toggled via button (ChevronLeft/Right icon)
- [ ] Collapsed width is 72px, expanded is 256px
- [ ] Smooth transition animation (200ms)
- [ ] State persists across page navigation (localStorage)
- [ ] Tooltips show on hover when collapsed
- [ ] Keyboard accessible (Ctrl+B or Cmd+B to toggle)

### AC2: Optional Right Panel (Dashboard)

- [ ] Right panel can be toggled on dashboard
- [ ] Default state is collapsed (can be made visible)
- [ ] Width is 320px when expanded
- [ ] Shows contextual information (insights, tips, or filters)
- [ ] Smooth transition animation (200ms)
- [ ] State persists (localStorage)

### AC3: Improved Visual Hierarchy

- [ ] Page titles use `text-3xl font-bold` (30px)
- [ ] Stats cards use `text-4xl font-bold tabular-nums` for values
- [ ] Clear distinction between H1, H2, H3 sizing
- [ ] Proper text color contrast: titles use foreground, descriptions use muted-foreground

### AC4: Spacing Improvements

- [ ] Progressive spacing implemented (16px within sections, 32px between sections)
- [ ] Breadcrumb integrated into header OR removed on simple pages
- [ ] No uniform spacing - each section uses appropriate gaps

### AC5: Stats Card Redesign

- [ ] Values are 36px, bold, tabular-nums
- [ ] Icons positioned top-right
- [ ] Loading states use skeleton UI with shimmer
- [ ] Cards have subtle hover effect

### AC6: Table Improvements

- [ ] Column widths explicitly set
- [ ] Row hover states implemented
- [ ] Long filenames truncated with tooltip
- [ ] Actions column right-aligned

---

## ASCII Layout Diagram - Recommended

### Desktop (≥1440px) - Sidebars Expanded

```
┌──────────────┬────────────────────────────────────────────────┬──────────────┐
│              │ Header (Connection • Theme • User)              │              │
│              ├────────────────────────────────────────────────┤              │
│              │ Breadcrumb (optional, integrated in header)    │              │
│   Sidebar    ├────────────────────────────────────────────────┤   Right      │
│   (256px)    │                                                │   Panel      │
│              │              Main Content                       │   (320px)    │
│  • Nav       │  ┌──────────────────────────────────────────┐ │              │
│  • Tenant    │  │ Page Title (text-3xl)                    │ │  • Insights  │
│  • Version   │  ├──────────────────────────────────────────┤ │  • Filters   │
│              │  │ Stats Cards (4-col, gap-4)               │ │  • Activity  │
│  [Collapse]  │  ├──────────────────────────────────────────┤ │              │
│              │  │ Quick Actions (gap-4)                    │ │  [Collapse]  │
│              │  ├──────────────────────────────────────────┤ │              │
│              │  │ Recent Activity (2/3) │ System Status    │ │              │
│              │  └──────────────────────────────────────────┘ │              │
└──────────────┴────────────────────────────────────────────────┴──────────────┘
     256px                    ~1344px                              320px
```

### Desktop (≥1440px) - Sidebars Collapsed

```
┌───┬──────────────────────────────────────────────────────────────────┬───┐
│   │ Header (Connection • Theme • User)                                │   │
│   ├──────────────────────────────────────────────────────────────────┤   │
│ S │                      Main Content (expanded)                      │ R │
│ B │  ┌────────────────────────────────────────────────────────────┐  │ P │
│   │  │ Page Title (text-3xl)                                       │  │   │
│ 72│  ├────────────────────────────────────────────────────────────┤  │72 │
│ px│  │ Stats Cards (4-col, gap-4) - WIDER                         │  │px │
│   │  ├────────────────────────────────────────────────────────────┤  │   │
│   │  │ Quick Actions                                              │  │   │
│   │  ├────────────────────────────────────────────────────────────┤  │   │
│   │  │ Recent Activity │ System Status                            │  │   │
│   │  └────────────────────────────────────────────────────────────┘  │   │
└───┴──────────────────────────────────────────────────────────────────┴───┘
 72px                          ~1776px                                 72px
```

### Laptop (1024px - 1439px) - Sidebar Collapsed, No Right Panel

```
┌───┬────────────────────────────────────────────────────────────┐
│   │ Header                                                      │
│   ├────────────────────────────────────────────────────────────┤
│ S │              Main Content                                   │
│ B │  ┌──────────────────────────────────────────────────────┐  │
│   │  │ Page Title                                            │  │
│ 72│  ├──────────────────────────────────────────────────────┤  │
│ px│  │ Stats Cards (2-col on smaller screens)               │  │
│   │  ├──────────────────────────────────────────────────────┤  │
│   │  │ Quick Actions (2-col)                                │  │
│   │  ├──────────────────────────────────────────────────────┤  │
│   │  │ Recent Activity (full width)                         │  │
│   │  │ System Status (full width, below)                    │  │
│   │  └──────────────────────────────────────────────────────┘  │
└───┴────────────────────────────────────────────────────────────┘
```

---

## Related Files & Components

### Components to Modify:

- ✏️ [`src/components/layout/sidebar.tsx`](../edgequake_webui/src/components/layout/sidebar.tsx) - Add desktop collapse
- ✏️ [`src/app/(dashboard)/layout.tsx`](<../edgequake_webui/src/app/(dashboard)/layout.tsx>) - Add sidebar state management
- ✏️ [`src/components/dashboard/stats-card.tsx`](../edgequake_webui/src/components/dashboard/stats-card.tsx) - Improve typography
- ✏️ [`src/components/dashboard/recent-activity.tsx`](../edgequake_webui/src/components/dashboard/recent-activity.tsx) - Table density

### New Components to Create:

- 🆕 `src/components/layout/right-panel.tsx` - Reusable right panel component
- 🆕 `src/components/dashboard/insights-panel.tsx` - Dashboard right panel content

### Styles to Update:

- ✏️ [`src/app/globals.css`](../edgequake_webui/src/app/globals.css) - Add spacing scale tokens
- ✏️ Add typography scale variables

---

## Priority Summary

**🔥 Must Do (Quick Wins):**

1. ✅ Enable collapsible left sidebar (R1) - 2-3 hours
2. ✅ Fix typography hierarchy (R6, R7) - 1-2 hours
3. ✅ Improve spacing system (R8) - 1 hour
4. ✅ Optimize breadcrumb (R9) - 30 minutes

**📌 Should Do (Next Sprint):** 5. Add optional right panel structure (R4) - 3-4 hours 6. Improve table density (R10) - 1-2 hours 7. Better loading states (m1) - 1 hour

**💡 Nice to Have (Later):** 8. Quick Actions visual redesign (m2) 9. Enhanced System Status panel (m3) 10. Mobile header cleanup (m4)
