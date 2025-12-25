# Navigation Patterns Audit

**Components Reviewed:**

- Sidebar (`src/components/layout/sidebar.tsx`)
- Header (`src/components/layout/header.tsx`)
- Breadcrumb (`src/components/layout/dynamic-breadcrumb.tsx`)
- Mobile Menu (Sheet-based)
- User Menu (Dropdown)

**Cross-cutting Concerns:** Consistency, Mobile Adaptation, Keyboard Navigation, Active States

---

## Navigation Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ ┌─ HEADER ────────────────────────────────────────────────┐ │
│ │ [☰] Logo/Title        │ API Status │ Theme │ User [⌄] │ │
│ └─────────────────────────────────────────────────────────┘ │
├────────────┬────────────────────────────────────────────────┤
│ SIDEBAR    │ ┌─ BREADCRUMB ──────────────────────────────┐ │
│ ┌────────┐ │ │ Home > Section > Page                    │ │
│ │📊 Dash │ │ └───────────────────────────────────────────┘ │
│ │📄 Docs │ │                                              │
│ │💬 Query│ │                                              │
│ │🔗 Graph│ │           MAIN CONTENT                       │
│ │⚙️ Sett │ │                                              │
│ │🔧 API  │ │                                              │
│ └────────┘ │                                              │
└────────────┴────────────────────────────────────────────────┘
```

---

## Slickness Score

| Criterion           | Score (1–5) | Notes                                |
| ------------------- | ----------- | ------------------------------------ |
| Visual consistency  | 4.2         | Good icon + text pattern             |
| Animation quality   | 4.0         | Smooth collapse                      |
| Mobile adaptation   | 3.5         | Works but has a11y issue             |
| Keyboard navigation | 3.8         | Could add shortcuts                  |
| **Overall**         | **3.9**     | Solid foundation, minor fixes needed |

---

## Component Analysis

### Sidebar (`sidebar.tsx`)

#### Current Implementation

- Collapsible width: 64px (collapsed) ↔ 256px (expanded)
- Desktop: Always visible, collapse toggle
- Mobile: Sheet drawer from left
- Icons: Lucide icons with text labels
- Active state: Background highlight

#### Issues

| Issue                          | Severity | Notes                                 |
| ------------------------------ | -------- | ------------------------------------- |
| Missing tooltip on collapsed   | 🟡 Minor | Hard to identify icons when collapsed |
| No keyboard shortcuts          | 🟡 Minor | Common nav shortcuts missing          |
| Collapse persists but may race | 🟡 Minor | Check hydration mismatch              |

---

### Header (`header.tsx`)

#### Current Implementation

- Fixed height: 64px
- Contains: Logo, API status, theme toggle, user menu
- Mobile: Hamburger menu trigger

#### Issues

| Issue                              | Severity | Notes                  |
| ---------------------------------- | -------- | ---------------------- |
| API status could be more prominent | 🟡 Minor | Small badge            |
| No breadcrumb in header on mobile  | 🟡 Minor | Lost context           |
| Theme toggle position              | 🟡 Minor | Consider settings only |

---

### Breadcrumb (`dynamic-breadcrumb.tsx`)

#### Current Implementation

- Shows current route hierarchy
- Separator: `/` or `>`
- Links to parent routes

#### Issues

| Issue                      | Severity | Notes                  |
| -------------------------- | -------- | ---------------------- |
| May truncate on long paths | 🟡 Minor | Need ellipsis strategy |
| Mobile visibility          | 🟡 Minor | May be hidden          |
| Not focusable              | 🟡 Minor | Add role="navigation"  |

---

### Mobile Menu

#### Current Implementation

- Sheet component from left
- Same nav items as sidebar
- Close on navigation

#### Issues

| Issue                 | Severity    | Notes                    |
| --------------------- | ----------- | ------------------------ |
| Missing DialogTitle   | 🔴 Critical | Accessibility violation  |
| Close on route change | ⚠️ Check    | Should auto-close        |
| No search             | 🟡 Minor    | Quick navigation missing |

---

## Issues Summary

### 🔴 Critical

#### Mobile Menu Missing Accessibility Attributes

- **Severity:** 🔴 Critical
- **Location:** Mobile sheet menu
- **Current behavior:** DialogContent lacks DialogTitle
- **Expected behavior:** Must have title for screen readers
- **Console error:** `DialogContent requires DialogTitle`

**Fix:**

```tsx
<SheetContent side="left">
  <SheetHeader>
    <SheetTitle>Navigation</SheetTitle>
    <SheetDescription className="sr-only">
      Main navigation menu
    </SheetDescription>
  </SheetHeader>
  {/* nav items */}
</SheetContent>
```

---

### 🟠 Major

#### Sidebar Tooltips on Collapsed State

- **Severity:** 🟠 Major
- **Location:** Collapsed sidebar icons
- **Current behavior:** No tooltip, icons only
- **Expected behavior:** Tooltip showing nav item name

**Fix:**

```tsx
<Tooltip>
  <TooltipTrigger asChild>
    <Link href={item.href} className={...}>
      <item.icon className="h-5 w-5" />
      {!isCollapsed && <span>{item.name}</span>}
    </Link>
  </TooltipTrigger>
  {isCollapsed && (
    <TooltipContent side="right">
      {item.name}
    </TooltipContent>
  )}
</Tooltip>
```

---

### 🟡 Minor

#### Add Keyboard Navigation Shortcuts

- **Severity:** 🟡 Minor
- **Location:** Global
- **Expected shortcuts:**

| Shortcut | Action                        |
| -------- | ----------------------------- |
| `Cmd+1`  | Go to Dashboard               |
| `Cmd+2`  | Go to Documents               |
| `Cmd+3`  | Go to Query                   |
| `Cmd+4`  | Go to Graph                   |
| `Cmd+5`  | Go to Settings                |
| `Cmd+/`  | Toggle sidebar                |
| `Cmd+K`  | Open command palette (future) |

---

## Recommendations

### 1. Fix Mobile Menu Accessibility

**Change:** Add required ARIA attributes

**Specifications:**

```tsx
// In sidebar.tsx mobile sheet
<Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
  <SheetContent
    side="left"
    className="w-64 p-0"
    aria-describedby="mobile-nav-description"
  >
    <SheetHeader className="p-4 border-b">
      <SheetTitle>EdgeQuake</SheetTitle>
      <SheetDescription id="mobile-nav-description" className="sr-only">
        Main navigation menu for EdgeQuake application
      </SheetDescription>
    </SheetHeader>
    <nav className="flex flex-col p-2" role="navigation">
      {navItems.map(item => (
        <Link
          key={item.href}
          href={item.href}
          onClick={() => setMobileOpen(false)}
          className={cn(...)}
        >
          <item.icon className="h-5 w-5" />
          <span>{item.name}</span>
        </Link>
      ))}
    </nav>
  </SheetContent>
</Sheet>
```

**Acceptance Criteria:**

- [ ] SheetTitle present
- [ ] SheetDescription present (can be sr-only)
- [ ] No console errors
- [ ] Screen reader announces correctly

---

### 2. Add Tooltips to Collapsed Sidebar

**Change:** Show tooltips on hover when collapsed

**Specifications:**

```tsx
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

<TooltipProvider delayDuration={100}>
  <nav className="flex flex-col gap-1 p-2">
    {navItems.map((item) => (
      <Tooltip key={item.href}>
        <TooltipTrigger asChild>
          <Link
            href={item.href}
            className={cn(
              "flex items-center gap-3 px-3 py-2 rounded-md",
              "hover:bg-accent transition-colors",
              isActive && "bg-accent",
              isCollapsed && "justify-center"
            )}
          >
            <item.icon className="h-5 w-5 shrink-0" />
            {!isCollapsed && <span className="truncate">{item.name}</span>}
          </Link>
        </TooltipTrigger>
        {isCollapsed && (
          <TooltipContent side="right" sideOffset={10}>
            <p>{item.name}</p>
          </TooltipContent>
        )}
      </Tooltip>
    ))}
  </nav>
</TooltipProvider>;
```

**Acceptance Criteria:**

- [ ] Tooltip appears on hover when collapsed
- [ ] Shows on right side of icon
- [ ] 100ms delay before showing
- [ ] No tooltip when expanded (text visible)

---

### 3. Add Keyboard Navigation

**Change:** Global keyboard shortcuts for navigation

**Specifications:**

```tsx
// src/hooks/use-keyboard-shortcuts.ts
import { useHotkeys } from "react-hotkeys-hook";
import { useRouter } from "next/navigation";

export function useKeyboardShortcuts() {
  const router = useRouter();
  const { toggleSidebar } = useSettingsStore();

  // Navigation shortcuts
  useHotkeys("mod+1", () => router.push("/dashboard"), []);
  useHotkeys("mod+2", () => router.push("/documents"), []);
  useHotkeys("mod+3", () => router.push("/query"), []);
  useHotkeys("mod+4", () => router.push("/graph"), []);
  useHotkeys("mod+5", () => router.push("/settings"), []);

  // UI shortcuts
  useHotkeys("mod+/", () => toggleSidebar(), []);
  useHotkeys("mod+b", () => toggleSidebar(), []);
}

// In layout.tsx
function DashboardLayout({ children }) {
  useKeyboardShortcuts();
  // ...
}
```

**Acceptance Criteria:**

- [ ] Cmd+1-5 navigates to screens
- [ ] Cmd+/ toggles sidebar
- [ ] Works on both Mac and Windows
- [ ] No conflicts with browser shortcuts

---

### 4. Improve Breadcrumb

**Change:** Add truncation and navigation role

**Specifications:**

```tsx
<nav aria-label="Breadcrumb" className="flex items-center text-sm">
  <ol className="flex items-center gap-1.5" role="list">
    <li className="flex items-center">
      <Link
        href="/dashboard"
        className="text-muted-foreground hover:text-foreground"
      >
        EdgeQuake
      </Link>
    </li>
    {segments.map((segment, index) => (
      <li key={segment.href} className="flex items-center gap-1.5">
        <ChevronRight className="h-4 w-4 text-muted-foreground" />
        {index === segments.length - 1 ? (
          <span
            className="font-medium truncate max-w-[200px]"
            aria-current="page"
          >
            {segment.name}
          </span>
        ) : (
          <Link
            href={segment.href}
            className="text-muted-foreground hover:text-foreground truncate max-w-[150px]"
          >
            {segment.name}
          </Link>
        )}
      </li>
    ))}
  </ol>
</nav>
```

**Acceptance Criteria:**

- [ ] Has `role="navigation"` or `<nav>`
- [ ] Current page has `aria-current="page"`
- [ ] Long names truncate with ellipsis
- [ ] Links are keyboard accessible

---

### 5. Add Command Palette (Future)

**Change:** Cmd+K opens search/command palette

**Specifications:**

```tsx
// Future enhancement using cmdk or similar
<CommandDialog open={open} onOpenChange={setOpen}>
  <CommandInput placeholder="Search or type a command..." />
  <CommandList>
    <CommandGroup heading="Navigation">
      {navItems.map((item) => (
        <CommandItem key={item.href} onSelect={() => router.push(item.href)}>
          <item.icon className="mr-2 h-4 w-4" />
          {item.name}
        </CommandItem>
      ))}
    </CommandGroup>
    <CommandGroup heading="Actions">
      <CommandItem onSelect={() => {}}>
        <Upload className="mr-2 h-4 w-4" />
        Upload Document
      </CommandItem>
      <CommandItem onSelect={() => {}}>
        <Search className="mr-2 h-4 w-4" />
        Search Graph
      </CommandItem>
    </CommandGroup>
  </CommandList>
</CommandDialog>
```

**Acceptance Criteria:**

- [ ] Cmd+K opens palette
- [ ] Can search for pages
- [ ] Can execute actions
- [ ] Keyboard navigable

---

## Navigation Item Specifications

| Item         | Icon              | Route           | Badge          |
| ------------ | ----------------- | --------------- | -------------- |
| Dashboard    | `LayoutDashboard` | `/dashboard`    | -              |
| Documents    | `FileText`        | `/documents`    | Document count |
| Query        | `MessageSquare`   | `/query`        | -              |
| Graph        | `Network`         | `/graph`        | -              |
| Settings     | `Settings`        | `/settings`     | -              |
| API Explorer | `Code2`           | `/api-explorer` | -              |

---

## Responsive Behavior

### Mobile (< 768px)

- Sidebar: Hidden, accessible via hamburger
- Header: Show hamburger, API status, user menu
- Breadcrumb: May be hidden or in header

### Tablet (768px - 1024px)

- Sidebar: Collapsed by default
- Header: Full
- Breadcrumb: Visible

### Desktop (> 1024px)

- Sidebar: Expanded by default
- Header: Full
- Breadcrumb: Visible

---

## Accessibility Checklist

| Requirement           | Status         | Notes               |
| --------------------- | -------------- | ------------------- |
| Semantic nav elements | ⚠️ Add `<nav>` | Wrap in nav element |
| aria-current="page"   | ⚠️ Missing     | Add to active link  |
| aria-label on nav     | ⚠️ Missing     | "Main navigation"   |
| Focus visible         | ✅ Good        | Ring on focus       |
| Skip to content       | ⚠️ Missing     | Add skip link       |
| Mobile sheet title    | 🔴 Missing     | Critical fix needed |
| Keyboard shortcuts    | ⚠️ Missing     | Nice to have        |

### Add Skip Link

```tsx
// In layout.tsx
<a
  href="#main-content"
  className="sr-only focus:not-sr-only focus:absolute focus:z-50 focus:p-4 focus:bg-background"
>
  Skip to main content
</a>

// Main content
<main id="main-content" className="...">
```

---

## Implementation Priority

1. **Critical:** Fix mobile menu accessibility (DialogTitle)
2. **High:** Add tooltips to collapsed sidebar
3. **Medium:** Add skip link
4. **Medium:** Improve breadcrumb semantics
5. **Low:** Add keyboard shortcuts
6. **Low:** Implement command palette

---

_Last updated: December 25, 2025_
