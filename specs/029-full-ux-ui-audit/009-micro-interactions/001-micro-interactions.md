# 001 — Micro-interactions & Animation Audit

**First Principle: Delight** — Precision in small things earns loyalty.

---

## Animation Inventory

### Defined Animations (globals.css / design-tokens.css)

```css
/* From design-tokens.css */
tw-animate-css provides:
- animate-pulse (skeleton loading)
- animate-spin (loader icons)

Custom likely defined:
- motion-safe:animate-fade-in-up (query empty state)
- motion-safe:animate-slide-in-right (user chat messages)
- motion-safe:animate-pulse-soft (query icon gradient)
```

### Component-Level Transitions

```typescript
// stats-card.tsx
'transition-all duration-200'
'hover:shadow-md hover:-translate-y-0.5'  // micro-lift on hover

// quick-actions.tsx
'transition-all duration-200'
'hover:border-primary/50 hover:shadow-md hover:-translate-y-0.5'

// sidebar nav items
'transition-all duration-150'  // faster for navigation feedback

// backend-status-banner — no explicit transition
```

---

## Issues

### MI-01 · Inconsistent Transition Duration

Duration values are scattered across components:

```
sidebar.tsx:     duration-150
stats-card.tsx:  duration-200
quick-actions:   duration-200
chat-message:    no explicit duration (uses default)
```

**Fix:** Standardize on the transition token system:

```css
/* design-tokens.css */
:root {
  --duration-instant: 50ms;
  --duration-fast: 100ms;    /* hover states, micro feedback */
  --duration-base: 150ms;    /* default interactions */
  --duration-moderate: 200ms; /* reveals, panels */
  --duration-slow: 300ms;    /* page transitions, overlays */
  --duration-crawl: 500ms;   /* loading animations */
  
  --ease-standard: cubic-bezier(0.4, 0, 0.2, 1);  /* Material Design standard */
  --ease-decelerate: cubic-bezier(0, 0, 0.2, 1);  /* Enter/appear */
  --ease-accelerate: cubic-bezier(0.4, 0, 1, 1);  /* Exit/disappear */
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1); /* Spring effect */
}
```

Then in Tailwind config:
```typescript
// tailwind.config.ts
theme: {
  extend: {
    transitionDuration: {
      instant: 'var(--duration-instant)',
      fast: 'var(--duration-fast)',
      base: 'var(--duration-base)',
      moderate: 'var(--duration-moderate)',
      slow: 'var(--duration-slow)',
    }
  }
}
```

### MI-02 · Hover `translate-y` on Cards is Overused

Both `StatsCard` and `QuickActions` use `hover:-translate-y-0.5` (2px lift). When multiple cards are in a grid, hovering any card lifts it — this can look mechanical when cards are adjacent.

**Better alternative for card hover:**
```css
/* Instead of translate-y, use shadow escalation + subtle bg tint */
.card-hover {
  transition: box-shadow var(--duration-base) var(--ease-standard),
              background-color var(--duration-base) var(--ease-standard);
}

.card-hover:hover {
  box-shadow: var(--shadow-md);
  background-color: oklch(0.99 0 0); /* barely visible tint */
}
```

The translate lift should be reserved for **clickable navigation cards** (like Quick Actions), not for display cards (like Stats Cards).

### MI-03 · No Active/Press State on Buttons

The current button variants use hover states but no explicit `active:` (press) state:

```typescript
// button.tsx — shadcn/ui default
// Typically has: hover:bg-primary/90
// Missing:       active:scale-[0.97] active:bg-primary/80
```

A subtle scale-down on press (`active:scale-[0.97]`) creates satisfying tactile feedback. This is visible in Linear, Vercel, and Apple's native buttons.

**Fix:**
```css
/* Add to button component or globals.css */
button, [role="button"] {
  &:active:not(:disabled) {
    transform: scale(0.97);
    transition-duration: var(--duration-instant);
  }
}
```

### MI-04 · Batch Action Bar Has No Entry Animation

When items are selected in the documents table, the batch action bar appears. Currently it likely appears/disappears without animation, creating an abrupt layout shift.

**Fix:**
```typescript
// batch-actions-bar.tsx
<div className={cn(
  "transition-all duration-base",
  isVisible 
    ? "opacity-100 translate-y-0" 
    : "opacity-0 -translate-y-2 pointer-events-none"
)}>
  {/* batch actions */}
</div>
```

### MI-05 · Chat Streaming: No Character-Level Animation

The chat streaming uses `isStreaming` flag but the character-by-character rendering could be enhanced with a **cursor blink** at the text insertion point during streaming.

```typescript
// Current: text appears instantly in chunks
// Better: Add blinking cursor to end of streaming content

{isStreaming && (
  <span 
    className="inline-block w-[2px] h-[1em] bg-current ml-0.5 align-middle motion-safe:animate-[blink_1s_step-end_infinite]"
    aria-hidden="true"
  />
)}
```

### MI-06 · Status Badge Spinning Animation

The spinning `Loader2` icon on processing documents (`animate: true` in statusConfig) is applied to all 10 in-progress states. This means the entire status column can have multiple spinning elements simultaneously, which is visually chaotic.

**Fix:** Use a **progress pulse** instead of continuous spin for most states:

```typescript
// Only spin for active AI processing states
const useSpin = ['extracting', 'gleaning', 'converting'].includes(status);
const useProgress = ['chunking', 'embedding', 'storing', 'preprocessing'].includes(status);

<Icon 
  className={cn(
    'h-3.5 w-3.5',
    useSpin && 'animate-spin',
    useProgress && 'animate-pulse'
  )} 
/>
```

### MI-07 · Focus Ring Animation

Current focus rings appear instantly (`focus-visible:ring-2`). A subtle fade-in on the focus ring is more polished:

```css
/* Add focus ring transition */
*:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--ring);
  transition: box-shadow var(--duration-fast) var(--ease-standard);
}
```

### MI-08 · Sidebar Collapse Animation

The sidebar collapse (`collapsed` state) changes content layout. If the transition is instantaneous, it creates jarring text→icon changes.

The content transition should:
1. Fade text out quickly
2. Then collapse the width with ease
3. Icons remain stable throughout

```typescript
// sidebar.tsx
<div className={cn(
  "transition-[width] duration-moderate ease-[var(--ease-standard)]",
  collapsed ? "w-[var(--sidebar-collapsed-width)]" : "w-[var(--sidebar-width)]"
)}>
```

---

## Accessibility: prefers-reduced-motion

The codebase already uses `motion-safe:` prefix for some animations (good!). This should be extended to ALL animation-related classes.

**Audit of motion-safe usage:**

```typescript
// Good (found in codebase):
"motion-safe:animate-fade-in-up"
"motion-safe:animate-slide-in-right"
"motion-safe:animate-pulse-soft"

// MISSING motion-safe on:
"animate-pulse" (skeleton loading)  → use motion-safe:animate-pulse
"animate-spin"  (loading icons)    → OK: loading spinners exempt
"hover:-translate-y-0.5"           → wrap in motion-safe:
```

**Global rule to add:**

```css
/* globals.css */
@media (prefers-reduced-motion: reduce) {
  *,
  ::before,
  ::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

---

## Animation Principles for EdgeQuake

```
1. PURPOSEFUL   → Every animation communicates state change
2. FAST         → UI interactions < 150ms (imperceptible)
3. RESPECTFUL   → Respect prefers-reduced-motion
4. CONSISTENT   → Same duration/easing for same category of interaction
5. SUBTLE       → 2px is enough; avoid dramatic 3D transforms
```

---

## Positive Findings

```
✅ motion-safe: prefix used in query empty state and chat messages
✅ backdrop-blur-sm on sticky header (polished effect)
✅ Skeleton animation (animate-pulse) on loading states
✅ Smooth theme switching (theme-switching class pattern)
✅ requestAnimationFrame for theme transition cleanup
✅ transition-all on nav items (150ms)
```

---

## External References

- [Material Design Motion Principles](https://m3.material.io/styles/motion/overview)
- [The Web Animations API — MDN](https://developer.mozilla.org/en-US/docs/Web/API/Web_Animations_API)
- [prefers-reduced-motion — CSS Tricks](https://css-tricks.com/introduction-reduced-motion-media-query/)
- [Designing with Motion — Stripe](https://stripe.com/blog/designing-with-motion)
- [Spring Animations in CSS — Emil Kowalski](https://animations.dev/)
