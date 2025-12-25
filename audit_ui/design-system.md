# EdgeQuake Design System Tokens & Patterns

## Design System Tokens

This document defines the visual language and reusable patterns for the EdgeQuake UI. All components should reference these tokens for consistency.

---

## 1. Typography Scale

Based on analysis of current implementation and best practices:

### Type Scale Tokens

```css
/* globals.css or design-tokens.css */
:root {
  /* Base */
  --font-size-xs: 0.6875rem;    /* 11px */
  --font-size-sm: 0.75rem;      /* 12px */
  --font-size-base: 0.875rem;   /* 14px - body text */
  --font-size-md: 1rem;         /* 16px */
  --font-size-lg: 1.125rem;     /* 18px */
  --font-size-xl: 1.25rem;      /* 20px */
  --font-size-2xl: 1.5rem;      /* 24px */
  --font-size-3xl: 1.875rem;    /* 30px - page titles */
  --font-size-4xl: 2.25rem;     /* 36px - hero text */
  
  /* Line Heights */
  --line-height-tight: 1.25;
  --line-height-snug: 1.375;
  --line-height-normal: 1.5;
  --line-height-relaxed: 1.625;
  --line-height-loose: 2;
  
  /* Font Weights */
  --font-weight-normal: 400;
  --font-weight-medium: 500;
  --font-weight-semibold: 600;
  --font-weight-bold: 700;
  
  /* Letter Spacing */
  --letter-spacing-tight: -0.025em;
  --letter-spacing-normal: 0;
  --letter-spacing-wide: 0.025em;
}
```

### Typography Usage Map

```tsx
// Page Titles (H1)
className="text-3xl font-bold tracking-tight"
// font-size: 30px, font-weight: 700, letter-spacing: -0.025em

// Section Headers (H2)
className="text-xl font-semibold tracking-tight"
// font-size: 20px, font-weight: 600

// Subsection Headers (H3)
className="text-lg font-medium"
// font-size: 18px, font-weight: 500

// Card Titles / Labels (H4)
className="text-base font-medium"
// font-size: 16px, font-weight: 500

// Body Text
className="text-base"
// font-size: 14px (base)

// Secondary/Helper Text
className="text-sm text-muted-foreground"
// font-size: 12px

// Captions / Micro Copy
className="text-xs text-muted-foreground"
// font-size: 11px

// Large Stats / Numbers
className="text-4xl font-bold tabular-nums"
// font-size: 36px, font-weight: 700, monospace numerals
```

---

## 2. Spacing Scale

Progressive spacing system for consistent layouts:

### Spacing Tokens

```css
:root {
  /* Spacing scale (4px base unit) */
  --spacing-0: 0;
  --spacing-1: 0.25rem;   /* 4px */
  --spacing-2: 0.5rem;    /* 8px */
  --spacing-3: 0.75rem;   /* 12px */
  --spacing-4: 1rem;      /* 16px */
  --spacing-5: 1.25rem;   /* 20px */
  --spacing-6: 1.5rem;    /* 24px */
  --spacing-8: 2rem;      /* 32px */
  --spacing-10: 2.5rem;   /* 40px */
  --spacing-12: 3rem;     /* 48px */
  --spacing-16: 4rem;     /* 64px */
  --spacing-20: 5rem;     /* 80px */
  --spacing-24: 6rem;     /* 96px */
}
```

### Spacing Usage Guidelines

```tsx
// Component padding/gaps
'p-2'     // Very tight (8px) - badges, pills
'p-3'     // Tight (12px) - buttons, form inputs
'p-4'     // Comfortable (16px) - cards, panels
'p-6'     // Spacious (24px) - page containers

// Between related items
'gap-2'   // 8px - icon + text, inline elements
'gap-3'   // 12px - form fields in a group
'gap-4'   // 16px - cards in a grid, list items

// Between sections
'space-y-4'  // 16px - within a section
'space-y-6'  // 24px - default section spacing
'space-y-8'  // 32px - major section breaks

// Margins for separation
'mt-2'    // 8px - subtle separation
'mt-4'    // 16px - related content
'mt-6'    // 24px - unrelated content
'mt-8'    // 32px - major breaks
```

### Layout Rhythm Example

```tsx
<div className="p-6 space-y-8">  {/* Page container */}
  <div className="space-y-2">    {/* Header group */}
    <h1 className="text-3xl font-bold">Title</h1>
    <p className="text-sm text-muted-foreground">Description</p>
  </div>
  
  <div className="grid gap-4">   {/* Stats cards */}
    <StatsCard />
    <StatsCard />
  </div>
  
  <div className="space-y-4 mt-8">  {/* New section with gap */}
    <h2 className="text-xl font-semibold">Section Title</h2>
    <div className="grid gap-4">
      {/* Content */}
    </div>
  </div>
</div>
```

---

## 3. Layout Tokens

### Panel Widths

```css
:root {
  /* Sidebar */
  --sidebar-width-collapsed: 4.5rem;    /* 72px */
  --sidebar-width-expanded: 16rem;       /* 256px */
  
  /* Right Panels */
  --right-panel-narrow: 20rem;           /* 320px - Dashboard insights */
  --right-panel-wide: 25rem;             /* 400px - Document preview, Query sources */
  
  /* Content */
  --content-max-width: 80rem;            /* 1280px - prevents text from being too wide */
  --chat-message-max-width: 85%;         /* 85% - messages shouldn't span full width */
  
  /* Modals/Dialogs */
  --dialog-sm: 28rem;    /* 448px */
  --dialog-md: 32rem;    /* 512px */
  --dialog-lg: 48rem;    /* 768px */
  --dialog-xl: 56rem;    /* 896px */
}
```

### Breakpoints

```css
:root {
  --breakpoint-sm: 640px;   /* Mobile landscape */
  --breakpoint-md: 768px;   /* Tablet portrait */
  --breakpoint-lg: 1024px;  /* Tablet landscape / Small laptop */
  --breakpoint-xl: 1280px;  /* Desktop */
  --breakpoint-2xl: 1536px; /* Large desktop */
}
```

### Responsive Behavior

```tsx
// Sidebar
// >= 1024px: Expanded by default, collapsible
// < 1024px: Mobile drawer

// Right Panel
// >= 1280px: 400px fixed width
// 1024px - 1279px: 320px fixed width
// < 1024px: Bottom sheet (mobile)

// Grid Layouts
// >= 1280px: 4 columns (stats cards)
// 768px - 1279px: 2 columns
// < 768px: 1 column

// Table Columns
// >= 1024px: All columns visible
// 768px - 1023px: Hide less important columns
// < 768px: Minimal columns + detail view
```

---

## 4. Color System

Based on `globals.css` analysis:

### Semantic Color Tokens

```css
:root {
  /* Backgrounds */
  --background: oklch(1 0 0);              /* Pure white */
  --card: oklch(1 0 0);                    /* Card background */
  --popover: oklch(1 0 0);                 /* Popover background */
  --muted: oklch(0.97 0 0);                /* Muted background (gray-100) */
  
  /* Foregrounds */
  --foreground: oklch(0.145 0 0);          /* Primary text (gray-900) */
  --muted-foreground: oklch(0.556 0 0);    /* Secondary text (gray-600) */
  --card-foreground: oklch(0.145 0 0);     /* Text on cards */
  
  /* Interactive */
  --primary: oklch(0.205 0 0);             /* Primary action color */
  --primary-foreground: oklch(0.985 0 0);  /* Text on primary */
  --secondary: oklch(0.97 0 0);            /* Secondary action */
  --accent: oklch(0.97 0 0);               /* Accent elements */
  
  /* Feedback */
  --destructive: oklch(0.577 0.245 27.325);/* Error/delete actions */
  --success: oklch(0.646 0.222 41.116);    /* Success states */
  --warning: oklch(0.828 0.189 84.429);    /* Warning states */
  --info: oklch(0.6 0.118 184.704);        /* Info states */
  
  /* Borders */
  --border: oklch(0.922 0 0);              /* Default border (gray-200) */
  --input: oklch(0.922 0 0);               /* Input borders */
  --ring: oklch(0.708 0 0);                /* Focus ring */
}

.dark {
  --background: oklch(0.145 0 0);          /* Dark gray */
  --foreground: oklch(0.985 0 0);          /* Light text */
  --card: oklch(0.205 0 0);                /* Slightly lighter */
  --muted: oklch(0.269 0 0);               /* Muted dark */
  --muted-foreground: oklch(0.708 0 0);    /* Gray-400 */
  --primary: oklch(0.922 0 0);             /* Light primary */
  --border: oklch(1 0 0 / 10%);            /* Subtle borders */
  /* ... other dark mode colors */
}
```

### Color Usage Guidelines

```tsx
// Backgrounds
'bg-background'        // Page background
'bg-card'              // Card surfaces
'bg-muted'             // Subtle backgrounds
'bg-muted/50'          // Hover states (50% opacity)

// Text
'text-foreground'      // Primary text
'text-muted-foreground'// Secondary text, labels
'text-sm text-muted-foreground' // Captions

// Borders
'border'               // Standard borders
'border-2'             // Emphasized borders
'border-dashed'        // Dropzones
'border-primary'       // Active/selected states

// Status Colors
'bg-destructive'       // Error backgrounds
'text-destructive'     // Error text
'bg-green-500'         // Success (if not using semantic token)
'bg-yellow-500'        // Warning
'bg-blue-500'          // Info
```

---

## 5. Component Patterns

### Button Variants

```tsx
// Primary Action
<Button className="gap-2">
  <Icon className="h-4 w-4" />
  Primary Action
</Button>

// Secondary Action
<Button variant="outline" className="gap-2">
  <Icon className="h-4 w-4" />
  Secondary
</Button>

// Ghost/Tertiary
<Button variant="ghost" size="sm">
  <Icon className="h-4 w-4" />
</Button>

// Destructive
<Button variant="destructive">Delete</Button>

// Icon Only
<Button variant="ghost" size="sm" aria-label="Close">
  <X className="h-4 w-4" />
</Button>
```

### Card Patterns

```tsx
// Standard Card
<Card>
  <CardHeader>
    <CardTitle>Title</CardTitle>
    <CardDescription>Description text</CardDescription>
  </CardHeader>
  <CardContent>
    {/* Content */}
  </CardContent>
  <CardFooter>
    <Button>Action</Button>
  </CardFooter>
</Card>

// Stats Card
<Card>
  <CardHeader className="pb-2">
    <div className="flex items-center justify-between">
      <CardTitle className="text-sm font-medium text-muted-foreground">
        Label
      </CardTitle>
      <Icon className="h-4 w-4 text-muted-foreground" />
    </div>
  </CardHeader>
  <CardContent>
    <div className="text-4xl font-bold tabular-nums">42</div>
    <p className="text-xs text-muted-foreground mt-1">
      Description
    </p>
  </CardContent>
</Card>

// Clickable/Interactive Card
<Card className="cursor-pointer hover:border-primary hover:bg-primary/5 transition-colors">
  <CardContent className="p-4">
    {/* Content */}
  </CardContent>
</Card>
```

### Table Patterns

```tsx
<Table>
  <TableHeader>
    <TableRow>
      <TableHead className="w-[40px]">
        <Checkbox aria-label="Select all" />
      </TableHead>
      <TableHead className="w-[40%]">Title</TableHead>
      <TableHead className="w-[120px]">Status</TableHead>
      <TableHead className="w-[100px]">Size</TableHead>
      <TableHead className="w-[140px]">Date</TableHead>
      <TableHead className="w-[100px] text-right">Actions</TableHead>
    </TableRow>
  </TableHeader>
  <TableBody>
    <TableRow className="hover:bg-muted/50">
      <TableCell>
        <Checkbox aria-label="Select row" />
      </TableCell>
      <TableCell className="font-medium">
        {/* Title with truncation */}
        <div className="flex items-center gap-2">
          <FileIcon className="h-4 w-4 text-muted-foreground" />
          <span className="truncate">{title}</span>
        </div>
      </TableCell>
      <TableCell>
        <Badge variant="outline">Status</Badge>
      </TableCell>
      <TableCell className="text-sm text-muted-foreground">
        {size}
      </TableCell>
      <TableCell className="text-sm text-muted-foreground">
        {date}
      </TableCell>
      <TableCell className="text-right">
        {/* Action buttons */}
      </TableCell>
    </TableRow>
  </TableBody>
</Table>
```

### Form Patterns

```tsx
// Standard Form Field
<div className="space-y-2">
  <Label htmlFor="field">Field Label</Label>
  <Input 
    id="field" 
    placeholder="Placeholder text"
    aria-describedby="field-description"
  />
  <p id="field-description" className="text-xs text-muted-foreground">
    Helper text
  </p>
</div>

// Form Field with Error
<div className="space-y-2">
  <Label htmlFor="field" className="text-destructive">
    Field Label
  </Label>
  <Input 
    id="field" 
    className="border-destructive"
    aria-invalid="true"
    aria-describedby="field-error"
  />
  <p id="field-error" className="text-xs text-destructive flex items-center gap-1">
    <AlertCircle className="h-3 w-3" />
    Error message
  </p>
</div>

// Slider with Label
<div className="space-y-2">
  <div className="flex items-center justify-between">
    <Label>Temperature</Label>
    <span className="text-sm text-muted-foreground">{value}</span>
  </div>
  <Slider 
    value={[value]}
    onValueChange={([v]) => setValue(v)}
    min={0}
    max={1}
    step={0.1}
  />
</div>
```

### Empty State Patterns

```tsx
// Large Empty State (Dashboard, full-page)
<div className="flex-1 flex items-center justify-center">
  <div className="max-w-md mx-auto px-6 text-center space-y-6">
    <div className="inline-flex h-16 w-16 items-center justify-center rounded-2xl bg-primary/10">
      <Icon className="h-8 w-8 text-primary" />
    </div>
    <div className="space-y-2">
      <h3 className="text-2xl font-semibold">No Items Yet</h3>
      <p className="text-sm text-muted-foreground">
        Get started by creating your first item.
      </p>
    </div>
    <Button size="lg" className="gap-2">
      <Plus className="h-4 w-4" />
      Create Item
    </Button>
  </div>
</div>

// Small Empty State (Section, card)
<Card>
  <CardContent className="flex flex-col items-center justify-center py-12">
    <Icon className="h-12 w-12 text-muted-foreground mb-4" />
    <h4 className="font-medium mb-1">No Items</h4>
    <p className="text-sm text-muted-foreground text-center mb-4">
      You haven't added any items yet.
    </p>
    <Button variant="outline" size="sm">Add Item</Button>
  </CardContent>
</Card>
```

### Loading State Patterns

```tsx
// Skeleton Loading
<Card>
  <CardHeader>
    <Skeleton className="h-5 w-32" />
    <Skeleton className="h-4 w-48 mt-2" />
  </CardHeader>
  <CardContent className="space-y-3">
    <Skeleton className="h-4 w-full" />
    <Skeleton className="h-4 w-3/4" />
    <Skeleton className="h-4 w-5/6" />
  </CardContent>
</Card>

// Inline Loading
<Button disabled>
  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
  Loading...
</Button>

// Progress Indicator
<div className="space-y-2">
  <div className="flex items-center justify-between text-sm">
    <span>Processing...</span>
    <span className="text-muted-foreground">67%</span>
  </div>
  <Progress value={67} />
</div>
```

### Toast/Notification Patterns

```tsx
// Success Toast
toast.success("Action completed successfully", {
  description: "Your changes have been saved.",
});

// Error Toast
toast.error("Something went wrong", {
  description: "Please try again or contact support.",
  action: {
    label: "Retry",
    onClick: () => retryAction(),
  },
});

// Loading Toast
const toastId = toast.loading("Uploading files...", {
  duration: Infinity,
});
// Later...
toast.success("Upload complete!", { id: toastId });
```

---

## 6. Animation Tokens

```css
:root {
  /* Durations */
  --duration-fast: 100ms;
  --duration-normal: 200ms;
  --duration-slow: 300ms;
  --duration-slower: 500ms;
  
  /* Easings */
  --ease-in: cubic-bezier(0.4, 0, 1, 1);
  --ease-out: cubic-bezier(0, 0, 0.2, 1);
  --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-spring: cubic-bezier(0.68, -0.55, 0.265, 1.55);
}
```

### Animation Usage

```tsx
// Hover transitions
'transition-colors duration-200'          // Button hover
'transition-all duration-200'             // Layout changes

// Panel slide
'transition-transform duration-200 ease-out'

// Fade in/out
'transition-opacity duration-300'

// Scale on click
'active:scale-95 transition-transform duration-100'

// Shimmer effect (loading)
'animate-shimmer'  // Custom animation in tailwind.config
```

---

## 7. Icon Guidelines

### Icon Sizes

```tsx
// Extra small (badges, inline text)
<Icon className="h-3 w-3" />    // 12px

// Small (buttons, form inputs)
<Icon className="h-4 w-4" />    // 16px

// Medium (headers, nav items)
<Icon className="h-5 w-5" />    // 20px

// Large (empty states, hero)
<Icon className="h-6 w-6" />    // 24px
<Icon className="h-8 w-8" />    // 32px

// Icon with text
<Button className="gap-2">
  <Icon className="h-4 w-4" />
  Button Text
</Button>
```

### Icon Colors

```tsx
// Primary icons
'text-foreground'

// Secondary/muted icons
'text-muted-foreground'

// Interactive icons (buttons, links)
'text-primary hover:text-primary/80'

// Status icons
'text-destructive'  // Error
'text-green-500'    // Success
'text-yellow-500'   // Warning
'text-blue-500'     // Info
```

---

## 8. Accessibility Tokens

```css
:root {
  /* Focus ring */
  --focus-ring-width: 2px;
  --focus-ring-offset: 2px;
  --focus-ring-color: var(--primary);
  
  /* Touch targets */
  --touch-target-min: 44px;  /* WCAG AAA: 44x44px */
}
```

### Accessibility Patterns

```tsx
// Focus visible
'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2'

// Skip link
<a 
  href="#main-content"
  className="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-50 focus:px-4 focus:py-2 focus:bg-primary focus:text-primary-foreground focus:rounded-lg"
>
  Skip to main content
</a>

// Screen reader only text
<span className="sr-only">Screen reader only text</span>

// Aria labels
<Button aria-label="Close dialog">
  <X className="h-4 w-4" />
</Button>

// Live regions
<div 
  role="status" 
  aria-live="polite" 
  aria-atomic="true"
  className="sr-only"
>
  {statusMessage}
</div>
```

---

## 9. Z-Index Scale

```css
:root {
  --z-base: 0;
  --z-dropdown: 1000;
  --z-sticky: 1020;
  --z-fixed: 1030;
  --z-modal-backdrop: 1040;
  --z-modal: 1050;
  --z-popover: 1060;
  --z-tooltip: 1070;
  --z-notification: 1080;
}
```

Usage:
- Base content: `z-0`
- Dropdowns: `z-[1000]`
- Sticky elements: `z-[1020]`
- Modals: `z-50` (Tailwind default, or use custom z-[1050])
- Toasts/Notifications: `z-[1080]`

---

## 10. Standardized Component Checklist

When creating any new component, ensure:

### ✅ Visual
- [ ] Uses design system tokens (no hardcoded values)
- [ ] Consistent spacing (8px, 16px, 24px, 32px)
- [ ] Proper typography scale
- [ ] Theme-aware colors
- [ ] Responsive breakpoints

### ✅ Interaction
- [ ] Hover states defined
- [ ] Active/pressed states defined
- [ ] Focus visible ring (keyboard navigation)
- [ ] Loading states
- [ ] Error states
- [ ] Empty states
- [ ] Disabled states

### ✅ Accessibility
- [ ] Semantic HTML elements
- [ ] ARIA labels where needed
- [ ] Keyboard navigable
- [ ] Focus management
- [ ] Color contrast WCAG AA (4.5:1)
- [ ] Touch target minimum 44x44px
- [ ] Screen reader tested

### ✅ Performance
- [ ] Lazy loaded if large
- [ ] Debounced inputs where applicable
- [ ] Optimistic UI updates
- [ ] Virtualized lists (if > 100 items)

---

## Usage Examples

### Example: Creating a New Stats Card

```tsx
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { TrendingUp } from 'lucide-react';

export function StatsCard({ title, value, description, icon: Icon, trend }) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium text-muted-foreground">
            {title}
          </CardTitle>
          {Icon && <Icon className="h-4 w-4 text-muted-foreground" aria-hidden="true" />}
        </div>
      </CardHeader>
      <CardContent>
        {/* Large value - uses tabular-nums for alignment */}
        <div className="text-4xl font-bold tabular-nums">
          {value}
        </div>
        
        {/* Description and trend */}
        <div className="flex items-center gap-2 mt-1">
          <p className="text-xs text-muted-foreground">
            {description}
          </p>
          {trend && (
            <div className="flex items-center gap-1 text-xs text-green-500">
              <TrendingUp className="h-3 w-3" />
              <span>+12%</span>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
```

### Example: Creating a Responsive Layout

```tsx
export function DashboardLayout({ children }) {
  return (
    <div className="flex h-screen">
      {/* Sidebar - collapses on mobile */}
      <Sidebar className="hidden lg:flex w-64 xl:w-72" />
      
      {/* Main content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        <Header />
        
        <main className="flex-1 overflow-auto">
          <div className="p-4 md:p-6 lg:p-8 space-y-6 md:space-y-8">
            {children}
          </div>
        </main>
      </div>
      
      {/* Right panel - converts to bottom sheet on mobile */}
      <RightPanel className="hidden xl:flex w-80 2xl:w-96" />
    </div>
  );
}
```

---

## Implementation Priority

1. **Phase 1:** Typography + Spacing tokens (1-2 days)
2. **Phase 2:** Component patterns (Card, Button, Form) (2-3 days)
3. **Phase 3:** Layout tokens + responsive (1-2 days)
4. **Phase 4:** Animation + Accessibility audit (2-3 days)
5. **Phase 5:** Documentation + Storybook examples (2-3 days)

Total: 8-13 days for complete design system implementation.

---

## References

- Current implementation: [`src/app/globals.css`](../edgequake_webui/src/app/globals.css)
- Component library: [`src/components/ui/`](../edgequake_webui/src/components/ui/)
- Tailwind config: [`tailwind.config.ts`](../edgequake_webui/tailwind.config.ts)
