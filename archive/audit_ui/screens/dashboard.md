# Dashboard Screen Audit

**Route:** `/`  
**Viewport(s) Tested:** 320px, 428px, 768px, 1280px, 1536px  
**UI Regions:** Header, Sidebar, Main Content, Stats Cards, Quick Actions, Recent Activity, System Status  
**States Captured:** Default, Sidebar Collapsed, Mobile Menu Open  
**Screenshots:** `screenshots/screens/dashboard/`  
**Relevant Files:** `src/app/(dashboard)/page.tsx`, `src/components/dashboard/`

---

## What I Reviewed

### Layout Structure

```
┌─────────────────────────────────────────────────────────┐
│ Header (fixed, h: 64px)                    │ API │ 🌐 ☀️ 👤│
├────────────┬────────────────────────────────────────────┤
│ Sidebar    │ Main Content                               │
│ (collapsible)│                                          │
│ w: 256px   │ ┌──────────────────────────────────────┐  │
│            │ │ Page Title & Description             │  │
│ Logo       │ ├──────────────────────────────────────┤  │
│ Tenant     │ │ Stats Cards (4-col grid)             │  │
│ Workspace  │ │ ┌────┐ ┌────┐ ┌────┐ ┌────┐         │  │
│            │ │ │Docs│ │Ents│ │Rels│ │Types│        │  │
│ Navigation │ │ └────┘ └────┘ └────┘ └────┘         │  │
│ - Dashboard│ ├──────────────────────────────────────┤  │
│ - Graph    │ │ Quick Actions (3-col cards)          │  │
│ - Documents│ ├──────────────────────────────────────┤  │
│ - Query    │ │ ┌─────────────────┐ ┌───────────┐   │  │
│ - API      │ │ │ Recent Activity │ │ System    │   │  │
│ - Settings │ │ │ (2/3 width)     │ │ Status    │   │  │
│            │ │ └─────────────────┘ └───────────┘   │  │
│ Collapse ◀ │ └──────────────────────────────────────┘  │
│ EdgeQuake  │                                            │
│ v0.1.0     │                                            │
└────────────┴────────────────────────────────────────────┘
```

---

## Slickness Score

| Criterion           | Score (1–5) | Notes                                             |
| ------------------- | ----------- | ------------------------------------------------- |
| Visual refinement   | 4.0         | Clean cards, subtle gradients on Quick Actions    |
| Modern styling      | 4.2         | Good use of colored card accents                  |
| Smooth interactions | 3.5         | Sidebar collapse is smooth; no card hover effects |
| Professional polish | 4.0         | Good empty states, but "0" values feel static     |
| **Overall**         | **3.9**     | Solid foundation, needs micro-interactions        |

---

## Issues

### 🔴 Critical

#### Missing Breadcrumb Navigation

- **Severity:** 🔴 Critical
- **Location:** Main content area, below header
- **Viewport(s) affected:** All
- **Current behavior:** Dashboard has no breadcrumb; other pages do
- **Expected behavior:** Consistent navigation pattern across all pages
- **Impact:** Users may feel disoriented; inconsistent UX pattern

---

### 🟠 Major

#### Dual H1 Heading Tags

- **Severity:** 🟠 Major
- **Location:** Header (mobile) + Page title
- **Viewport(s) affected:** Mobile (two H1s visible)
- **Current behavior:**
  - Mobile header has `<h1 className="text-lg">EdgeQuake</h1>`
  - Page title is also H1: `<h1>Tableau de bord</h1>`
- **Expected behavior:** Single H1 per page for accessibility and SEO
- **Code location:** `src/components/layout/header.tsx:71`

#### Stats Cards Show "0" During Load

- **Severity:** 🟠 Major
- **Location:** Stats Cards section
- **Viewport(s) affected:** All
- **Current behavior:** Shows "0" while loading, then updates
- **Expected behavior:** Show skeleton or animated placeholder
- **Screenshot:** Shows static "0" before data loads

---

### 🟡 Minor

#### No Hover Effects on Quick Action Cards

- **Severity:** 🟡 Minor
- **Location:** Quick Actions section
- **Viewport(s) affected:** Desktop
- **Current behavior:** Cards have colored backgrounds but no hover feedback
- **Expected behavior:** Subtle scale or shadow increase on hover

#### System Status Card Has No Skeleton Loading

- **Severity:** 🟡 Minor
- **Location:** System Status panel
- **Viewport(s) affected:** All
- **Current behavior:** Shows "Unavailable" for LLM before connection check
- **Expected behavior:** Show loading indicator during connection check

#### Recent Activity Empty State Could Be More Engaging

- **Severity:** 🟡 Minor
- **Location:** Recent Activity panel
- **Viewport(s) affected:** All
- **Current behavior:** Icon + text + link
- **Expected behavior:** Add subtle illustration or animated empty state

---

## Recommendations

### 1. Add Breadcrumb to Dashboard

**Change:** Add breadcrumb navigation to dashboard for consistency

**Specifications:**

- Show "EdgeQuake > Dashboard" on dashboard
- Use same component as other pages
- Height: 48px with border-bottom

**Applies to:** Dashboard page only

**Code hint:** Already using `DynamicBreadcrumb` component, ensure it renders on `/`

**Acceptance Criteria:**

- [ ] Breadcrumb displays "EdgeQuake > Dashboard"
- [ ] Matches styling of other page breadcrumbs
- [ ] Navigation works correctly

---

### 2. Fix Dual H1 Issue

**Change:** Change mobile branding to non-heading element

**Specifications:**

```tsx
// Before (header.tsx:71)
<h1 className="text-lg font-semibold md:hidden">EdgeQuake</h1>

// After
<span className="text-lg font-semibold md:hidden" aria-hidden="true">EdgeQuake</span>
```

**Applies to:** All pages

**Acceptance Criteria:**

- [ ] Only one H1 per page
- [ ] Mobile branding still visible
- [ ] Screen readers only announce main page title as H1

---

### 3. Add Skeleton Loading to Stats Cards

**Change:** Show animated skeleton while stats are loading

**Specifications:**

```tsx
// Stats card loading state
{
  isLoading ? (
    <div className="animate-pulse">
      <div className="h-8 w-16 bg-muted rounded" />
    </div>
  ) : (
    <span className="text-2xl font-bold">{value}</span>
  );
}
```

**Animation:**

- Duration: 1.5s
- Easing: ease-in-out
- Pattern: Shimmer effect

**Applies to:** All 4 stats cards

**Acceptance Criteria:**

- [ ] Skeleton shows during initial load
- [ ] Smooth transition to actual value
- [ ] No layout shift

---

### 4. Add Hover Effects to Quick Action Cards

**Change:** Add interactive hover states

**Specifications:**

```tsx
<Link
  className={cn(
    "... existing classes ...",
    "transition-all duration-200",
    "hover:shadow-md hover:-translate-y-0.5"
  )}
>
```

**Animation:**

- Duration: 200ms
- Transform: translateY(-2px)
- Shadow: Add `shadow-md`

**Applies to:** All 3 Quick Action cards

**Acceptance Criteria:**

- [ ] Cards lift slightly on hover
- [ ] Shadow increases on hover
- [ ] Transition is smooth (200ms)

---

## Measurements

| Element                  | Current       | Recommended |
| ------------------------ | ------------- | ----------- |
| Header height            | 64px          | ✅ Good     |
| Sidebar width            | 256px         | ✅ Good     |
| Sidebar collapsed        | 64px          | ✅ Good     |
| Main content padding     | 24px          | ✅ Good     |
| Stats card gap           | 24px          | ✅ Good     |
| Quick action card height | Auto (~150px) | ✅ Good     |
| Section spacing          | 32px          | ✅ Good     |

---

## Responsive Behavior

### Mobile (320-428px)

- ✅ Sidebar hidden, hamburger menu
- ✅ Stats cards stack vertically
- ✅ Quick actions stack vertically
- ⚠️ Activity/Status cards could have more padding

### Tablet (768px)

- ✅ Sidebar hidden, hamburger menu
- ✅ Stats cards 2x2 grid
- ✅ Quick actions 3-col

### Desktop (1280px+)

- ✅ Sidebar visible
- ✅ Stats cards 4-col
- ✅ Activity/Status side-by-side

---

## Accessibility

| Check             | Status     | Notes                  |
| ----------------- | ---------- | ---------------------- |
| Skip link         | ✅ Present | Links to #main-content |
| Heading hierarchy | ⚠️ Issue   | Dual H1 on mobile      |
| ARIA labels       | ✅ Good    | Navigation labeled     |
| Focus states      | ✅ Visible | Ring visible on focus  |
| Color contrast    | ✅ Good    | Passes WCAG AA         |
| Touch targets     | ✅ Good    | Nav items 48px+        |

---

## Screenshots Reference

| State             | Breakpoint       | File                                 |
| ----------------- | ---------------- | ------------------------------------ |
| Default           | Desktop 1280px   | `01-dashboard-desktop.png`           |
| Default           | Desktop L 1536px | `01-dashboard-desktop-l.png`         |
| Default           | Tablet 768px     | `01-dashboard-tablet.png`            |
| Default           | Mobile L 428px   | `01-dashboard-mobile-l.png`          |
| Default           | Mobile S 320px   | `01-dashboard-mobile-s.png`          |
| Sidebar Collapsed | Desktop          | `01-dashboard-sidebar-collapsed.png` |
| Mobile Menu Open  | Mobile           | `mobile-menu-open-375.png`           |

---

_Last updated: December 25, 2025_
