# Design Tokens Comparison

> **Document:** 07-design-tokens.md  
> **Last Updated:** 2025-12-30

---

## 1. Overview

This document compares design tokens between LightRAG and EdgeQuake Knowledge Graph UIs, identifying alignment opportunities for the EdgeQuake migration.

---

## 2. Color Systems

### 2.1 EdgeQuake - shadcn/ui Theme Tokens

```css
/* globals.css - CSS Variables */
:root {
  --background: 0 0% 100%;
  --foreground: 222.2 47.4% 11.2%;
  --muted: 210 40% 96.1%;
  --muted-foreground: 215.4 16.3% 46.9%;
  --popover: 0 0% 100%;
  --popover-foreground: 222.2 47.4% 11.2%;
  --border: 214.3 31.8% 91.4%;
  --input: 214.3 31.8% 91.4%;
  --card: 0 0% 100%;
  --card-foreground: 222.2 47.4% 11.2%;
  --primary: 222.2 47.4% 11.2%;
  --primary-foreground: 210 40% 98%;
  --secondary: 210 40% 96.1%;
  --secondary-foreground: 222.2 47.4% 11.2%;
  --accent: 210 40% 96.1%;
  --accent-foreground: 222.2 47.4% 11.2%;
  --destructive: 0 100% 50%;
  --destructive-foreground: 210 40% 98%;
  --ring: 215 20.2% 65.1%;
  --radius: 0.5rem;
}

.dark {
  --background: 224 71% 4%;
  --foreground: 213 31% 91%;
  /* ... other dark variants */
}
```

### 2.2 LightRAG - Tailwind Direct Colors

```typescript
// Uses Tailwind color utilities directly
// Examples from GraphViewer.tsx:
bg - gray - 900 / 70; // Overlay background
text - white; // Primary text
text - gray - 400; // Muted text
border - slate - 600; // Borders
```

### 2.3 Entity Type Colors (Both Systems)

| Entity Type  | EdgeQuake Hex | EdgeQuake Tailwind | LightRAG |
| ------------ | ------------- | ------------------ | -------- |
| PERSON       | #3b82f6       | blue-500           | #3b82f6  |
| ORGANIZATION | #10b981       | emerald-500        | #10b981  |
| LOCATION     | #f59e0b       | amber-500          | #f59e0b  |
| EVENT        | #ef4444       | red-500            | #ef4444  |
| CONCEPT      | #8b5cf6       | violet-500         | #8b5cf6  |
| DOCUMENT     | #6366f1       | indigo-500         | #6366f1  |
| DEFAULT      | #64748b       | slate-500          | #64748b  |

**Status:** ✅ Aligned - Same color palette

### 2.4 Edge Colors

| State        | EdgeQuake           | LightRAG           |
| ------------ | ------------------- | ------------------ |
| Default      | #94a3b8 (slate-400) | #6b7280 (gray-500) |
| Highlighted  | source node color   | source node color  |
| Relationship | weight-based alpha  | weight-based alpha |

---

## 3. Typography Tokens

### 3.1 Font Families

| Purpose      | EdgeQuake      | LightRAG       |
| ------------ | -------------- | -------------- |
| Body         | Inter          | Inter          |
| Monospace    | JetBrains Mono | JetBrains Mono |
| Graph Labels | Inter          | System default |

### 3.2 Font Sizes

| Token | EdgeQuake       | LightRAG | Notes       |
| ----- | --------------- | -------- | ----------- |
| xs    | 0.75rem (12px)  | 0.75rem  | Labels      |
| sm    | 0.875rem (14px) | 0.875rem | Secondary   |
| base  | 1rem (16px)     | 1rem     | Body        |
| lg    | 1.125rem (18px) | 1.125rem | Subheadings |
| xl    | 1.25rem (20px)  | 1.25rem  | Headings    |
| 2xl   | 1.5rem (24px)   | 1.5rem   | Titles      |

### 3.3 Graph Label Typography

```typescript
// EdgeQuake - graph-renderer.tsx
labelSize: 12,
labelWeight: '500',
labelFont: 'Inter, ui-sans-serif, system-ui',
labelColor: { color: '#374151' },  // Hardcoded!

// LightRAG - GraphViewer.tsx
labelFont: 'Inter, ui-sans-serif, system-ui, sans-serif',
labelSize: 12,
labelWeight: 'normal',
labelColor: {
  color: useDarkTheme ? '#e2e8f0' : '#1e293b',  // Theme-aware
},
```

**Gap:** EdgeQuake has hardcoded label colors, needs theme awareness.

---

## 4. Spacing Tokens

### 4.1 Tailwind Spacing Scale (Both)

| Token | Value          | Usage           |
| ----- | -------------- | --------------- |
| 0.5   | 0.125rem (2px) | Micro gaps      |
| 1     | 0.25rem (4px)  | Dense UI        |
| 2     | 0.5rem (8px)   | Compact spacing |
| 3     | 0.75rem (12px) | Default gap     |
| 4     | 1rem (16px)    | Section spacing |
| 6     | 1.5rem (24px)  | Large gaps      |
| 8     | 2rem (32px)    | Containers      |

### 4.2 Panel Dimensions

| Panel           | EdgeQuake     | LightRAG    | Notes            |
| --------------- | ------------- | ----------- | ---------------- |
| Entity Browser  | 256px (w-64)  | N/A         | EdgeQuake unique |
| Details Panel   | 320px default | N/A         | EdgeQuake unique |
| Details Min     | 280px         | N/A         | EdgeQuake        |
| Details Max     | 480px         | N/A         | EdgeQuake        |
| Controls Height | 40px          | 32px        | Toolbar height   |
| Search Input    | h-8 (32px)    | h-9 (36px)  | Input fields     |
| Toolbar Gap     | 8px (gap-2)   | 4px (gap-1) | Button spacing   |

---

## 5. Border Radius Tokens

| Token   | EdgeQuake      | LightRAG | Notes            |
| ------- | -------------- | -------- | ---------------- |
| none    | 0              | 0        | Sharp corners    |
| sm      | 0.125rem (2px) | 2px      | Subtle rounding  |
| default | 0.25rem (4px)  | 4px      | Buttons, inputs  |
| md      | 0.375rem (6px) | 6px      | Cards            |
| lg      | 0.5rem (8px)   | 8px      | Modals, panels   |
| xl      | 0.75rem (12px) | 12px     | Large containers |
| full    | 9999px         | 9999px   | Pills, avatars   |

**Status:** ✅ Aligned - Both use Tailwind defaults

---

## 6. Shadow Tokens

### 6.1 EdgeQuake Shadows

```css
/* From globals.css */
--shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
--shadow: 0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1);
--shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1);
--shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
```

### 6.2 Usage Comparison

| Component    | EdgeQuake | LightRAG   |
| ------------ | --------- | ---------- |
| Panel        | shadow-lg | shadow-xl  |
| Dropdown     | shadow-md | shadow-lg  |
| Button hover | shadow-sm | none       |
| Tooltip      | shadow    | shadow-md  |
| Modal        | shadow-xl | shadow-2xl |

---

## 7. Animation Tokens

### 7.1 Duration

| Token   | EdgeQuake | LightRAG | Usage          |
| ------- | --------- | -------- | -------------- |
| instant | 0ms       | 0ms      | Immediate      |
| fast    | 150ms     | 100ms    | Hover states   |
| normal  | 200ms     | 200ms    | Transitions    |
| slow    | 300ms     | 300ms    | Layout changes |
| slower  | 500ms     | 400ms    | Camera pans    |

### 7.2 Easing

| Token       | EdgeQuake                               | LightRAG | Notes   |
| ----------- | --------------------------------------- | -------- | ------- |
| ease-in-out | cubic-bezier(0.4, 0, 0.2, 1)            | Same     | Default |
| ease-out    | cubic-bezier(0, 0, 0.2, 1)              | Same     | Exit    |
| ease-in     | cubic-bezier(0.4, 0, 1, 1)              | Same     | Enter   |
| spring      | cubic-bezier(0.175, 0.885, 0.32, 1.275) | N/A      | Bounce  |

### 7.3 Graph Animation Tokens

```typescript
// EdgeQuake - Missing explicit tokens
// LightRAG - Defined in LayoutsControl.tsx
const LAYOUT_ANIMATION_DURATION = 300;
const CAMERA_PAN_DURATION = 500;
const HOVER_HIGHLIGHT_DURATION = 150;
```

**Gap:** EdgeQuake needs explicit animation tokens for graph interactions.

---

## 8. Z-Index Tokens

| Layer    | EdgeQuake | LightRAG | Notes          |
| -------- | --------- | -------- | -------------- |
| base     | 0         | 0        | Content        |
| dropdown | 10        | 10       | Menus          |
| sticky   | 20        | 20       | Headers        |
| fixed    | 30        | 30       | Fixed elements |
| overlay  | 40        | 40       | Overlays       |
| modal    | 50        | 50       | Dialogs        |
| popover  | 60        | 60       | Popovers       |
| tooltip  | 70        | 70       | Tooltips       |

**Status:** ✅ Aligned

---

## 9. Breakpoint Tokens

| Breakpoint | EdgeQuake | LightRAG | Notes            |
| ---------- | --------- | -------- | ---------------- |
| sm         | 640px     | 640px    | Mobile landscape |
| md         | 768px     | 768px    | Tablet           |
| lg         | 1024px    | 1024px   | Desktop          |
| xl         | 1280px    | 1280px   | Large desktop    |
| 2xl        | 1536px    | 1536px   | Wide screens     |

**Status:** ✅ Aligned - Standard Tailwind breakpoints

**Gap:** EdgeQuake doesn't USE breakpoints for responsive behavior (see P0 bug).

---

## 10. Icon Sizing Tokens

| Size | EdgeQuake      | LightRAG | Usage           |
| ---- | -------------- | -------- | --------------- |
| xs   | h-3 w-3 (12px) | 12px     | Dense UI        |
| sm   | h-4 w-4 (16px) | 16px     | Default buttons |
| md   | h-5 w-5 (20px) | 20px     | Emphasis        |
| lg   | h-6 w-6 (24px) | 24px     | Headers         |
| xl   | h-8 w-8 (32px) | 32px     | Featured        |

**Status:** ✅ Aligned

---

## 11. Component-Specific Tokens

### 11.1 Node Rendering

| Property        | EdgeQuake | LightRAG | Notes       |
| --------------- | --------- | -------- | ----------- |
| minNodeSize     | 4         | 3        | Min radius  |
| maxNodeSize     | 20        | 20       | Max radius  |
| defaultNodeSize | 8         | 5        | Default     |
| borderWidth     | 0         | 2        | Node border |
| borderColor     | N/A       | #ffffff  | Border      |

### 11.2 Edge Rendering

| Property        | EdgeQuake    | LightRAG | Notes         |
| --------------- | ------------ | -------- | ------------- |
| minEdgeSize     | 1            | 0.5      | Min thickness |
| maxEdgeSize     | 5            | 3        | Max thickness |
| defaultEdgeSize | 2            | 1        | Default       |
| curvature       | 0 (straight) | 0.25     | Curve amount  |

### 11.3 Label Rendering

| Property                   | EdgeQuake | LightRAG | Notes                |
| -------------------------- | --------- | -------- | -------------------- |
| labelGridCellSize          | 50        | 60       | Overlap prevention   |
| labelRenderedSizeThreshold | 10        | 12       | Hide when zoomed out |
| labelDensity               | 0.07      | 0.1      | Label coverage       |

---

## 12. Token Consolidation Recommendations

### 12.1 Create Theme-Aware Graph Tokens

```typescript
// lib/graph-tokens.ts
import { useTheme } from "next-themes";

export function useGraphTokens() {
  const { theme } = useTheme();
  const isDark = theme === "dark";

  return {
    // Node tokens
    nodeDefaults: {
      minSize: 4,
      maxSize: 20,
      defaultSize: 8,
      borderWidth: 2,
      borderColor: isDark ? "#374151" : "#ffffff",
    },

    // Edge tokens
    edgeDefaults: {
      minSize: 1,
      maxSize: 5,
      defaultSize: 2,
      color: isDark ? "#4b5563" : "#94a3b8",
      curvature: 0.25,
    },

    // Label tokens
    labelDefaults: {
      font: "Inter, ui-sans-serif, system-ui",
      size: 12,
      weight: "500",
      color: isDark ? "#e2e8f0" : "#374151",
      gridCellSize: 60,
      density: 0.1,
    },

    // Animation tokens
    animation: {
      layout: 300,
      camera: 500,
      hover: 150,
      easing: "quadraticInOut" as const,
    },

    // Colors
    colors: {
      highlight: isDark ? "#fbbf24" : "#f59e0b",
      selected: isDark ? "#60a5fa" : "#3b82f6",
      muted: isDark ? "#4b5563" : "#9ca3af",
    },
  };
}
```

### 12.2 Migrate Hardcoded Values

```typescript
// Before (current EdgeQuake)
labelColor: { color: '#374151' },

// After (with tokens)
const tokens = useGraphTokens();
labelColor: { color: tokens.labelDefaults.color },
```

---

## 13. Summary

### Aligned Tokens (No Changes Needed)

- ✅ Entity type colors
- ✅ Tailwind spacing scale
- ✅ Border radius
- ✅ Z-index layers
- ✅ Breakpoints (definition, not usage)
- ✅ Icon sizes

### Gaps to Address

- ⚠️ Label colors (hardcoded vs theme-aware)
- ⚠️ Animation durations (no central definition)
- ⚠️ Node border styles (missing in EdgeQuake)
- ⚠️ Edge curvature (straight vs curved)
- ⚠️ Responsive breakpoint usage (P0 bug)

### Recommended Actions

1. Create `lib/graph-tokens.ts` with centralized tokens
2. Add theme-aware label colors
3. Add explicit animation duration tokens
4. Import node/edge program tokens from LightRAG
5. Use breakpoints for responsive panel behavior

---

_Related Documents:_

- [02-architecture-comparison.md](./02-architecture-comparison.md)
- [03-visual-interaction-audit.md](./03-visual-interaction-audit.md)
- [06-recommendations-roadmap.md](./06-recommendations-roadmap.md)
