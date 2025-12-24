# Phase 1: Core Fixes & Feedback

## Overview

Phase 1 focuses on critical user-facing issues that block or significantly impair the user experience. These changes establish foundational UX patterns.

**Duration:** 2-3 days  
**Priority:** P0 + P1 issues

---

## 1. Home/Dashboard Page (P0 Critical)

### Current State

- [page.tsx](<../../edgequake_webui/src/app/(dashboard)/page.tsx>) redirects to `/graph`
- Logo in [sidebar.tsx#L27](../../edgequake_webui/src/components/layout/sidebar.tsx#L27) links to `/graph`

### Target State

Create a proper dashboard at `/` with:

- System statistics (document count, entity count, relationship count)
- API connection status
- Quick action buttons (Upload, Query, View Graph)
- Recent activity (last uploads, last queries)

### Implementation Steps

#### 1.1 Create Dashboard Page

**File:** `edgequake_webui/src/app/(dashboard)/page.tsx`

```tsx
// Replace redirect with actual dashboard content
// - Use React Query to fetch stats from API
// - Create StatCard component for metrics
// - Add QuickActions section with links
// - Add RecentActivity section
```

#### 1.2 Create Dashboard Components

**New File:** `edgequake_webui/src/components/dashboard/stats-card.tsx`

```tsx
// Props: title, value, icon, description, trend?
// - Show loading skeleton state
// - Show error state if data unavailable
// - Animate value changes
```

**New File:** `edgequake_webui/src/components/dashboard/quick-actions.tsx`

```tsx
// Three action cards: Upload, Query, View Graph
// - Large clickable cards
// - Icons and descriptions
// - Link to respective pages
```

**New File:** `edgequake_webui/src/components/dashboard/recent-activity.tsx`

```tsx
// List of recent documents and queries
// - Timestamp
// - Action type (upload, query)
// - Document/query preview
```

#### 1.3 Update Sidebar Logo Link

**File:** `edgequake_webui/src/components/layout/sidebar.tsx#L27`

Change:

```tsx
<Link href="/graph" ...>
```

To:

```tsx
<Link href="/" ...>
```

#### 1.4 Add Dashboard API Endpoint

**File:** `edgequake_webui/src/lib/api/edgequake.ts`

```tsx
export async function getStats(): Promise<{
  documentCount: number;
  entityCount: number;
  relationshipCount: number;
  lastUpdated: string;
}> {
  // Combine calls to existing endpoints:
  // - GET /documents (count)
  // - GET /graph (node/edge counts)
}
```

#### 1.5 Add Translation Keys

**Files:** `edgequake_webui/src/locales/{en,zh,fr}.json`

```json
{
  "dashboard": {
    "title": "Dashboard",
    "welcome": "Welcome to EdgeQuake",
    "stats": {
      "documents": "Documents",
      "entities": "Entities",
      "relationships": "Relationships"
    },
    "quickActions": {
      "title": "Quick Actions",
      "upload": "Upload Documents",
      "query": "Query Knowledge",
      "graph": "View Graph"
    },
    "recentActivity": {
      "title": "Recent Activity",
      "noActivity": "No recent activity"
    }
  }
}
```

### Test Cases

- [ ] Dashboard loads without errors
- [ ] Stats display correct counts
- [ ] Quick action buttons navigate correctly
- [ ] Logo click navigates to dashboard
- [ ] Empty state shows when no documents

---

## 2. Settings Save Confirmation (P1 High)

### Current State

- Settings change silently in [settings/page.tsx](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>)
- No visual feedback when settings are saved

### Target State

Show toast notification when settings are changed.

### Implementation Steps

#### 2.1 Add Toast on Settings Change

**File:** `edgequake_webui/src/stores/use-settings-store.ts`

Modify store methods to optionally trigger toasts:

```tsx
setLanguage: (language, showToast = true) => {
  set({ language });
  if (showToast) {
    toast.success("Language updated");
  }
};
```

Or add wrapper function in settings page:

**File:** `edgequake_webui/src/app/(dashboard)/settings/page.tsx`

```tsx
const handleLanguageChange = (lang: string) => {
  setLanguage(lang);
  toast.success(t("settings.saved", "Settings saved"));
};
```

### Test Cases

- [ ] Toast appears when theme changes
- [ ] Toast appears when language changes
- [ ] Toast appears when graph settings change
- [ ] Toasts are localized

---

## 3. Clear History Confirmation (P1 High)

### Current State

- "Clear Query History" in settings has no confirmation

### Target State

Add confirmation dialog before clearing history.

### Implementation Steps

#### 3.1 Wrap Clear History in AlertDialog

**File:** `edgequake_webui/src/app/(dashboard)/settings/page.tsx`

```tsx
<AlertDialog>
  <AlertDialogTrigger asChild>
    <Button variant="destructive" size="sm">
      <Trash2 className="h-4 w-4 mr-2" />
      {t("settings.clearHistory")}
    </Button>
  </AlertDialogTrigger>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>{t("settings.clearHistoryConfirm")}</AlertDialogTitle>
      <AlertDialogDescription>
        {t("settings.clearHistoryDescription", {
          count: queryCount,
          favorites: favoriteCount,
        })}
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
      <AlertDialogAction onClick={handleClearHistory}>
        {t("common.delete")}
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
```

### Test Cases

- [ ] Dialog appears when clicking Clear History
- [ ] Cancel button closes dialog without action
- [ ] Confirm button clears history
- [ ] Toast confirms deletion

---

## 4. Upload Zone Improvements (P1 High)

### Current State

- No max file size displayed
- No client-side size validation

### Target State

- Show supported file types AND max file size
- Validate file size before upload

### Implementation Steps

#### 4.1 Add File Size Display

**File:** `edgequake_webui/src/components/documents/document-manager.tsx`

```tsx
// In upload zone text
<p className="text-sm text-muted-foreground mt-1">
  {t("documents.uploadSupports")} (max 10MB)
</p>
```

#### 4.2 Add Client-Side Validation

**File:** `edgequake_webui/src/components/documents/document-manager.tsx`

```tsx
const MAX_FILE_SIZE = 10 * 1024 * 1024; // 10MB

const handleFilesUpload = useCallback(async (files: File[]) => {
  // Filter out oversized files
  const validFiles: File[] = [];
  const oversizedFiles: File[] = [];

  for (const file of files) {
    if (file.size > MAX_FILE_SIZE) {
      oversizedFiles.push(file);
    } else {
      validFiles.push(file);
    }
  }

  // Show error for oversized files
  if (oversizedFiles.length > 0) {
    toast.error(t('documents.upload.fileTooLarge', {
      count: oversizedFiles.length,
      max: '10MB'
    }));
  }

  // Continue with valid files
  if (validFiles.length === 0) return;

  // ... rest of upload logic
}, [...]);
```

#### 4.3 Add Translation Keys

```json
{
  "documents": {
    "upload": {
      "fileTooLarge": "{{count}} file(s) exceed the 10MB limit",
      "maxSize": "Maximum file size: 10MB"
    }
  }
}
```

### Test Cases

- [ ] Max file size is displayed
- [ ] Files over 10MB are rejected
- [ ] Error toast appears for oversized files
- [ ] Valid files still upload successfully

---

## 5. Empty State Improvements (P1 High)

### Current State

- Basic "No documents yet" message
- No illustration or guidance

### Target State

- Engaging empty state with icon/illustration
- Clear call-to-action
- Helpful guidance text

### Implementation Steps

#### 5.1 Create Empty State Component

**New File:** `edgequake_webui/src/components/shared/empty-state.tsx`

```tsx
interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
}: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12 px-4 text-center">
      <div className="w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4">
        <Icon className="h-8 w-8 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-medium mb-2">{title}</h3>
      <p className="text-sm text-muted-foreground max-w-sm mb-4">
        {description}
      </p>
      {action && <Button onClick={action.onClick}>{action.label}</Button>}
    </div>
  );
}
```

#### 5.2 Use in Documents Page

**File:** `edgequake_webui/src/components/documents/document-manager.tsx`

```tsx
// Replace basic empty message with:
<EmptyState
  icon={FileText}
  title={t("documents.noDocuments")}
  description={t("documents.noDocumentsSubtitle")}
  action={{
    label: t("documents.uploadFirst"),
    onClick: () => document.getElementById("upload-input")?.click(),
  }}
/>
```

### Test Cases

- [ ] Empty state shows icon
- [ ] Action button is visible
- [ ] Clicking action triggers upload

---

## 6. Sidebar Collapse (P1 High)

### Current State

- Fixed 256px sidebar width
- No collapse functionality

### Target State

- Toggle button to collapse sidebar
- Icon-only mode when collapsed
- Persist preference in localStorage

### Implementation Steps

#### 6.1 Add Collapse State to Store

**File:** `edgequake_webui/src/stores/use-settings-store.ts`

```tsx
interface SettingsState {
  // ... existing
  sidebarCollapsed: boolean;
  setSidebarCollapsed: (collapsed: boolean) => void;
}
```

#### 6.2 Update Sidebar Component

**File:** `edgequake_webui/src/components/layout/sidebar.tsx`

```tsx
export function Sidebar() {
  const { sidebarCollapsed, setSidebarCollapsed } = useSettingsStore();

  return (
    <aside
      className={cn(
        "hidden border-r bg-card md:block transition-all duration-200",
        sidebarCollapsed ? "w-16" : "w-64"
      )}
    >
      {/* Collapse toggle button */}
      <Button
        variant="ghost"
        size="icon"
        className="absolute right-2 top-4"
        onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
      >
        {sidebarCollapsed ? <ChevronRight /> : <ChevronLeft />}
      </Button>

      {/* Conditional rendering of labels */}
      {navItems.map(({ href, icon: Icon, labelKey }) => (
        <Link key={href} href={href}>
          <Icon />
          {!sidebarCollapsed && <span>{t(labelKey)}</span>}
        </Link>
      ))}
    </aside>
  );
}
```

### Test Cases

- [ ] Toggle button collapses sidebar
- [ ] Icons remain visible when collapsed
- [ ] Labels hidden when collapsed
- [ ] Preference persists across page loads

---

## E2E Test Additions

**File:** `edgequake_webui/e2e/phase1-ux.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

test.describe("Phase 1: Core UX Fixes", () => {
  test("dashboard page loads with stats", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText(/dashboard|welcome/i)).toBeVisible();
    await expect(page.getByText(/documents/i)).toBeVisible();
  });

  test("logo navigates to dashboard", async ({ page }) => {
    await page.goto("/documents");
    await page.getByRole("link", { name: /edgequake/i }).click();
    await expect(page).toHaveURL("/");
  });

  test("settings changes show toast", async ({ page }) => {
    await page.goto("/settings");
    // Change theme
    await page.getByRole("combobox").first().click();
    await page.getByRole("option", { name: /dark/i }).click();
    // Toast should appear
    await expect(page.getByText(/saved|updated/i)).toBeVisible();
  });

  test("upload zone shows max file size", async ({ page }) => {
    await page.goto("/documents");
    await expect(page.getByText(/10.*mb/i)).toBeVisible();
  });
});
```

---

## Verification Checklist

Before marking Phase 1 complete:

- [ ] All P0 issues resolved
- [ ] All P1 issues in scope resolved
- [ ] No TypeScript errors (`npm run build`)
- [ ] Lint passes (`npm run lint`)
- [ ] E2E tests pass (`npm run test:e2e`)
- [ ] Manual testing on desktop
- [ ] Manual testing on mobile viewport
- [ ] Translations complete for en/zh/fr

---

## Files Modified Summary

| File                                        | Action  | Description                |
| ------------------------------------------- | ------- | -------------------------- |
| `app/(dashboard)/page.tsx`                  | Replace | Create dashboard page      |
| `components/dashboard/stats-card.tsx`       | Create  | Stats display component    |
| `components/dashboard/quick-actions.tsx`    | Create  | Quick action buttons       |
| `components/dashboard/recent-activity.tsx`  | Create  | Activity feed              |
| `components/layout/sidebar.tsx`             | Modify  | Logo link + collapse       |
| `components/shared/empty-state.tsx`         | Create  | Reusable empty state       |
| `components/documents/document-manager.tsx` | Modify  | File size validation       |
| `app/(dashboard)/settings/page.tsx`         | Modify  | Confirmation dialogs       |
| `stores/use-settings-store.ts`              | Modify  | Sidebar collapse state     |
| `locales/en.json`                           | Modify  | Add dashboard translations |
| `locales/zh.json`                           | Modify  | Add dashboard translations |
| `locales/fr.json`                           | Modify  | Add dashboard translations |
| `e2e/phase1-ux.spec.ts`                     | Create  | Phase 1 E2E tests          |

---

## Next Phase

After Phase 1 is complete and committed, proceed to:

- [Phase 2: Graph & Query Experience](./02-phase2-graph-query.md)
