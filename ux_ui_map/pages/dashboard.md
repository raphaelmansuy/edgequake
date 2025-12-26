# Page: Dashboard

## Overview

- **Route**: `/`
- **Title**: "Tableau de bord" (Dashboard)
- **Layout**: Responsive grid layout, max-width container, centered
- **Source File**: [src/app/(dashboard)/page.tsx](../../edgequake_webui/src/app/(dashboard)/page.tsx)

## Layout Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│ ┌───────────────┬─────────────────────────────────────────────────┐ │
│ │               │ Header (64px, fixed)                            │ │
│ │               │ [Logo] [Mobile Menu]    [API Status] [Theme] [User] │
│ │               ├─────────────────────────────────────────────────┤ │
│ │   Sidebar     │ Breadcrumb (48px)                               │ │
│ │   (256px/64px)│ [Home]                                          │ │
│ │               ├─────────────────────────────────────────────────┤ │
│ │   Navigation: │ Main Content (fluid, p-6)                       │ │
│ │   - Dashboard │ ┌─────────────────────────────────────────────┐ │ │
│ │   - Graph     │ │ Page Header                                 │ │ │
│ │   - Documents │ │ "Tableau de bord" + Welcome message         │ │ │
│ │   - Query     │ └─────────────────────────────────────────────┘ │ │
│ │   - API       │                                                 │ │
│ │   - Settings  │ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐   │ │
│ │               │ │Stats   │ │Stats   │ │Stats   │ │Stats   │   │ │
│ │               │ │Card    │ │Card    │ │Card    │ │Card    │   │ │
│ │               │ └────────┘ └────────┘ └────────┘ └────────┘   │ │
│ │               │                                                 │ │
│ │               │ ┌─────────────────────────────────────────────┐ │ │
│ │               │ │ Quick Actions Card                          │ │ │
│ │               │ │ [Upload] [Query] [Graph]                    │ │ │
│ │               │ └─────────────────────────────────────────────┘ │ │
│ │               │                                                 │ │
│ │               │ ┌──────────────────────┐ ┌──────────────────┐ │ │
│ │   Toggle      │ │ Recent Activity      │ │ System Status    │ │ │
│ │   [Collapse]  │ │ (2/3 width)          │ │ (1/3 width)      │ │ │
│ │               │ └──────────────────────┘ └──────────────────┘ │ │
│ │   App Info    │                                                 │ │
│ └───────────────┴─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

## Screenshots

| Viewport | Screenshot |
|----------|------------|
| Desktop (1440px) | [desktop.png](../screenshots/dashboard/desktop.png) |
| Tablet (768px) | [tablet.png](../screenshots/dashboard/tablet.png) |
| Mobile (375px) | [mobile.png](../screenshots/dashboard/mobile.png) |

---

## Region: Sidebar

- **Position**: Left, fixed width
- **Dimensions**: 256px expanded, 64px collapsed
- **Behavior**: Collapsible with toggle button; hidden on mobile (replaced by Sheet)
- **Background**: `var(--card)` - `oklch(1 0 0)` light / `oklch(0.205 0 0)` dark
- **Border**: 1px solid `var(--border)` on right edge
- **Source File**: [src/components/layout/sidebar.tsx](../../edgequake_webui/src/components/layout/sidebar.tsx)

### Container: Logo Area

- **Position**: Top, 64px height
- **Content**: EdgeQuake logo icon (36px × 36px, rounded-xl, bg-primary) + "EdgeQuake" text
- **Behavior**: Logo icon only when collapsed

### Container: Tenant Selector

- **Position**: Below logo, 12px padding
- **Type**: Dropdown selector
- **Content**: Current workspace indicator with chevron
- **Behavior**: Compact icon-only mode when sidebar collapsed

### Container: Navigation Menu

- **Position**: Flexible grow area below tenant selector
- **Type**: Vertical navigation list
- **Spacing**: 4px gap between items, 12px horizontal padding

#### Component: Nav Item

- **Type**: Link button
- **Dimensions**: Full width, 44px minimum height (touch target compliant)
- **Typography**: 14px, medium (500) weight
- **Spacing**: 12px padding, 12px gap between icon and label
- **Border Radius**: 12px (rounded-xl)
- **States**:
  - Default: `text-muted-foreground`, transparent background
  - Hover: `bg-muted`, `text-foreground`
  - Active: `bg-primary`, `text-primary-foreground`, shadow-sm
- **Icons**: 20px × 20px, Lucide icons (Home, Network, FileText, MessageSquare, Terminal, Settings)

### Container: Footer

- **Position**: Bottom, border-top
- **Content**: Collapse toggle button + App info (logo, name, version)
- **Behavior**: Shows tooltip on hover when collapsed

---

## Region: Header

- **Position**: Top, full width of main content area
- **Dimensions**: 64px height, fixed
- **Background**: `var(--card)` 
- **Border**: 1px solid `var(--border)` on bottom
- **Source File**: [src/components/layout/header.tsx](../../edgequake_webui/src/components/layout/header.tsx)

### Container: Left Section

- **Position**: Left side
- **Content**: Mobile menu button (hamburger icon), "EdgeQuake" text (mobile only)
- **Behavior**: Menu button visible only on mobile (md:hidden)

### Container: Right Section

- **Position**: Right side, flex row
- **Spacing**: 8px gap between items

#### Component: API Status Indicator

- **Type**: Status badge with dot
- **Content**: Colored dot (2px circle) + "API [version]" text
- **States**:
  - Connected: Green dot (`text-green-500`), shows version
  - Disconnected: Red dot (`text-red-500`), shows "Offline"
  - Checking: Yellow dot (`text-yellow-500`), animated pulse

#### Component: Language Selector

- **Type**: Dropdown button
- **Icon**: Globe icon (20px)
- **Behavior**: Opens dropdown with language options (EN, ZH, JA, KO)

#### Component: Theme Toggle

- **Type**: Dropdown button
- **Icons**: Sun/Moon icons with rotation transition
- **Options**: Light, Dark, System

#### Component: User Menu

- **Type**: Dropdown button
- **Icon**: User icon (20px)
- **Content**: Username, email (if authenticated), Logout option

---

## Region: Breadcrumb

- **Position**: Below header
- **Dimensions**: ~48px height with 12px vertical padding
- **Background**: `var(--muted)/30` (30% opacity muted)
- **Border**: 1px solid `var(--border)` on bottom
- **Content**: Dynamic breadcrumb trail based on current route
- **Source File**: [src/components/layout/dynamic-breadcrumb.tsx](../../edgequake_webui/src/components/layout/dynamic-breadcrumb.tsx)

---

## Region: Main Content

- **Position**: Center, below breadcrumb
- **Dimensions**: Fluid width, scrollable height
- **Background**: `var(--background)`
- **Padding**: 24px (`p-page` custom class)
- **Spacing**: 32px gap between sections

### Container: Page Header

- **Type**: Text block
- **Content**: H1 heading + subtitle paragraph
- **Typography**: 
  - Title: 30px, bold (700), tracking-tight
  - Subtitle: 16px, `text-muted-foreground`, max-width 672px

### Container: Stats Grid

- **Type**: Responsive card grid
- **Layout**: 4 columns on desktop (lg:grid-cols-4), 2 columns on tablet, 1 on mobile
- **Spacing**: 24px gap between cards

#### Component: Stats Card

- **Type**: Card component
- **Dimensions**: Flexible width, auto height
- **Border**: 1px solid `var(--border)`
- **Border Radius**: 12px
- **Shadow**: shadow-sm
- **Background**: `var(--card)` with variant-specific gradient overlay
- **Source File**: [src/components/dashboard/stats-card.tsx](../../edgequake_webui/src/components/dashboard/stats-card.tsx)
- **Variants**:
  - documents: Blue gradient accent
  - entities: Purple gradient accent
  - relationships: Emerald gradient accent
  - types: Orange gradient accent

### Container: Quick Actions Card

- **Type**: Card with action links
- **Layout**: Header + 3-column grid of action cards
- **Source File**: [src/components/dashboard/quick-actions.tsx](../../edgequake_webui/src/components/dashboard/quick-actions.tsx)

#### Component: Action Card

- **Type**: Link card with icon
- **Dimensions**: Full column width, min-height for touch target
- **Content**: Icon (in colored circle), title, description
- **States**: Hover lift effect with shadow transition
- **Variants**: Upload (blue), Query (purple), Graph (green)

### Container: Activity & Status Grid

- **Type**: 2-column responsive grid (3:1 ratio on desktop)
- **Layout**: Recent Activity (2/3) + System Status (1/3)

#### Component: Recent Activity Card

- **Type**: Card with document list
- **Content**: Header + scrollable list of recent documents or empty state
- **Source File**: [src/components/dashboard/recent-activity.tsx](../../edgequake_webui/src/components/dashboard/recent-activity.tsx)

#### Component: System Status Card

- **Type**: Card with status indicators
- **Content**: API status, version, LLM provider status
- **Source File**: [src/components/dashboard/system-status.tsx](../../edgequake_webui/src/components/dashboard/system-status.tsx)

---

## Responsive Behavior

| Breakpoint | Sidebar | Stats Grid | Actions Grid | Activity Grid |
|------------|---------|------------|--------------|---------------|
| Mobile (<768px) | Hidden (Sheet) | 1 column | 1 column | 1 column |
| Tablet (768-1024px) | 64px collapsed | 2 columns | 3 columns | 1 column |
| Desktop (>1024px) | 256px expanded | 4 columns | 3 columns | 2 columns (3:1) |

---

## Component Cross-References

- [Button](../components/buttons.md) — Used in header actions, sidebar toggle
- [Card](../components/cards.md) — Stats cards, quick actions, activity panels
- [Navigation](../components/navigation.md) — Sidebar navigation, breadcrumb
- [Dropdown Menu](../components/dialogs.md) — Theme selector, user menu
- [Tooltip](../components/dialogs.md) — Sidebar icons when collapsed
