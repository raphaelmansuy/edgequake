# Tablet Responsive Audit

**Breakpoints Tested:** 768px (iPad Mini), 820px (iPad), 1024px (iPad Pro 11")  
**Screens Reviewed:** All six main screens  
**Orientation:** Landscape (primary), Portrait (secondary)  
**Components:** Sidebar, Panels, Tables, Grids

---

## Tablet Summary

| Screen | Tablet Score | Layout Strategy | Key Notes |
|--------|--------------|-----------------|-----------|
| Dashboard | 4.3/5 | 2-3 column grid | Good adaptation |
| Documents | 4.0/5 | Table with sidebar | Works well |
| Query | 4.5/5 | Optimal layout | Best experience |
| Graph | 4.2/5 | Sidebar + canvas | Room for graph |
| Settings | 4.0/5 | Tabs + content | Works well |
| API Explorer | 4.0/5 | Split view | Both panels fit |

---

## Tablet Layout Strategy

### Sidebar Behavior at 768px
```
Portrait (768px):
┌────┬─────────────────────────┐
│ 64 │                         │
│ px │    Main Content         │  <- Collapsed sidebar
│    │    (flex-1)             │
│    │                         │
└────┴─────────────────────────┘

Landscape (1024px):
┌────────────┬──────────────────┐
│   256px    │                  │
│            │   Main Content   │  <- Expanded sidebar
│  Sidebar   │   (flex-1)       │
│            │                  │
└────────────┴──────────────────┘
```

---

## Issues by Screen

### Dashboard (768px)

**Current Behavior:**
- Stats cards in 2-3 column grid ✅
- Quick actions visible ✅
- Good use of space ✅

**Issues:**
| Issue | Severity | Notes |
|-------|----------|-------|
| Card spacing | 🟡 Minor | Could use 24px gap |

**Recommendations:**
- 2-column at 768px
- 3-column at 1024px
- Larger stat numbers

---

### Documents (768px)

**Current Behavior:**
- Table view works ✅
- Right panel might be tight ⚠️

**Issues:**
| Issue | Severity | Notes |
|-------|----------|-------|
| Panel width | 🟡 Minor | Consider 300px max |
| Table columns | 🟡 Minor | May need priority columns |

**Recommendations:**
- Hide less important columns at 768px
- Show key columns: Name, Date, Size, Actions
- Panel as overlay instead of pushing content

---

### Query (768px)

**Current Behavior:**
- Chat interface optimal ✅
- Conversation history accessible ✅

**Issues:**
| Issue | Severity | Notes |
|-------|----------|-------|
| History panel | 🟡 Minor | Could be collapsible |

**Recommendations:**
- This screen works well on tablet
- Consider history as sheet on portrait

---

### Graph (768px)

**Current Behavior:**
- Entity browser + canvas fits ✅
- More room for graph ✅

**Issues:**
| Issue | Severity | Notes |
|-------|----------|-------|
| Entity browser width | 🟡 Minor | 200px sufficient |
| Controls | 🟡 Minor | Could consolidate |

**Recommendations:**
- Slightly narrower entity browser
- More canvas space
- Controls in single toolbar

---

### Settings (768px)

**Current Behavior:**
- Horizontal or vertical tabs work ⚠️

**Issues:**
| Issue | Severity | Notes |
|-------|----------|-------|
| Tab orientation | 🟡 Minor | Horizontal may overflow |

**Recommendations:**
- Vertical tabs work well at 768px+
- Horizontal tabs at 1024px+

---

### API Explorer (768px)

**Current Behavior:**
- Split view fits ✅
- Both panels visible ✅

**Issues:**
| Issue | Severity | Notes |
|-------|----------|-------|
| Panel proportions | 🟡 Minor | 40/60 split might work better |

**Recommendations:**
- 40% request / 60% response
- Resizable divider

---

## Tablet-Specific Patterns

### Sidebar at Tablet Breakpoints

| Breakpoint | Sidebar State | Width | Behavior |
|------------|---------------|-------|----------|
| 768px | Collapsed | 64px | Icons only, tooltips |
| 1024px | Expanded | 256px | Full with text |
| 1024px+ | User choice | 64/256px | Toggle available |

```tsx
// Responsive sidebar logic
const getSidebarState = (width: number, userPref: boolean | null) => {
  if (width < 768) return 'hidden'; // Mobile: sheet
  if (width < 1024) return 'collapsed'; // Tablet portrait: icons
  return userPref ?? 'expanded'; // Tablet landscape+: user pref
};
```

---

### Two-Panel Layouts on Tablet

For screens with side panels (Documents, Graph):

```tsx
// Tablet panel sizing
<div className="flex">
  {/* Main content */}
  <main className="flex-1 min-w-0">
    {children}
  </main>
  
  {/* Side panel - adjust width by breakpoint */}
  <aside className={cn(
    "border-l shrink-0",
    "w-[280px]",           // 768px: narrower
    "lg:w-[320px]",        // 1024px: standard
    "xl:w-[400px]"         // 1280px+: comfortable
  )}>
    <PanelContent />
  </aside>
</div>
```

---

### Grid Layouts

| Breakpoint | Dashboard Cards | Document Grid |
|------------|-----------------|---------------|
| 768px | 2 columns | 2 columns or table |
| 1024px | 3 columns | 3 columns or table |

```tsx
// Responsive grid
<div className={cn(
  "grid gap-4",
  "grid-cols-1",        // Mobile
  "sm:grid-cols-2",     // 640px+
  "md:grid-cols-2",     // 768px+
  "lg:grid-cols-3",     // 1024px+
  "xl:grid-cols-4"      // 1280px+
)}>
```

---

## Touch Considerations

Tablet users may use touch or keyboard/mouse. Design for both:

### Touch-Friendly
- 44px minimum touch targets ✅
- Visible hover states on touch ✅
- No hover-only features ✅

### Mouse-Friendly
- Precise interactions work ✅
- Context menus available ⚠️
- Keyboard shortcuts work ⚠️

---

## Landscape vs Portrait

### Portrait (768x1024)
- Sidebar: Collapsed (64px)
- Panels: Overlay or narrower
- More vertical scroll

### Landscape (1024x768)
- Sidebar: Expanded (256px)
- Panels: Side by side
- Less vertical scroll

```tsx
// Detect orientation
const isPortrait = window.innerHeight > window.innerWidth;
```

---

## iPad-Specific Considerations

### Split View Support
When iPad is in split view (multitasking), the app may be in a narrower viewport:

| Split Mode | App Width | Design Behavior |
|------------|-----------|-----------------|
| Full | 1024px+ | Desktop layout |
| 2/3 | ~680px | Tablet layout |
| 1/2 | ~512px | Compressed tablet |
| 1/3 | ~320px | Mobile layout |

### Safe Areas
```css
/* iPad notch/home indicator */
@supports (padding: max(0px)) {
  .main-content {
    padding-left: max(16px, env(safe-area-inset-left));
    padding-right: max(16px, env(safe-area-inset-right));
  }
}
```

---

## Recommendations Summary

### High Priority

1. **Collapse sidebar at 768px by default**
   - Icons only
   - Tooltips on hover/focus
   - Expand on 1024px+

2. **Adjust panel widths**
   - Documents panel: 280px at 768px
   - Graph entity browser: 200px at 768px

3. **Responsive grid columns**
   - 2 columns at 768px
   - 3 columns at 1024px

### Medium Priority

4. **Orientation-aware layouts**
   - Portrait: More compact
   - Landscape: More horizontal space

5. **Touch-friendly interactions**
   - Larger buttons
   - Clear touch targets

### Low Priority

6. **Split view support**
   - Handle narrow widths gracefully
   - Adapt layout dynamically

---

## Testing Checklist

### Portrait Mode (768x1024)
- [ ] Sidebar collapsed by default
- [ ] All screens accessible
- [ ] Panels fit properly
- [ ] No horizontal scroll

### Landscape Mode (1024x768)
- [ ] Sidebar expanded
- [ ] Two-panel layouts work
- [ ] Tables have room
- [ ] Good content density

### Touch Interactions
- [ ] 44px touch targets
- [ ] No hover-only features
- [ ] Gestures work (if implemented)

### Keyboard
- [ ] Tab navigation works
- [ ] Shortcuts work
- [ ] Focus visible

---

*Last updated: December 25, 2025*
