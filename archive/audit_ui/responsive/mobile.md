# Mobile Responsive Audit

**Breakpoints Tested:** 320px (iPhone SE), 375px (iPhone 12), 428px (iPhone 12 Pro Max)  
**Screens Reviewed:** All six main screens  
**Components:** Navigation, Panels, Forms, Cards, Dialogs  
**Orientation:** Portrait (primary), Landscape (secondary)

---

## Mobile-First Summary

| Screen       | Mobile Score | Critical Issues      | Key Recommendations  |
| ------------ | ------------ | -------------------- | -------------------- |
| Dashboard    | 3.8/5        | Stats cards crowded  | 2-column grid        |
| Documents    | 4.0/5        | Table scrolls        | Card view option     |
| Query        | 4.2/5        | Good adaptation      | Minor input fixes    |
| Graph        | 3.2/5        | Poor on mobile       | Full redesign needed |
| Settings     | 3.5/5        | Tab overflow         | Dropdown selector    |
| API Explorer | 3.0/5        | Two panels don't fit | Tabbed interface     |

---

## Mobile Layout Strategy

### Current Pattern

```
┌──────────────────────┐
│ Header  [☰] [⚙] [👤] │  48-56px
├──────────────────────┤
│                      │
│    Main Content      │  flex-1
│    (scrollable)      │
│                      │
├──────────────────────┤
│ (Optional bottom bar)│  48-56px
└──────────────────────┘
```

### Recommended Pattern

```
┌──────────────────────┐
│ [☰] Title       [⚙] │  48px header
├──────────────────────┤
│ Breadcrumb/Tabs      │  40px (optional)
├──────────────────────┤
│                      │
│    Main Content      │  flex-1
│    (scrollable)      │
│                      │
├──────────────────────┤
│ [📊] [📄] [💬] [🔗]  │  56px bottom nav
└──────────────────────┘
```

---

## Issues by Severity

### 🔴 Critical

#### Mobile Menu Accessibility Violation

- **Console Error:** `DialogContent requires DialogTitle`
- **Impact:** Screen reader users cannot navigate
- **Fix:** Add `<SheetTitle>` to mobile menu sheet

---

### 🟠 Major

#### Graph Screen Unusable on Mobile

- **320px behavior:** Entity browser and graph canvas both try to display
- **Problem:** Neither panel has enough space
- **Recommendation:**
  - Hide entity browser (put in sheet)
  - Graph canvas should be fullscreen
  - Controls in bottom sheet or floating bar

#### API Explorer Two-Panel Layout Breaks

- **320px behavior:** Panels stack but are too narrow
- **Problem:** Can't see request and response together
- **Recommendation:** Tabbed interface on mobile

#### Settings Tabs Overflow

- **320px behavior:** Tabs may cut off or wrap poorly
- **Problem:** Can't access all settings sections
- **Recommendation:** Dropdown or horizontal scroll

---

### 🟡 Minor

#### Dashboard Stats Cards Too Cramped

- **320px behavior:** Cards may be too narrow for content
- **Recommendation:** 2-column grid on 375px+, single column on 320px

#### Document Table Horizontal Scroll

- **320px behavior:** Table requires horizontal scrolling
- **Recommendation:** Card view alternative for mobile

#### Query Input Area Too Small

- **320px behavior:** Input height may be too short
- **Recommendation:** Larger input area, visible send button

#### Touch Targets Too Small

- **Various locations:** Buttons < 44px
- **Recommendation:** Minimum 44x44px touch targets

---

## Screen-by-Screen Mobile Analysis

### Dashboard (320px)

```
┌──────────────────────┐
│ [☰] EdgeQuake   [⚙]  │
├──────────────────────┤
│ Welcome back!        │
├──────────────────────┤
│ ┌──────────────────┐ │
│ │ 📄 Documents: 0  │ │
│ └──────────────────┘ │
│ ┌──────────────────┐ │
│ │ 🔗 Entities: 0   │ │
│ └──────────────────┘ │
│ ┌──────────────────┐ │
│ │ 📊 Relationships │ │
│ └──────────────────┘ │
│ ┌──────────────────┐ │
│ │ 💾 Storage       │ │
│ └──────────────────┘ │
├──────────────────────┤
│ Quick Actions        │
│ [+ Upload] [💬 Query]│
└──────────────────────┘
```

**Issues:**

- Single column works but may feel sparse
- Quick action buttons could be larger

**Recommendations:**

- Consider 2-column grid at 375px+
- Make action buttons full-width

---

### Documents (320px)

```
┌──────────────────────┐
│ [☰] Documents   [+]  │
├──────────────────────┤
│ [Search...       🔍] │
├──────────────────────┤
│ ┌──────────────────┐ │
│ │ 📄 Document 1    │ │
│ │ 1.2 KB • 2m ago  │ │
│ │ [View] [Delete]  │ │
│ └──────────────────┘ │
│ ┌──────────────────┐ │
│ │ 📄 Document 2    │ │
│ │ 3.4 KB • 5m ago  │ │
│ │ [View] [Delete]  │ │
│ └──────────────────┘ │
│         ...          │
└──────────────────────┘
```

**Issues:**

- Table view doesn't work well
- Actions may be cramped

**Recommendations:**

- Use card view on mobile
- Swipe actions for delete
- Floating action button for upload

---

### Query (320px)

```
┌──────────────────────┐
│ [☰] Query      [⚙]  │
├──────────────────────┤
│ [Hybrid ▾] [History] │
├──────────────────────┤
│                      │
│                      │
│    Chat Messages     │
│                      │
│                      │
│                      │
├──────────────────────┤
│ [Type message...   ]│
│ [            ] [➤]  │
└──────────────────────┘
```

**Issues:**

- Mode selector could be tabs
- History should open as sheet

**Recommendations:**

- Full-width input
- Clear send button (44px+)
- History as bottom sheet

---

### Graph (320px) - NEEDS REDESIGN

```
┌──────────────────────┐
│ [☰] Graph   [🔗][⚙] │
├──────────────────────┤
│ ┌──────────────────┐ │
│ │                  │ │
│ │   Graph Canvas   │ │
│ │   (fullscreen)   │ │
│ │                  │ │
│ │    [+] [-] [↻]   │ │
│ └──────────────────┘ │
├──────────────────────┤
│ [🔍 Entities Sheet] │
└──────────────────────┘
```

**Current Problems:**

- Entity browser takes up space
- Graph canvas too small
- Controls conflict

**Recommendations:**

- Graph should be fullscreen
- Entity browser as bottom sheet
- Controls as floating bar
- Gesture navigation (pinch/zoom)

---

### Settings (320px)

```
┌──────────────────────┐
│ [☰] Settings        │
├──────────────────────┤
│ [General        ▾]  │  <- Dropdown instead of tabs
├──────────────────────┤
│ Tenant ID            │
│ [________________]   │
│                      │
│ Working Mode         │
│ [Hybrid ▾]           │
│                      │
│ Language             │
│ [English ▾]          │
├──────────────────────┤
│ [Reset Settings]     │
└──────────────────────┘
```

**Issues:**

- Horizontal tabs overflow

**Recommendations:**

- Dropdown section selector
- Or vertical tabs as accordion

---

### API Explorer (320px) - NEEDS TABBED UI

```
┌──────────────────────┐
│ [☰] API Explorer    │
├──────────────────────┤
│ [Request][Response] │  <- Tabs
├──────────────────────┤
│ GET /api/health     │
│                      │
│ Headers              │
│ [________________]   │
│                      │
│ Body (JSON)          │
│ [                   ]│
│ [                   ]│
│                      │
│ [  Send Request   ] │
└──────────────────────┘
```

**Current Problems:**

- Two panels don't fit

**Recommendations:**

- Tab between Request/Response
- Badge on Response tab shows status
- Full-width JSON editor

---

## Touch Target Audit

| Element        | Current Size | Required | Status |
| -------------- | ------------ | -------- | ------ |
| Sidebar toggle | 40x40px      | 44x44px  | ⚠️     |
| Nav items      | 44px height  | 44x44px  | ✅     |
| Theme toggle   | 40x40px      | 44x44px  | ⚠️     |
| User menu      | 32x32px      | 44x44px  | ⚠️     |
| Close buttons  | 40x40px      | 44x44px  | ⚠️     |
| Form inputs    | 40px height  | 44px     | ⚠️     |
| Action buttons | Variable     | 44px min | ⚠️     |

---

## Mobile-Specific Recommendations

### 1. Add Bottom Navigation

**Rationale:** Easier thumb access on mobile

```tsx
// Only show on mobile
<nav className="fixed bottom-0 left-0 right-0 h-14 border-t bg-background flex md:hidden">
  {navItems.slice(0, 4).map((item) => (
    <Link
      key={item.href}
      href={item.href}
      className={cn(
        "flex-1 flex flex-col items-center justify-center gap-1",
        isActive && "text-primary"
      )}
    >
      <item.icon className="h-5 w-5" />
      <span className="text-xs">{item.name}</span>
    </Link>
  ))}
  <button className="flex-1 flex flex-col items-center justify-center gap-1">
    <MoreHorizontal className="h-5 w-5" />
    <span className="text-xs">More</span>
  </button>
</nav>
```

---

### 2. Swipe Gestures

**For Document List:**

```tsx
// Swipe left to reveal delete
<SwipeableItem
  rightActions={[
    { label: "Delete", color: "destructive", onClick: handleDelete },
  ]}
>
  <DocumentCard document={doc} />
</SwipeableItem>
```

---

### 3. Pull to Refresh

**For All Lists:**

```tsx
<PullToRefresh onRefresh={refetchData}>
  <DocumentList />
</PullToRefresh>
```

---

### 4. Increase Touch Targets

```css
/* Global mobile touch target fix */
@media (max-width: 768px) {
  button,
  [role="button"],
  a,
  input,
  select {
    min-height: 44px;
    min-width: 44px;
  }

  .icon-button {
    padding: 10px; /* Increase from 8px */
  }
}
```

---

## Mobile Testing Checklist

### Functionality

- [ ] All screens accessible
- [ ] Navigation works
- [ ] Forms submit correctly
- [ ] Actions complete
- [ ] Data loads properly

### Gestures

- [ ] Scroll works smoothly
- [ ] Pull to refresh (if implemented)
- [ ] Swipe actions (if implemented)
- [ ] Pinch to zoom (on graph)

### Performance

- [ ] Fast initial load
- [ ] Smooth scrolling
- [ ] No jank during interactions
- [ ] Images lazy load

### Offline

- [ ] Graceful offline handling
- [ ] Cached data displays
- [ ] Reconnection works

---

## Viewport Meta Tag

Ensure proper viewport:

```html
<meta
  name="viewport"
  content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no"
/>
```

Note: Consider removing `user-scalable=no` for accessibility.

---

## Safe Areas

For notched devices:

```css
.header {
  padding-top: env(safe-area-inset-top);
}

.bottom-nav {
  padding-bottom: env(safe-area-inset-bottom);
}
```

---

## Implementation Priority

1. **Critical:** Fix mobile menu accessibility
2. **High:** Redesign Graph screen for mobile
3. **High:** Add tabbed UI for API Explorer
4. **Medium:** Add card view for Documents
5. **Medium:** Fix settings tabs
6. **Medium:** Increase touch targets
7. **Low:** Add bottom navigation
8. **Low:** Add gesture support

---

_Last updated: December 25, 2025_
