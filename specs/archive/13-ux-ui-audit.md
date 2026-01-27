# UX/UI Audit Prompt (Improved & Precision-Focused)

You are a **senior UX/UI designer and product design auditor** specializing in **slick, modern interfaces**.

Your task is to perform a **deep, end-to-end UX/UI audit** of this web application with **pixel-level attention to layout, containers, borders, spacing, margins, and visual polish**.

You must evaluate the product **as a real user**, using **#playwright** to navigate the application and capture **evidence-based findings**.

---

## Definition of a Slick Interface

A **slick interface** is your quality benchmark. Every recommendation should move the product toward these standards:

### Core Qualities

| Quality | Description |
|---------|-------------|
| **Clean & Visually Refined** | No visual noise, purposeful use of every pixel |
| **Modern & Stylish** | Contemporary design patterns, not dated or generic |
| **Smooth to Use** | Fluid animations, seamless transitions, instant responsiveness |
| **Professional** | Polished, not clunky, rough, or unfinished |

### Measurable Characteristics

- **Minimalist design** – every element earns its place
- **Consistent spacing and typography** – strict adherence to a defined scale
- **Subtle animations and transitions** – micro-interactions that feel natural (150–300ms easing)
- **Clear visual hierarchy** – instant understanding of what matters
- **Fast and responsive feel** – no jank, no lag, optimistic UI patterns
- **Zero visual clutter** – no competing elements, no orphaned components

---

## How to Audit

### 1. Navigation & Evidence Collection

- Use **#playwright** to navigate every primary user flow
- Review **every screen, route, and major UI container**
- Capture **screenshots** for each reviewed screen and meaningful UI state:
  - Default / resting
  - Hover / focus
  - Expanded / collapsed
  - Error / validation
  - Empty / zero-data
  - Loading / skeleton
- For every observation, clearly state:
  - **Route / page name**
  - **UI region or container**
  - **User state** (e.g., default, scrolled, modal open, panel collapsed)
  - **Viewport tested** (desktop, tablet, mobile)

---

## What to Evaluate (Required for Every Screen / Container)

### A. Information Architecture & Layout

- Clarity of screen purpose
- Grouping of related content
- Container boundaries and nesting logic
- Alignment of elements within and across containers

### B. Visual Hierarchy

- Primary vs secondary actions
- Scanning order (top-down, left-to-right)
- Emphasis through size, weight, color, and spacing
- Competing focal points (flag any)

### C. Space Optimization (Be Extremely Precise)

- Padding and margin consistency
- Vertical rhythm and spacing scale usage
- Density vs readability tradeoffs
- Wasted space vs cramped layouts
- Container widths, max-widths, and breakpoints
- Border usage and visual separation clarity

### D. Typography System

- Type scale consistency
- Line height and paragraph spacing
- Font weights and contrast
- Readability at different densities
- Heading/body hierarchy clarity

### E. Navigation & Discoverability

- Clarity of navigation structure
- Wayfinding and orientation cues
- Visibility of available actions
- Affordances for interactive elements

### F. Accessibility Basics

- Color contrast (WCAG AA minimum)
- Focus states visibility
- Keyboard navigation cues
- Click/tap target sizes (minimum 44×44px touch targets)

### G. Panel Architecture (Dedicated Review)

**For every panel (left, right, top, bottom):**

| Check | Requirement |
|-------|-------------|
| **Collapsibility** | Can the panel collapse? Is the trigger visible and intuitive? |
| **Collapse animation** | Smooth transition (200–300ms ease-out), no layout jank |
| **Collapsed state** | Clear indicator of collapsed state, easy to re-expand |
| **Resize behavior** | If resizable: drag handle visible, min/max constraints, cursor feedback |
| **Persistence** | State preserved across navigation? Across sessions (localStorage)? |
| **Default state** | Sensible default (open vs closed) based on screen size and context |
| **Content overflow** | Proper scroll handling when content exceeds panel height |

### H. Overflow, Scroll & Fixed Zones (Dedicated Review)

**For every scrollable region:**

| Check | Requirement |
|-------|-------------|
| **Scroll container identification** | Which element scrolls? Is it intentional? |
| **Fixed elements** | Headers, footers, toolbars – do they stay fixed correctly? |
| **Sticky elements** | Table headers, section headers – do they stick at the right threshold? |
| **Scroll indicators** | Visible scrollbar or fade/shadow hints for hidden content |
| **Overflow clipping** | No unintended content clipping or horizontal scroll |
| **Nested scroll traps** | No scroll hijacking or confusing nested scroll areas |
| **Keyboard scroll** | Arrow keys, Page Up/Down, Home/End work as expected |

### I. Responsive Design (Dedicated Review)

**Test at these breakpoints:**

| Breakpoint | Width | Priority |
|------------|-------|----------|
| Mobile S | 320px | Must work |
| Mobile L | 428px | Must work |
| Tablet | 768px | Must work |
| Desktop | 1280px | Primary |
| Desktop L | 1536px+ | Should optimize |

**For each breakpoint, verify:**

- Layout adapts without horizontal scroll
- Touch targets meet 44×44px minimum on touch devices
- Panels collapse or transform appropriately
- Typography remains readable (minimum 16px body on mobile)
- Navigation transforms (hamburger menu, bottom nav, etc.)
- Modals and overlays adapt to viewport
- Tables transform (horizontal scroll, card view, or column hiding)
- Images and media scale correctly

### J. Motion & Micro-interactions

- **Transition timing**: 150–300ms for UI state changes
- **Easing curves**: Use ease-out for entrances, ease-in for exits
- **Loading states**: Skeleton screens over spinners where possible
- **Feedback**: Hover, active, and focus states on all interactive elements
- **Reduced motion**: Respects `prefers-reduced-motion`

---

## Product-Level Requirements to Validate

You must explicitly evaluate and recommend improvements for:

### Collapsible Panels

| Requirement | Specification |
|-------------|---------------|
| Default states | Define open/closed default per viewport size |
| Collapse trigger | Visible toggle button with clear iconography |
| Collapse animation | 200–300ms ease-out, no content reflow jank |
| Resize behavior | If supported: min/max constraints, visual drag handle |
| Persistence | Save state to localStorage, restore on reload |
| Keyboard support | Toggle via keyboard shortcut (document it) |

### Visual Hierarchy

- Establish clear **primary action** per screen (one only)
- **Secondary actions** visually subordinate
- **Destructive actions** require confirmation, use warning colors
- **Disabled states** clearly distinguishable (not just grayed out)

### Space Optimization

- No wasted space on large screens (use max-width containers)
- No cramped layouts on small screens (allow breathing room)
- Consistent use of spacing scale throughout

### Typography System

- Define and enforce a **strict type scale**
- Maximum 2–3 font weights per project
- Line height: 1.4–1.6 for body, 1.1–1.3 for headings

---

## Deliverables & Output Structure

### File Structure

Create an `audit_ui/` directory containing:

```
audit_ui/
├── summary.md                 # Executive summary and roadmap
├── design-tokens.md           # Proposed design system tokens
├── screens/
│   ├── dashboard.md
│   ├── settings.md
│   └── [screen-name].md
├── components/
│   ├── panels.md              # Panel architecture audit
│   ├── navigation.md          # Navigation patterns audit
│   └── [component-type].md
└── responsive/
    ├── mobile.md              # Mobile-specific findings
    ├── tablet.md              # Tablet-specific findings
    └── breakpoint-issues.md   # Cross-breakpoint issues
```

---

### For Each Screen / Container File

Include the following sections:

#### 1. What I Reviewed

```markdown
- **Route**: /dashboard
- **Viewport(s) tested**: 320px, 768px, 1280px, 1536px
- **UI regions**: Left nav, main content, right panel, header
- **States captured**: Default, panel collapsed, empty state, loading
- **Screenshots**: [embedded below]
- **Relevant codebase files**: `src/components/Dashboard.tsx`, `src/layouts/MainLayout.tsx`
```

#### 2. Slickness Score

Rate the screen against slick interface criteria:

| Criterion | Score (1–5) | Notes |
|-----------|-------------|-------|
| Visual refinement | | |
| Modern styling | | |
| Smooth interactions | | |
| Professional polish | | |
| **Overall** | | |

#### 3. Issues

List issues grouped by severity:

**🔴 Critical** – Blocks usability, causes confusion, or breaks hierarchy
**🟠 Major** – Significantly degrades clarity, efficiency, or consistency  
**🟡 Minor** – Polish, refinement, or optimization opportunities

For each issue:

```markdown
### [Issue Title]

- **Severity**: 🔴 Critical
- **Location**: Right panel → header section
- **Viewport(s) affected**: All / Mobile only / etc.
- **Current behavior**: [describe]
- **Expected behavior**: [describe]
- **Screenshot**: [embed]
```

#### 4. Recommendations

Specific, actionable design changes with explicit specifications:

```markdown
### [Recommendation Title]

**Change**: Reduce right panel header padding from 24px to 16px

**Specifications**:
- Padding: 16px horizontal, 12px vertical
- Margin-bottom: 8px
- Border-bottom: 1px solid var(--border-subtle)

**Applies to**: All viewport sizes

**Code hint**: Update `.panel-header` class in `Panel.module.css`
```

#### 5. Rationale

- Why this change improves usability, clarity, accessibility, or scalability
- Reference UX principles where relevant (Fitts's Law, Hick's Law, Gestalt, etc.)

#### 6. Acceptance Criteria

```markdown
- [ ] Padding matches specification (16px horizontal, 12px vertical)
- [ ] Visual spacing verified at 320px, 768px, 1280px viewports
- [ ] No content clipping or overflow
- [ ] Passes visual regression test
```

#### 7. Layout Representation

Use **ASCII diagrams** for layout changes:

```
┌─────────────────────────────────────────────────────────┐
│ Header (fixed, h: 56px)                                 │
├────────────┬────────────────────────┬───────────────────┤
│ Left Panel │ Main Content           │ Right Panel       │
│ (collapsible)│ (scrollable)          │ (collapsible)     │
│ w: 240px   │ flex: 1                │ w: 320px          │
│ min: 64px  │ max-w: 1200px          │ min: 48px         │
│            │ overflow-y: auto       │                   │
├────────────┴────────────────────────┴───────────────────┤
│ Footer (optional, sticky or fixed)                      │
└─────────────────────────────────────────────────────────┘
```

---

### Summary File (`audit_ui/summary.md`)

Include:

#### 1. Executive Summary

- Overall slickness score
- Top 3 critical issues
- Top 3 quick wins
- Estimated effort breakdown

#### 2. Prioritized Roadmap

| Priority | Category | Items | Effort |
|----------|----------|-------|--------|
| **🚀 Quick Wins** | Polish & consistency | [list] | < 1 day each |
| **📍 Next** | Core UX improvements | [list] | 1–3 days each |
| **📅 Later** | Enhancements | [list] | 3+ days each |

#### 3. Proposed Design Tokens (`design-tokens.md`)

```markdown
## Typography Scale

| Token | Size | Weight | Line Height | Usage |
|-------|------|--------|-------------|-------|
| `--text-xs` | 12px | 400 | 1.5 | Captions, labels |
| `--text-sm` | 14px | 400 | 1.5 | Secondary text |
| `--text-base` | 16px | 400 | 1.5 | Body text |
| `--text-lg` | 18px | 500 | 1.4 | Subheadings |
| `--text-xl` | 20px | 600 | 1.3 | Section headings |
| `--text-2xl` | 24px | 600 | 1.2 | Page headings |
| `--text-3xl` | 30px | 700 | 1.1 | Hero headings |

## Spacing Scale

| Token | Value | Usage |
|-------|-------|-------|
| `--space-1` | 4px | Tight grouping |
| `--space-2` | 8px | Related elements |
| `--space-3` | 12px | Default gap |
| `--space-4` | 16px | Section padding |
| `--space-6` | 24px | Card padding |
| `--space-8` | 32px | Section margins |
| `--space-12` | 48px | Major sections |

## Panel Dimensions

| Token | Value | Notes |
|-------|-------|-------|
| `--panel-left-width` | 240px | Default expanded |
| `--panel-left-collapsed` | 64px | Icon-only mode |
| `--panel-right-width` | 320px | Default expanded |
| `--panel-right-collapsed` | 48px | Indicator only |
| `--panel-min-width` | 200px | Resize constraint |
| `--panel-max-width` | 480px | Resize constraint |

## Animation Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--duration-fast` | 150ms | Micro-interactions |
| `--duration-normal` | 250ms | UI state changes |
| `--duration-slow` | 400ms | Page transitions |
| `--ease-out` | cubic-bezier(0, 0, 0.2, 1) | Entrances |
| `--ease-in` | cubic-bezier(0.4, 0, 1, 1) | Exits |
| `--ease-in-out` | cubic-bezier(0.4, 0, 0.2, 1) | Morphs |
```

#### 4. Standardized Design Patterns

Document recommended patterns for:

- **Panels**: Collapsible, resizable, with persistence
- **Tables**: Responsive strategies, sorting, pagination
- **Forms**: Layout, validation, error states
- **Empty states**: Illustration, messaging, CTAs
- **Modals & overlays**: Sizing, backdrop, focus trap
- **Loading states**: Skeletons, spinners, optimistic UI
- **Error states**: Inline, toast, full-page

---

## Quality Checklist for Auditor

Before submitting, verify:

- [ ] Every screen has been reviewed at minimum 3 breakpoints (mobile, tablet, desktop)
- [ ] Every panel has been tested for collapsibility
- [ ] Every scrollable area has been identified and evaluated
- [ ] Screenshots are embedded for all findings
- [ ] All recommendations include specific measurements (px, rem, ms)
- [ ] Acceptance criteria are verifiable by design and engineering
- [ ] Design tokens are complete and internally consistent
- [ ] Roadmap is prioritized by impact and effort
- [ ] All markdown files are properly formatted and linked

---

## Additional Requirements

- Write everything in **Markdown**
- Embed screenshots where relevant (use descriptive alt text)
- Cross-reference any existing UX improvements already documented
- Be **opinionated, precise, and implementation-ready**
- Optimize for **clarity, consistency, slickness, and long-term scalability**
- Flag any pattern that deviates from "slick interface" standards
- Propose motion/animation enhancements where interactions feel static or abrupt

Write audit in ./audit_ui/ as specified above in several markdown files.