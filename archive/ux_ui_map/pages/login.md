# Page: Login

## Overview

- **Route**: `/login`
- **Title**: "EdgeQuake" (displayed in card)
- **Layout**: Full-screen centered card layout (no sidebar or header)
- **Route Group**: (auth) — Authentication route group
- **Source File**: [src/app/(auth)/login/page.tsx](../../edgequake_webui/src/app/(auth)/login/page.tsx)

## Layout Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│           Background: gradient from background to muted/50          │
│                                                                     │
│                    ┌─────────────────────────────┐                  │
│                    │                             │                  │
│                    │         [Logo Icon]         │                  │
│                    │                             │                  │
│                    │        EdgeQuake            │                  │
│                    │  Sign in to access the...   │                  │
│                    │                             │                  │
│                    │  ┌─────────────────────┐    │                  │
│                    │  │ Username            │    │                  │
│                    │  └─────────────────────┘    │                  │
│                    │                             │                  │
│                    │  ┌─────────────────────┐    │                  │
│                    │  │ Password            │    │                  │
│                    │  └─────────────────────┘    │                  │
│                    │                             │                  │
│                    │  [      Sign In       ]     │                  │
│                    │                             │                  │
│                    │  ─────────── OR ───────────  │                  │
│                    │                             │                  │
│                    │  [ Continue without login ] │                  │
│                    │                             │                  │
│                    └─────────────────────────────┘                  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Screenshots

| Viewport | Screenshot |
|----------|------------|
| Desktop (1440px) | [login-desktop.png](../screenshots/login/login-desktop.png) |
| Tablet (768px) | [login-tablet.png](../screenshots/login/login-tablet.png) |
| Mobile (375px) | [login-mobile.png](../screenshots/login/login-mobile.png) |

---

## Region: Page Background

- **Type**: Full viewport container
- **Dimensions**: min-h-screen, flex centered
- **Background**: Gradient from `var(--background)` to `var(--muted)/50`
- **Padding**: 16px (p-4)

---

## Container: Login Card

- **Type**: Card component
- **Dimensions**: Max width 448px (max-w-md), full width on mobile
- **Background**: `var(--card)`
- **Border**: 1px solid `var(--border)`
- **Border Radius**: 12px
- **Shadow**: Card shadow (shadow-sm)
- **Source**: [src/components/ui/card.tsx](../../edgequake_webui/src/components/ui/card.tsx)

### Container: Card Header

- **Layout**: Centered text
- **Spacing**: Default CardHeader padding

#### Component: Logo Icon Container

- **Type**: Decorative container
- **Dimensions**: 48px × 48px (h-12 w-12)
- **Background**: `var(--primary)/10`
- **Border Radius**: 50% (rounded-full)
- **Position**: Centered, 16px bottom margin

##### Element: Network Icon

- **Type**: Lucide Network icon
- **Dimensions**: 24px × 24px (h-6 w-6)
- **Color**: `var(--primary)`

#### Component: Title

- **Type**: CardTitle (H2)
- **Typography**: 24px (text-2xl), bold (font-bold)
- **Content**: "EdgeQuake"

#### Component: Description

- **Type**: CardDescription
- **Typography**: 14px, muted-foreground
- **Content**: "Sign in to access the Knowledge Graph RAG Platform"

---

### Container: Card Content (Form)

- **Type**: Form element
- **Layout**: Flex column
- **Spacing**: 16px gap between form fields (space-y-4)
- **onSubmit**: handleSubmit function

#### Container: Username Field

- **Layout**: Flex column
- **Spacing**: 8px gap

##### Component: Username Label

- **Type**: HTML label element
- **Typography**: 14px, medium (font-medium)
- **Content**: "Username"

##### Component: Username Input

- **Type**: Input component
- **Input Type**: text
- **Placeholder**: "Enter your username"
- **Required**: Yes
- **Disabled State**: When isLoading is true
- **Source**: [src/components/ui/input.tsx](../../edgequake_webui/src/components/ui/input.tsx)

#### Container: Password Field

- **Layout**: Flex column
- **Spacing**: 8px gap

##### Component: Password Label

- **Type**: HTML label element
- **Typography**: 14px, medium
- **Content**: "Password"

##### Component: Password Input

- **Type**: Input component
- **Input Type**: password
- **Placeholder**: "Enter your password"
- **Required**: Yes
- **Disabled State**: When isLoading is true

---

### Container: Error Message

- **Type**: Error display block
- **Visibility**: Only when error state is set
- **Background**: `var(--destructive)/10`
- **Border Radius**: 6px (rounded-md)
- **Padding**: 12px (p-3)
- **Typography**: 14px, text-destructive
- **Content**: Dynamic error message

---

### Component: Sign In Button

- **Type**: Button, default variant
- **Width**: Full width (w-full)
- **Typography**: 14px, medium
- **States**:
  - Default: "Sign In"
  - Loading: Loader2 icon (animated spin) + "Signing in..."
  - Disabled: When isLoading is true
- **Function**: Submits login form

---

### Container: Divider

- **Type**: Visual separator with text
- **Layout**: Relative positioning with centered text
- **Margin**: 16px vertical (my-4)

#### Element: Divider Line

- **Type**: Span with border-t
- **Width**: Full width
- **Position**: Absolute, centered vertically

#### Element: Divider Text

- **Type**: Span
- **Typography**: 12px, uppercase, muted-foreground
- **Background**: `var(--background)` (for text visibility)
- **Padding**: 8px horizontal
- **Content**: "Or"

---

### Component: Demo Button

- **Type**: Button, outline variant
- **Width**: Full width (w-full)
- **Typography**: 14px
- **Content**: "Continue without login (Demo)"
- **Function**: Navigates directly to /graph without authentication

---

## State Management

- **Local State**:
  - username: string
  - password: string
  - isLoading: boolean
  - error: string | null
- **Auth Store**: [src/stores/use-auth-store.ts](../../edgequake_webui/src/stores/use-auth-store.ts)
  - Method: login(response)
- **API**: [src/lib/api/edgequake.ts](../../edgequake_webui/src/lib/api/edgequake.ts)
  - Function: login({ username, password })

---

## Responsive Behavior

| Breakpoint | Card Width | Padding |
|------------|------------|---------|
| Mobile (<768px) | Full width - 32px | 16px |
| Tablet (768-1024px) | Max 448px | 16px |
| Desktop (>1024px) | Max 448px | 16px |

---

## Component Cross-References

- [Card](../components/cards.md) — Login card container
- [Button](../components/buttons.md) — Sign In, Demo buttons
- [Input](../components/inputs.md) — Username, Password inputs

