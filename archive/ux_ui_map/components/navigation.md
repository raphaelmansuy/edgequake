# Component: Navigation

## Source Files

- **Sidebar**: [src/components/layout/sidebar.tsx](../../edgequake_webui/src/components/layout/sidebar.tsx)
- **Header**: [src/components/layout/header.tsx](../../edgequake_webui/src/components/layout/header.tsx)
- **Breadcrumb**: [src/components/ui/breadcrumb.tsx](../../edgequake_webui/src/components/ui/breadcrumb.tsx)
- **Dynamic Breadcrumb**: [src/components/layout/dynamic-breadcrumb.tsx](../../edgequake_webui/src/components/layout/dynamic-breadcrumb.tsx)
- **Tabs**: [src/components/ui/tabs.tsx](../../edgequake_webui/src/components/ui/tabs.tsx)
- **Scroll Area**: [src/components/ui/scroll-area.tsx](../../edgequake_webui/src/components/ui/scroll-area.tsx)
- **Separator**: [src/components/ui/separator.tsx](../../edgequake_webui/src/components/ui/separator.tsx)

---

## Sidebar Component

### Base Styles

- **Width**: 256px expanded, 64px collapsed
- **Position**: Fixed left
- **Height**: Full viewport height
- **Background**: `var(--card)`
- **Border**: 1px solid `var(--border)` on right

### Collapsed State

- **Width**: 64px
- **Content**: Icons only
- **Tooltips**: Show on hover

### Sections

#### Logo Area

- **Height**: 64px
- **Content**: Logo icon (36px) + "EdgeQuake" text
- **Collapsed**: Icon only

#### Tenant Selector

- **Type**: Dropdown button
- **Content**: Current workspace name + chevron
- **Collapsed**: Icon only

#### Navigation Menu

- **Layout**: Vertical list
- **Gap**: 4px between items
- **Padding**: 12px horizontal

##### Nav Item

- **Height**: 44px (touch target)
- **Border Radius**: 12px (rounded-xl)
- **Padding**: 12px
- **Gap**: 12px (icon to label)
- **States**:
  - Default: text-muted-foreground, transparent bg
  - Hover: bg-muted, text-foreground
  - Active: bg-primary, text-primary-foreground, shadow-sm

#### Footer

- **Position**: Bottom
- **Content**: Collapse toggle + App info
- **Border**: 1px solid `var(--border)` on top

### Navigation Items

| Icon | Label | Route |
|------|-------|-------|
| Home | Dashboard | `/` |
| Network | Graph | `/graph` |
| FileText | Documents | `/documents` |
| MessageSquare | Query | `/query` |
| Terminal | API Explorer | `/api-explorer` |
| Settings | Settings | `/settings` |

---

## Header Component

### Base Styles

- **Height**: 64px
- **Position**: Top of main content area
- **Background**: `var(--card)`
- **Border**: 1px solid `var(--border)` on bottom

### Layout

```
┌───────────────────────────────────────────────────────────────────┐
│ [Mobile Menu]                    [API Status] [Lang] [Theme] [User]│
└───────────────────────────────────────────────────────────────────┘
```

### Left Section (Mobile)

- **Mobile Menu**: Hamburger icon (Sheet trigger)
- **Logo Text**: "EdgeQuake" (visible on mobile)

### Right Section

- **API Status**: Colored dot + version text
- **Language Selector**: Globe icon dropdown
- **Theme Toggle**: Sun/Moon icon dropdown
- **User Menu**: User icon dropdown

### API Status Indicator

| State | Dot Color | Text |
|-------|-----------|------|
| Connected | Green | "API {version}" |
| Disconnected | Red | "Offline" |
| Checking | Yellow (pulse) | "Checking..." |

---

## Breadcrumb Component

### Base Styles

```
flex flex-wrap items-center gap-1.5 text-sm
```

- **Layout**: Flex row with wrap
- **Gap**: 6px
- **Typography**: 14px

### Elements

#### BreadcrumbItem

```
inline-flex items-center gap-1.5
```

#### BreadcrumbLink

```
transition-colors hover:text-foreground
```

- **Default**: text-muted-foreground
- **Hover**: text-foreground
- **Current**: text-foreground (not clickable)

#### BreadcrumbSeparator

```
[&>svg]:h-3.5 [&>svg]:w-3.5
```

- **Icon**: ChevronRight
- **Size**: 14px

### Dynamic Breadcrumb

- **Source**: [src/components/layout/dynamic-breadcrumb.tsx](../../edgequake_webui/src/components/layout/dynamic-breadcrumb.tsx)
- **Behavior**: Auto-generates from route path
- **Icons**: Route-specific icons

### Breadcrumb Bar

- **Height**: ~48px (with padding)
- **Background**: `var(--muted)/30`
- **Border**: 1px solid `var(--border)` on bottom
- **Padding**: 12px vertical

---

## Tabs Component

### Base Styles

```
inline-flex h-9 items-center justify-center rounded-lg bg-muted p-1
```

- **Height**: 36px
- **Background**: `var(--muted)`
- **Border Radius**: 8px
- **Padding**: 4px

### Tab Trigger

```
inline-flex items-center justify-center whitespace-nowrap rounded-md 
px-3 py-1 text-sm font-medium ring-offset-background
```

- **Border Radius**: 6px
- **Padding**: 12px horizontal, 4px vertical
- **States**:
  - Default: Transparent background
  - Active: bg-background, shadow
  - Hover: Slight background change

### Usage Locations

- Graph: View mode tabs (Grouped / List)
- Query: Mode selector (Local / Global / Hybrid / Simple)

---

## Scroll Area Component

### Base Styles

```
relative overflow-hidden
```

- **Viewport**: Full area, overflow hidden
- **Content**: Actual scrollable content

### Scrollbar

```
flex touch-none select-none transition-colors
```

- **Orientation**: Vertical or horizontal
- **Width**: 10px (vertical), Height: 10px (horizontal)
- **Thumb**: rounded-full bg-border

### Usage Locations

- Sidebar: Navigation menu
- Graph: Entity list
- Query: Chat messages, conversation history
- API Explorer: Endpoint list, response area

---

## Separator Component

### Base Styles

```
shrink-0 bg-border
```

- **Background**: `var(--border)`
- **Orientation**: 
  - Horizontal: `h-[1px] w-full`
  - Vertical: `h-full w-[1px]`

### Usage Locations

- Settings: Between setting sections
- Dropdown menus: Between item groups
- Header: Between actions (optional)

---

## Responsive Navigation Patterns

### Desktop (>1024px)

- Sidebar: Visible, collapsible
- Header: Full header bar
- Breadcrumb: Full path shown

### Tablet (768-1024px)

- Sidebar: Collapsed (icons only)
- Header: Full header bar
- Breadcrumb: May truncate

### Mobile (<768px)

- Sidebar: Hidden (Sheet overlay)
- Header: Mobile menu button + logo
- Breadcrumb: Shortened path

---

## Accessibility

### Keyboard Navigation

- Tab: Move between interactive elements
- Arrow keys: Navigate within menus
- Enter/Space: Activate links/buttons
- Escape: Close overlays

### ARIA

- `role="navigation"` on nav containers
- `aria-current="page"` on current nav item
- `aria-expanded` on collapsible triggers
- `aria-label` on navigation regions

### Focus Management

- Visible focus ring on all interactive elements
- Focus trap in overlays (Sheet, Dialog)
- Return focus on overlay close

