# Audit: Design Tokens & System

**Files:** `src/app/design-tokens.css`, `src/app/globals.css`

---

## Findings

### F-DS-01 · Color palette is purely achromatic in globals.css · MED
**Problem:** All primary colors are `oklch(x 0 0)` — no chroma. The chart colors and semantic accents are defined separately but inconsistently applied.  
```css
--primary: oklch(0.205 0 0);       /* pure black/grey */
--secondary: oklch(0.97 0 0);      /* near-white */
```
**Fix:** Introduce a single brand accent color with chroma (e.g., a blue or indigo hue) as `--brand` and map `--primary` to use it where appropriate.

### F-DS-02 · Quick Actions use hardcoded Tailwind color classes instead of tokens · MED
**Problem:** `bg-blue-500/10`, `text-purple-500` etc. in `quick-actions.tsx` bypass the token system entirely.

### F-DS-03 · Design tokens file is 200+ lines but no usage documentation · LOW
**Problem:** The `design-tokens.css` file defines many tokens (chat, code, spacing) but components rarely reference them directly — they use Tailwind classes instead.

### F-DS-04 · Dark mode ring token is too low contrast · MED
**Problem:** `--ring: oklch(0.556 0 0)` in dark mode. At this lightness on dark backgrounds, focus rings may not meet WCAG 2.4.7 (3:1 ratio).

### F-DS-05 · Typography scale not documented · LOW
**Problem:** No explicit type scale tokens — `text-xs`, `text-sm`, `text-base`, `text-lg` etc. are used ad-hoc across components.

---

## Recommendations

1. Add a single brand hue to the color system
2. Map Quick Action colors to semantic design tokens
3. Document token usage in `design-tokens.css` comments
4. Add `--type-*` scale tokens for consistent typography
