# Breakpoint Issues & Cross-Cutting Concerns

**Breakpoints Analyzed:**

- 320px (iPhone SE, small Android)
- 375px (iPhone 12/13)
- 428px (iPhone 12/13 Pro Max)
- 768px (iPad Mini, tablet portrait)
- 1024px (iPad Pro, tablet landscape)
- 1280px (Desktop)
- 1536px (Large desktop)

---

## Breakpoint System

### Current Breakpoints (Tailwind Defaults)

| Name   | Min Width | CSS Class | Usage         |
| ------ | --------- | --------- | ------------- |
| (base) | 0px       | -         | Mobile first  |
| sm     | 640px     | `sm:`     | Large phones  |
| md     | 768px     | `md:`     | Tablets       |
| lg     | 1024px    | `lg:`     | Small laptops |
| xl     | 1280px    | `xl:`     | Desktops      |
| 2xl    | 1536px    | `2xl:`    | Large screens |

### Recommended Custom Breakpoints

```js
// tailwind.config.js
module.exports = {
  theme: {
    screens: {
      xs: "375px", // iPhone 12
      sm: "640px", // Large phones
      md: "768px", // Tablets
      lg: "1024px", // Laptops
      xl: "1280px", // Desktops
      "2xl": "1536px", // Large displays
    },
  },
};
```

---

## Cross-Breakpoint Issues

### 🔴 Critical

#### Issue 1: Layout Shift at md (768px)

- **Where:** Sidebar transition from hidden to visible
- **Problem:** Content may jump when sidebar appears
- **Impact:** Jarring experience, CLS issue

**Fix:**

```tsx
// Reserve space for sidebar even when hidden
<div className="flex">
  {/* Sidebar placeholder on mobile */}
  <div className="hidden md:block w-64 shrink-0" />

  {/* Or use consistent margin */}
  <main className="flex-1 md:ml-64">
</div>
```

---

### 🟠 Major

#### Issue 2: Panel Visibility Transitions

- **Where:** Right panels (Documents, Graph)
- **Problem:** Panels may not transition smoothly between breakpoints
- **Breakpoints affected:** 768px → 1024px

**Current behavior:**

- 768px: Panel may be hidden or sheet
- 1024px: Panel is inline

**Recommended behavior:**

- Animate panel appearance
- Maintain scroll position
- Remember open/closed state

---

#### Issue 3: Navigation Mode Changes

- **Where:** Mobile sheet ↔ Desktop sidebar
- **Problem:** Navigation state may not sync
- **Breakpoints affected:** < 768px ↔ ≥ 768px

**Fix:**

```tsx
// Sync mobile and desktop navigation state
useEffect(() => {
  const handleResize = () => {
    if (window.innerWidth >= 768 && mobileMenuOpen) {
      setMobileMenuOpen(false);
    }
  };
  window.addEventListener("resize", handleResize);
  return () => window.removeEventListener("resize", handleResize);
}, [mobileMenuOpen]);
```

---

#### Issue 4: Form Element Sizing

- **Where:** All inputs, selects, buttons
- **Problem:** May be too small on mobile, too large on desktop

**Recommended responsive sizing:**

```tsx
// Input heights by breakpoint
<Input className={cn(
  "h-12",      // Mobile: larger for touch
  "md:h-10",   // Tablet+: standard
)} />

// Button padding
<Button className={cn(
  "px-6 py-3",     // Mobile: larger
  "md:px-4 md:py-2" // Desktop: compact
)} />
```

---

### 🟡 Minor

#### Issue 5: Typography Scaling

- **Where:** Headings, body text
- **Problem:** Font sizes may not scale appropriately

**Recommended fluid typography:**

```css
/* Using clamp for fluid sizing */
h1 {
  font-size: clamp(1.5rem, 4vw, 2rem);
}

h2 {
  font-size: clamp(1.25rem, 3vw, 1.5rem);
}

p {
  font-size: clamp(0.875rem, 2.5vw, 1rem);
}
```

---

#### Issue 6: Grid Column Jumps

- **Where:** Dashboard cards, document grid
- **Problem:** Abrupt column changes

**Smoother grid transitions:**

```tsx
<div className={cn(
  "grid gap-4",
  "grid-cols-1",           // 0-374px
  "xs:grid-cols-2",        // 375-639px
  "sm:grid-cols-2",        // 640-767px
  "md:grid-cols-2",        // 768-1023px
  "lg:grid-cols-3",        // 1024-1279px
  "xl:grid-cols-4",        // 1280+
)}>
```

---

#### Issue 7: Spacing Inconsistencies

- **Where:** Various containers, cards
- **Problem:** Padding/margins don't scale

**Responsive spacing:**

```tsx
// Container padding
<div className={cn(
  "p-4",       // Mobile: 16px
  "md:p-6",    // Tablet: 24px
  "lg:p-8",    // Desktop: 32px
)} />

// Section gaps
<div className={cn(
  "space-y-4",    // Mobile
  "md:space-y-6", // Tablet
  "lg:space-y-8", // Desktop
)} />
```

---

## Breakpoint Transition Matrix

| From → To   | Element     | Current              | Recommended           |
| ----------- | ----------- | -------------------- | --------------------- |
| 320 → 768   | Sidebar     | Sheet → Inline       | ✅ Good               |
| 320 → 768   | Right panel | Hidden → Visible     | Add animation         |
| 768 → 1024  | Sidebar     | Collapsed → Expanded | ✅ Good               |
| 768 → 1024  | Grid        | 2 col → 3 col        | Add 1 col step        |
| 1024 → 1280 | Layout      | No change            | Consider max-width    |
| 1280 → 1536 | Layout      | No change            | Add 2xl optimizations |

---

## Container Width Strategy

### Problem

Content may become too wide on large screens.

### Solution

```tsx
// Global container constraint
<main className={cn(
  "flex-1",
  "max-w-[1600px]", // Prevent ultra-wide content
  "mx-auto",        // Center on large screens
  "w-full",
)}>
```

### Screen-Specific Max Widths

| Screen       | Recommended Max Width    |
| ------------ | ------------------------ |
| Dashboard    | None (full width cards)  |
| Documents    | 1400px (table readable)  |
| Query        | 900px (chat optimal)     |
| Graph        | None (need canvas space) |
| Settings     | 800px (form optimal)     |
| API Explorer | 1400px                   |

---

## CSS Media Query Best Practices

### Use Tailwind Classes (Preferred)

```tsx
// Good: Tailwind responsive
<div className="text-sm md:text-base lg:text-lg" />
```

### Custom Media Queries (When Needed)

```css
/* In globals.css for complex responsive logic */
@media (min-width: 768px) and (orientation: landscape) {
  .graph-canvas {
    height: calc(100vh - 64px);
  }
}
```

### Container Queries (Future)

```css
/* Coming to more browsers */
@container (min-width: 400px) {
  .card {
    display: grid;
    grid-template-columns: auto 1fr;
  }
}
```

---

## Responsive Testing Checklist

### At Each Breakpoint, Verify:

- [ ] **Layout:** No overflow, proper alignment
- [ ] **Typography:** Readable, appropriate size
- [ ] **Spacing:** Consistent, proportional
- [ ] **Navigation:** Accessible, functional
- [ ] **Forms:** Usable, touchable
- [ ] **Images:** Sized correctly, not distorted
- [ ] **Tables:** Scrollable or adapted
- [ ] **Modals:** Centered, sized well
- [ ] **Panels:** Visible or accessible
- [ ] **Touch targets:** ≥44px on touch devices

---

## Breakpoint-Specific CSS Variables

Consider dynamic spacing:

```css
:root {
  --container-padding: 16px;
  --card-padding: 16px;
  --section-gap: 16px;
}

@media (min-width: 768px) {
  :root {
    --container-padding: 24px;
    --card-padding: 20px;
    --section-gap: 24px;
  }
}

@media (min-width: 1024px) {
  :root {
    --container-padding: 32px;
    --card-padding: 24px;
    --section-gap: 32px;
  }
}
```

---

## Orientation Handling

```tsx
// Hook for orientation
function useOrientation() {
  const [isPortrait, setIsPortrait] = useState(
    typeof window !== "undefined"
      ? window.innerHeight > window.innerWidth
      : true
  );

  useEffect(() => {
    const handler = () => {
      setIsPortrait(window.innerHeight > window.innerWidth);
    };
    window.addEventListener("resize", handler);
    return () => window.removeEventListener("resize", handler);
  }, []);

  return isPortrait ? "portrait" : "landscape";
}
```

---

## Performance at Breakpoints

### Layout Thrashing

Avoid recalculating layout on every resize:

```tsx
// Debounce resize events
import { useDebouncedValue } from "@/hooks/use-debounced-value";

function useBreakpoint() {
  const [width, setWidth] = useState(
    typeof window !== "undefined" ? window.innerWidth : 1024
  );

  const debouncedWidth = useDebouncedValue(width, 100);

  useEffect(() => {
    const handler = () => setWidth(window.innerWidth);
    window.addEventListener("resize", handler);
    return () => window.removeEventListener("resize", handler);
  }, []);

  return {
    isMobile: debouncedWidth < 768,
    isTablet: debouncedWidth >= 768 && debouncedWidth < 1024,
    isDesktop: debouncedWidth >= 1024,
  };
}
```

---

## Summary: Priority Fixes

### Critical (Fix Immediately)

1. ~~Layout shift at md breakpoint~~ → Reserve sidebar space

### High Priority

2. Panel transition animations
3. Close mobile menu on resize to desktop
4. Form element sizing consistency

### Medium Priority

5. Fluid typography implementation
6. Smoother grid column transitions
7. Container max-width on large screens

### Low Priority

8. Breakpoint-specific CSS variables
9. Orientation handling improvements
10. Container query preparation

---

## Testing Tools

### Browser DevTools

- Chrome: Device toolbar, responsive mode
- Firefox: Responsive design mode
- Safari: Responsive design mode

### Recommended Devices to Test

| Category         | Device            | Width  |
| ---------------- | ----------------- | ------ |
| Small phone      | iPhone SE         | 320px  |
| Standard phone   | iPhone 14         | 390px  |
| Large phone      | iPhone 14 Pro Max | 430px  |
| Tablet portrait  | iPad              | 768px  |
| Tablet landscape | iPad              | 1024px |
| Laptop           | MacBook Air       | 1280px |
| Desktop          | iMac              | 1920px |

---

_Last updated: December 25, 2025_
