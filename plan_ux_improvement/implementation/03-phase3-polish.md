# Phase 3: Polish & Accessibility

## Overview

Phase 3 focuses on polish, accessibility, and long-term quality improvements. These changes ensure the application is usable by all users and performs well.

**Duration:** 2-3 days  
**Priority:** P2 + P3 issues  
**Prerequisite:** Phase 1 & 2 complete

---

## 1. Settings Import/Export (P2 Medium)

### Current State

- Settings stored in localStorage
- No way to backup or share settings

### Target State

- Export settings as JSON file
- Import settings from JSON file
- Validation on import

### Implementation Steps

#### 1.1 Create Settings Export/Import Functions

**File:** `edgequake_webui/src/stores/use-settings-store.ts`

```tsx
export const useSettingsStore = create<SettingsState>()(
  persist(
    (set, get) => ({
      // ... existing state

      exportSettings: () => {
        const state = get();
        const exportData = {
          version: "1.0",
          exportedAt: new Date().toISOString(),
          settings: {
            language: state.language,
            graphSettings: state.graphSettings,
            querySettings: state.querySettings,
            sidebarCollapsed: state.sidebarCollapsed,
          },
        };
        return JSON.stringify(exportData, null, 2);
      },

      importSettings: (jsonString: string) => {
        try {
          const data = JSON.parse(jsonString);

          // Validate structure
          if (!data.version || !data.settings) {
            throw new Error("Invalid settings file format");
          }

          // Apply settings
          const { settings } = data;
          set({
            language: settings.language || "en",
            graphSettings: settings.graphSettings || defaultGraphSettings,
            querySettings: settings.querySettings || defaultQuerySettings,
            sidebarCollapsed: settings.sidebarCollapsed || false,
          });

          return { success: true };
        } catch (error) {
          return {
            success: false,
            error: error instanceof Error ? error.message : "Unknown error",
          };
        }
      },
    }),
    { name: "edgequake-settings" }
  )
);
```

#### 1.2 Add UI for Import/Export

**File:** `edgequake_webui/src/app/(dashboard)/settings/page.tsx`

```tsx
// Export button
const handleExport = () => {
  const json = exportSettings();
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "edgequake-settings.json";
  a.click();
  URL.revokeObjectURL(url);
  toast.success(t("settings.exported"));
};

// Import button with file input
const handleImport = (event: React.ChangeEvent<HTMLInputElement>) => {
  const file = event.target.files?.[0];
  if (!file) return;

  const reader = new FileReader();
  reader.onload = (e) => {
    const result = importSettings(e.target?.result as string);
    if (result.success) {
      toast.success(t("settings.imported"));
    } else {
      toast.error(t("settings.importError", { error: result.error }));
    }
  };
  reader.readAsText(file);
};

// In the Data Management section:
<div className="flex items-center gap-2">
  <Button variant="outline" onClick={handleExport}>
    <Download className="h-4 w-4 mr-2" />
    {t("settings.export")}
  </Button>
  <Button variant="outline" asChild>
    <label className="cursor-pointer">
      <Upload className="h-4 w-4 mr-2" />
      {t("settings.import")}
      <input
        type="file"
        accept=".json"
        className="hidden"
        onChange={handleImport}
      />
    </label>
  </Button>
</div>;
```

### Test Cases

- [ ] Export downloads JSON file
- [ ] Import applies settings correctly
- [ ] Invalid file shows error
- [ ] Toast confirms success/failure

---

## 2. Skip Navigation Link (P3 Low)

### Current State

- No skip link for keyboard users
- Must tab through sidebar to reach main content

### Target State

- Skip link visible on focus
- Jumps to main content
- WCAG 2.1 compliant

### Implementation Steps

#### 2.1 Add Skip Link Component

**New File:** `edgequake_webui/src/components/shared/skip-link.tsx`

```tsx
"use client";

import { useTranslation } from "react-i18next";

export function SkipLink() {
  const { t } = useTranslation();

  return (
    <a
      href="#main-content"
      className="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-50 focus:px-4 focus:py-2 focus:bg-primary focus:text-primary-foreground focus:rounded-md"
    >
      {t("common.skipToContent", "Skip to main content")}
    </a>
  );
}
```

#### 2.2 Add to Layout

**File:** `edgequake_webui/src/app/(dashboard)/layout.tsx`

```tsx
import { SkipLink } from "@/components/shared/skip-link";

export default function DashboardLayout({ children }) {
  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <SkipLink />
      <Sidebar />
      <div className="flex flex-1 flex-col overflow-hidden">
        <Header />
        <DynamicBreadcrumb />
        <main id="main-content" className="flex-1 overflow-auto">
          {children}
        </main>
      </div>
    </div>
  );
}
```

### Test Cases

- [ ] Skip link visible when focused
- [ ] Clicking jumps to main content
- [ ] Works with keyboard navigation
- [ ] Not visible when not focused

---

## 3. Mobile Table Card View (P3 Low)

### Current State

- Table may overflow on mobile
- Hard to use on small screens

### Target State

- Card layout on mobile
- Full data visible
- Touch-friendly actions

### Implementation Steps

#### 3.1 Create Responsive Table Component

**New File:** `edgequake_webui/src/components/shared/responsive-table.tsx`

```tsx
"use client";

import { useMediaQuery } from "@/hooks/use-media-query";

interface Column<T> {
  key: keyof T;
  header: string;
  render?: (value: T[keyof T], item: T) => React.ReactNode;
}

interface ResponsiveTableProps<T> {
  data: T[];
  columns: Column<T>[];
  onRowClick?: (item: T) => void;
  renderActions?: (item: T) => React.ReactNode;
}

export function ResponsiveTable<T extends { id: string }>({
  data,
  columns,
  onRowClick,
  renderActions,
}: ResponsiveTableProps<T>) {
  const isMobile = useMediaQuery("(max-width: 768px)");

  if (isMobile) {
    return (
      <div className="space-y-3">
        {data.map((item) => (
          <div
            key={item.id}
            className="bg-card border rounded-lg p-4 space-y-2"
            onClick={() => onRowClick?.(item)}
          >
            {columns.map((col) => (
              <div key={String(col.key)} className="flex justify-between">
                <span className="text-sm text-muted-foreground">
                  {col.header}
                </span>
                <span className="text-sm font-medium">
                  {col.render
                    ? col.render(item[col.key], item)
                    : String(item[col.key])}
                </span>
              </div>
            ))}
            {renderActions && (
              <div className="pt-2 border-t flex justify-end">
                {renderActions(item)}
              </div>
            )}
          </div>
        ))}
      </div>
    );
  }

  // Regular table for desktop
  return (
    <table className="w-full">
      <thead>
        <tr>
          {columns.map((col) => (
            <th key={String(col.key)}>{col.header}</th>
          ))}
          {renderActions && <th>Actions</th>}
        </tr>
      </thead>
      <tbody>
        {data.map((item) => (
          <tr key={item.id} onClick={() => onRowClick?.(item)}>
            {columns.map((col) => (
              <td key={String(col.key)}>
                {col.render
                  ? col.render(item[col.key], item)
                  : String(item[col.key])}
              </td>
            ))}
            {renderActions && <td>{renderActions(item)}</td>}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

#### 3.2 Add Media Query Hook

**New File:** `edgequake_webui/src/hooks/use-media-query.ts`

```tsx
import { useState, useEffect } from "react";

export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(false);

  useEffect(() => {
    const media = window.matchMedia(query);
    setMatches(media.matches);

    const listener = (event: MediaQueryListEvent) => {
      setMatches(event.matches);
    };

    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, [query]);

  return matches;
}
```

### Test Cases

- [ ] Cards show on mobile viewport
- [ ] Table shows on desktop
- [ ] All data visible in cards
- [ ] Actions work in both modes

---

## 4. Page Skeleton Loaders (P2 Medium)

### Current State

- Simple spinner during loading
- Layout shifts when content loads

### Target State

- Skeleton loaders matching content shape
- Smooth transition to content
- Reduced layout shift

### Implementation Steps

#### 4.1 Create Skeleton Components

**New File:** `edgequake_webui/src/components/shared/skeletons.tsx`

```tsx
import { Skeleton } from "@/components/ui/skeleton";

export function DocumentTableSkeleton() {
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-4">
        <Skeleton className="h-8 w-[200px]" />
        <Skeleton className="h-8 w-[100px]" />
      </div>
      {Array.from({ length: 5 }).map((_, i) => (
        <div key={i} className="flex items-center gap-4 p-4 border rounded-lg">
          <Skeleton className="h-5 w-5 rounded" />
          <Skeleton className="h-4 flex-1" />
          <Skeleton className="h-4 w-[80px]" />
          <Skeleton className="h-4 w-[60px]" />
          <Skeleton className="h-8 w-8 rounded" />
        </div>
      ))}
    </div>
  );
}

export function GraphViewerSkeleton() {
  return (
    <div className="flex-1 relative">
      <Skeleton className="absolute inset-0" />
      <div className="absolute bottom-4 right-4 flex gap-2">
        <Skeleton className="h-10 w-10 rounded" />
        <Skeleton className="h-10 w-10 rounded" />
        <Skeleton className="h-10 w-10 rounded" />
      </div>
    </div>
  );
}

export function QueryInterfaceSkeleton() {
  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 p-4 space-y-4">
        <div className="flex items-start gap-3">
          <Skeleton className="h-8 w-8 rounded-full" />
          <Skeleton className="h-20 flex-1 rounded-lg" />
        </div>
        <div className="flex items-start gap-3 justify-end">
          <Skeleton className="h-12 w-3/4 rounded-lg" />
          <Skeleton className="h-8 w-8 rounded-full" />
        </div>
      </div>
      <div className="p-4 border-t">
        <Skeleton className="h-12 w-full rounded-lg" />
      </div>
    </div>
  );
}

export function DashboardSkeleton() {
  return (
    <div className="p-6 space-y-6">
      <Skeleton className="h-8 w-[200px]" />
      <div className="grid grid-cols-3 gap-4">
        <Skeleton className="h-24 rounded-lg" />
        <Skeleton className="h-24 rounded-lg" />
        <Skeleton className="h-24 rounded-lg" />
      </div>
      <Skeleton className="h-[300px] rounded-lg" />
    </div>
  );
}
```

#### 4.2 Use Skeletons in Pages

**File:** `edgequake_webui/src/components/documents/document-manager.tsx`

```tsx
if (isLoading) {
  return <DocumentTableSkeleton />;
}
```

### Test Cases

- [ ] Skeleton shows during load
- [ ] Shape matches actual content
- [ ] Smooth transition to content
- [ ] No layout shift

---

## 5. Toast Action Buttons (P2 Medium)

### Current State

- Toasts are passive (dismiss only)
- Cannot take action from toast

### Target State

- Add action buttons to toasts
- Undo, View, Retry actions

### Implementation Steps

#### 5.1 Update Toast Usage Throughout App

**Example File:** `edgequake_webui/src/components/documents/document-manager.tsx`

```tsx
// Success toast with action
toast.success("Document uploaded", {
  action: {
    label: "View",
    onClick: () => {
      setSelectedDocument(doc);
      setDetailOpen(true);
    },
  },
});

// Error toast with retry
toast.error("Upload failed", {
  action: {
    label: "Retry",
    onClick: () => handleFilesUpload([file]),
  },
});

// Delete toast with undo (more complex)
const handleDelete = async (id: string) => {
  // Store document temporarily
  const doc = documents.find((d) => d.id === id);

  // Optimistic delete
  queryClient.setQueryData(["documents"], (old) =>
    old.filter((d) => d.id !== id)
  );

  toast.success("Document deleted", {
    action: {
      label: "Undo",
      onClick: () => {
        // Restore document
        queryClient.setQueryData(["documents"], (old) => [...old, doc]);
        // Don't proceed with server delete
        clearTimeout(deleteTimeout);
      },
    },
  });

  // Actually delete after delay
  const deleteTimeout = setTimeout(() => {
    deleteMutation.mutate(id);
  }, 5000);
};
```

### Test Cases

- [ ] Toast shows action button
- [ ] Clicking action performs function
- [ ] Undo works for deletions
- [ ] Retry retries the action

---

## 6. Smooth Theme Transition (P2 Medium)

### Current State

- Theme changes may flash
- Abrupt color change

### Target State

- Smooth transition between themes
- No flash or flicker

### Implementation Steps

#### 6.1 Add CSS Transitions

**File:** `edgequake_webui/src/app/globals.css`

```css
/* Add smooth theme transition */
:root {
  transition: background-color 0.3s ease, color 0.3s ease;
}

* {
  transition: background-color 0.2s ease, border-color 0.2s ease;
}

/* Disable transitions during theme switch to prevent flicker */
html.theme-transition,
html.theme-transition *,
html.theme-transition *::before,
html.theme-transition *::after {
  transition-duration: 0s !important;
}
```

#### 6.2 Update Theme Switcher

**File:** `edgequake_webui/src/components/layout/header.tsx`

```tsx
const handleThemeChange = (newTheme: string) => {
  // Add transition class
  document.documentElement.classList.add("theme-transition");

  // Change theme
  setTheme(newTheme);

  // Remove class after brief delay
  setTimeout(() => {
    document.documentElement.classList.remove("theme-transition");
  }, 50);
};
```

### Test Cases

- [ ] Theme change is smooth
- [ ] No flash between themes
- [ ] Works for all theme values

---

## 7. Full Accessibility Audit (P2 Medium)

### Tasks

#### 7.1 Color Contrast Audit

- [ ] Run axe DevTools on all pages
- [ ] Fix any contrast failures
- [ ] Ensure 4.5:1 ratio for normal text
- [ ] Ensure 3:1 ratio for large text

#### 7.2 Keyboard Navigation

- [ ] Test all pages with keyboard only
- [ ] Ensure focus order is logical
- [ ] Add focus-visible styles where missing
- [ ] Ensure modals trap focus

#### 7.3 Screen Reader Testing

- [ ] Test with VoiceOver (macOS)
- [ ] Ensure all interactive elements labeled
- [ ] Add aria-live regions for dynamic content
- [ ] Ensure form errors announced

#### 7.4 Fix Common Issues

**File:** Various components

```tsx
// Add aria-label to icon buttons
<Button variant="ghost" size="icon" aria-label="Refresh">
  <RefreshCw className="h-4 w-4" />
</Button>

// Add role to custom controls
<div role="tablist">
  <button role="tab" aria-selected={isActive}>Tab 1</button>
</div>

// Add aria-live for status updates
<div aria-live="polite" aria-atomic="true">
  {status}
</div>
```

---

## E2E Test Additions

**File:** `edgequake_webui/e2e/phase3-ux.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

test.describe("Phase 3: Polish & Accessibility", () => {
  test("settings can be exported", async ({ page }) => {
    await page.goto("/settings");

    // Start download
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: /export/i }).click();
    const download = await downloadPromise;

    expect(download.suggestedFilename()).toContain(".json");
  });

  test("skip link works", async ({ page }) => {
    await page.goto("/");

    // Tab to skip link
    await page.keyboard.press("Tab");

    // Skip link should be visible when focused
    const skipLink = page.getByRole("link", { name: /skip/i });
    await expect(skipLink).toBeVisible();

    // Click skip link
    await skipLink.click();

    // Focus should be on main content
    await expect(page.locator("#main-content")).toBeFocused();
  });

  test("mobile shows card view", async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto("/documents");

    // Should see cards instead of table
    await expect(page.locator(".rounded-lg.border.p-4").first()).toBeVisible();
  });

  test("skeleton shows during load", async ({ page }) => {
    // Block API to see skeleton
    await page.route("**/api/**", (route) => {
      setTimeout(() => route.continue(), 2000);
    });

    await page.goto("/documents");

    // Skeleton should be visible
    await expect(page.locator(".animate-pulse").first()).toBeVisible();
  });

  test("theme transition is smooth", async ({ page }) => {
    await page.goto("/settings");

    // Change theme
    await page.getByRole("combobox").first().click();
    await page.getByRole("option", { name: /dark/i }).click();

    // No flash (hard to test, but ensure no error)
    await page.waitForTimeout(500);
    expect(await page.locator("html").getAttribute("class")).toContain("dark");
  });
});
```

---

## Verification Checklist

Before marking Phase 3 complete:

- [ ] All P3 issues in scope resolved
- [ ] Accessibility audit complete
- [ ] axe DevTools shows no critical issues
- [ ] No TypeScript errors (`npm run build`)
- [ ] Lint passes (`npm run lint`)
- [ ] All E2E tests pass
- [ ] Manual testing on desktop
- [ ] Manual testing on mobile viewport
- [ ] Screen reader testing complete

---

## Files Modified Summary

| File                                     | Action | Description             |
| ---------------------------------------- | ------ | ----------------------- |
| `stores/use-settings-store.ts`           | Modify | Import/export functions |
| `app/(dashboard)/settings/page.tsx`      | Modify | Import/export UI        |
| `components/shared/skip-link.tsx`        | Create | Skip navigation link    |
| `app/(dashboard)/layout.tsx`             | Modify | Add skip link           |
| `components/shared/responsive-table.tsx` | Create | Mobile card view        |
| `hooks/use-media-query.ts`               | Create | Media query hook        |
| `components/shared/skeletons.tsx`        | Create | Skeleton loaders        |
| `app/globals.css`                        | Modify | Theme transitions       |
| `components/layout/header.tsx`           | Modify | Smooth theme change     |
| `e2e/phase3-ux.spec.ts`                  | Create | Phase 3 E2E tests       |

---

## Final Verification

After Phase 3 is complete:

1. Run full E2E test suite: `npm run test:e2e`
2. Run build: `npm run build`
3. Manual test all user flows
4. Accessibility audit with axe DevTools
5. Performance audit with Lighthouse
6. Create final git commit with all changes
