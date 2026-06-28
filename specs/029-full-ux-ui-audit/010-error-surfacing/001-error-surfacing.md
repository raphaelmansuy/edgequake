# 001 — Error Surfacing Audit

**First Principle: Trust** — Errors are honest; recovery is easy.

---

## Error Layer Inventory

The app has **5 layers** of error communication:

```
Layer 1: Global Error (global-error.tsx)        ← App-level crash
Layer 2: Error Boundary (api-error-boundary)    ← Route-level failure
Layer 3: Backend Status Banner                  ← Network/service state
Layer 4: Inline Errors (document, login)        ← Field/action errors
Layer 5: Toast (sonner)                         ← Success/failure feedback
```

---

## Layer-by-Layer Analysis

### Layer 1 · Global Error Page (`global-error.tsx`)

Not yet audited for content, but common issues with Next.js global error pages:
- Generic "Something went wrong" message
- No actionable recovery steps
- No error reference code for support
- No automatic retry

**Minimum acceptable global error:**
```
┌──────────────────────────────────────────────────┐
│  [⚠️ Icon]                                       │
│  Something went wrong                            │
│  We encountered an unexpected error.             │
│                                                  │
│  [Try again]   [Go to Home]                      │
│                                                  │
│  Error code: ERR_2026_0628_ABC123  (copy)        │
└──────────────────────────────────────────────────┘
```

### Layer 3 · Backend Status Banner

**Positive:** The `BackendStatusBanner` distinguishes 3 states:
1. `unreachable` — backend is down
2. `misconfigured` — wrong port
3. Degraded/busy — backend is slow

**Issues:**

**ES-01 · Banner Position Creates CLS**

The banner renders between the breadcrumb and main content. When the backend is healthy, no banner renders. When it appears, the entire main content area shifts down.

This is a [Cumulative Layout Shift (CLS)](https://web.dev/cls/) — penalized by Core Web Vitals and disorienting for users.

**Fix:** Reserve space for the banner or render it as an overlay:

```typescript
// Option A: Fixed height placeholder
<div className={cn(
  "overflow-hidden transition-[height] duration-moderate",
  shouldShow ? "h-[auto]" : "h-0"
)}>
  {/* banner content */}
</div>

// Option B: Overlay banner (doesn't shift content)
<div className="sticky top-0 z-[var(--z-sticky)] ...">
  {/* banner */}
</div>
```

**ES-02 · Error Message Technical Language**

```
"Port 8080 is used by another service. Start EdgeQuake with make dev 
(backend runs on :8081 when :8080 is busy)."
```

This is developer documentation masquerading as a user-facing error. Regular users don't know what port 8080 means.

**Fix: Two-tier error messages**

```typescript
// User-facing (default):
"EdgeQuake is not available. Please try refreshing the page."

// Technical detail (expandable for developers):
<details>
  <summary>Technical details</summary>
  <code>Port 8080 unavailable. Backend may be starting...</code>
</details>
```

### Layer 4 · Login Form Errors

```typescript
// login/page.tsx
const [error, setError] = useState<string | null>(null);
// ...
{error && <div className="text-sm text-destructive">{error}</div>}
```

**ES-03 · Login Error Missing ARIA**

As documented in the accessibility audit (A11Y-02), the login error has no `role="alert"`, no association to the form via `aria-describedby`, and no icon.

**ES-04 · Generic Error Messages**

Login error: "Login failed" — this tells users nothing about *why* or *what to do*.

Better error hierarchy:
```
"Incorrect username or password. Please try again."
"Account locked — too many attempts. Contact your administrator."
"Service unavailable. Please try again later."
```

### Layer 4 · Document Processing Errors

Documents have an `ErrorMessagePopover` component that shows the failure message from the backend.

**Positive:** Hovering a failed document badge shows the error details.

**Issues:**

**ES-05 · Error in Popover: Low Discoverability**

Users must hover the status badge to see the error. This:
1. Doesn't work on mobile (no hover)
2. Requires discovering that the badge is hoverable

**Better pattern:** Show a truncated error inline, with "Show more" expand:

```
📄 report.pdf   ✕ Failed — Entity extraction timeout [Show more]
```

**ES-06 · No Recovery Guidance in Error Messages**

When entity extraction fails with "Network error: error sending request for url (http://localhost:11434/api/chat)", users see the raw error without context.

The UI should translate backend errors into actionable messages:

```typescript
// lib/utils/document-status.ts (likely location)
function humanizeError(error: string): { message: string; action?: string } {
  if (error.includes('11434')) {
    return {
      message: 'AI model service is not running',
      action: 'Start Ollama with: ollama serve'
    };
  }
  if (error.includes('quota')) {
    return {
      message: 'API usage limit reached',
      action: 'Check your API quota in Settings'
    };
  }
  return { message: 'Processing failed', action: 'Retry or contact support' };
}
```

### Layer 5 · Toast Notifications (Sonner)

**Positive:** The app uses `sonner` for toast notifications, which is modern and accessible.

**Issues:**

**ES-07 · Toast Accessibility Verification Needed**

Sonner's accessibility depends on configuration. Ensure:

```typescript
// In the Toaster setup (likely app/layout.tsx):
<Toaster 
  position="bottom-right"
  richColors
  expand={false}
  duration={4000}
  // Ensure the announcer is configured for screen readers
/>
```

Sonner uses `aria-live` regions but the configuration should be verified.

**ES-08 · Success Toast After Destructive Action**

After deleting a document, the app shows a success toast. However, there's no **undo** option. Modern UIs (like Notion, Linear, Gmail) provide an undo in the toast:

```
┌──────────────────────────────────────────────────┐
│  ✓ Document deleted        [Undo]  [✕]           │
└──────────────────────────────────────────────────┘
```

This is especially important for the **bulk delete** action which is irreversible.

---

## Error Hierarchy Framework

```
SEVERITY          UI PATTERN            Duration      Dismissible
─────────────────────────────────────────────────────────────────
Critical          Full-page error       Permanent     With action
High              Red inline alert      Permanent     Manual
Medium            Orange banner         Auto-dismiss  Manual
Low/Info          Toast notification    4s auto       Yes
Success           Green toast           3s auto       Yes
```

---

## Error Message Copy Framework

Good error messages follow this structure:

```
1. WHAT happened     → "Document processing failed"
2. WHY it happened   → "The AI model timed out after 60 seconds"  
3. WHAT to do        → "Try reprocessing with a smaller document"
4. ESCALATION        → "If this continues, contact support"
```

**Current state:** Most error messages only have #1.

---

## Recommended Error Recovery Patterns

### Document Failed → Recovery

```
┌──────────────────────────────────────────────────────────┐
│  ✕ Processing failed                                     │
│  ──────────────────────────────────────────────────────  │
│  Entity extraction timed out after 60 seconds.          │
│                                                          │
│  [⟳ Retry extraction]   [ℹ View error details]          │
└──────────────────────────────────────────────────────────┘
```

### Backend Unavailable → Recovery

```
┌──────────────────────────────────────────────────────────┐
│  ⚡ EdgeQuake is temporarily unavailable                 │
│  ──────────────────────────────────────────────────────  │
│  Some features may not work while reconnecting...        │
│                                                          │
│  [⟳ Retry now]   [✕ Dismiss]          Checking in 30s  │
└──────────────────────────────────────────────────────────┘
```

---

## External References

- [Error Message Design — NNGroup](https://www.nngroup.com/articles/error-message-guidelines/)
- [How to Write Better Error Messages — UX Collective](https://uxdesign.cc/ux-best-practices-error-handling)
- [Toast Notification Patterns — NNGroup](https://www.nngroup.com/articles/toast-notifications/)
- [Sonner Accessibility](https://sonner.emilkowalski.dev/accessibility)
- [Undoable Actions Pattern — UX Collective](https://uxdesign.cc/undoable-actions)
