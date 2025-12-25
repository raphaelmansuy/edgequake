# Settings Screen Audit

**Route:** `/settings`  
**Viewport(s) Tested:** 320px, 428px, 768px, 1280px, 1536px  
**UI Regions:** Header, Sidebar, Breadcrumb, Settings Navigation, Settings Content  
**States Captured:** Default, Theme Toggle, Form States  
**Screenshots:** `screenshots/screens/settings/`  
**Relevant Files:** `src/app/(dashboard)/settings/page.tsx`, `src/components/settings/`

---

## What I Reviewed

### Layout Structure

```
┌─────────────────────────────────────────────────────────────┐
│ Header (fixed, h: 64px)                      │ API │ 🌐 ☀️ 👤│
├────────────┬────────────────────────────────────────────────┤
│ Sidebar    │ Breadcrumb: EdgeQuake > Settings               │
│ w: 256px   ├────────────────────────────────────────────────┤
│            │ Settings                              Configure │
│            │ your EdgeQuake experience                      │
│            ├────────┬───────────────────────────────────────┤
│            │        │ ┌───────────────────────────────────┐ │
│            │General │ │ Settings Cards                    │ │
│            │Appear. │ │ ┌─────────────────────────────────┤ │
│            │Connect.│ │ │ Card: General Settings          │ │
│            │Advanced│ │ │ - Tenant Management             │ │
│            │        │ │ │   Tenant ID [________________]  │ │
│            │        │ │ │ - Working Mode                  │ │
│            │        │ │ │   [Hybrid ▾]                    │ │
│            │        │ │ │ - Language                      │ │
│            │        │ │ │   [English ▾]                   │ │
│            │        │ │ └─────────────────────────────────┤ │
│            │        │ │                                   │ │
│            │        │ │ [Reset Settings]                  │ │
│            │        │ └───────────────────────────────────┘ │
└────────────┴────────┴───────────────────────────────────────┘
```

---

## Slickness Score

| Criterion           | Score (1–5) | Notes                           |
| ------------------- | ----------- | ------------------------------- |
| Visual refinement   | 4.3         | Clean card layout, good spacing |
| Modern styling      | 4.5         | Contemporary settings pattern   |
| Smooth interactions | 4.0         | Theme toggle is smooth          |
| Professional polish | 4.2         | Well-organized sections         |
| **Overall**         | **4.3**     | Solid settings implementation   |

---

## Issues

### 🟠 Major

#### Settings Navigation Not Visible on Mobile

- **Severity:** 🟠 Major
- **Location:** Left settings tabs (General, Appearance, etc.)
- **Viewport(s) affected:** 320px, 428px
- **Current behavior:** Tabs may overflow or stack poorly
- **Expected behavior:** Horizontal scroll or dropdown selector on mobile

#### Form Inputs Lack Visible Borders

- **Severity:** 🟠 Major (Consistent with other screens)
- **Location:** All input fields, dropdowns
- **Viewport(s) affected:** All
- **Current behavior:** Subtle/invisible borders in light mode
- **Expected behavior:** Clear 1px border with `border-input` color

---

### 🟡 Minor

#### Card Section Spacing

- **Severity:** 🟡 Minor
- **Location:** Settings cards
- **Viewport(s) affected:** All
- **Current behavior:** Cards may use different padding
- **Expected behavior:** Consistent 24px padding in all cards

#### Reset Button Placement

- **Severity:** 🟡 Minor
- **Location:** Bottom of settings
- **Viewport(s) affected:** All
- **Current behavior:** Destructive action mixed with form
- **Expected behavior:** Move to separate "Danger Zone" section

#### Theme Toggle Integration

- **Severity:** 🟡 Minor
- **Location:** Appearance section vs Header
- **Viewport(s) affected:** All
- **Current behavior:** Theme can be changed in both places
- **Expected behavior:** Settings should be the primary, header is quick access

---

## Recommendations

### 1. Mobile Settings Navigation

**Change:** Use horizontal scrollable tabs or dropdown on mobile

**Specifications:**

```tsx
// Mobile: Horizontal scroll
<div className="md:hidden overflow-x-auto pb-2">
  <div className="flex gap-2 min-w-max">
    {tabs.map(tab => (
      <button
        key={tab.id}
        className={cn(
          "px-4 py-2 rounded-md text-sm whitespace-nowrap",
          activeTab === tab.id
            ? "bg-primary text-primary-foreground"
            : "bg-muted"
        )}
      >
        {tab.label}
      </button>
    ))}
  </div>
</div>

// Desktop: Vertical tabs
<aside className="hidden md:block w-48 shrink-0">
  {/* existing vertical tabs */}
</aside>
```

**Acceptance Criteria:**

- [ ] Tabs accessible on mobile
- [ ] No horizontal overflow cutting off text
- [ ] Active tab clearly visible

---

### 2. Add Form Field Borders

**Change:** Consistent borders on all inputs

**Specifications:**

```css
/* In globals.css or input component */
input,
select,
[role="combobox"] {
  @apply border border-input bg-background;
}

/* Focused state */
input:focus,
select:focus,
[role="combobox"]:focus {
  @apply ring-2 ring-ring ring-offset-2;
}
```

**Acceptance Criteria:**

- [ ] All inputs have visible 1px border
- [ ] Border color uses `--input` token
- [ ] Focus ring on interaction

---

### 3. Create Danger Zone Section

**Change:** Separate destructive actions

**Specifications:**

```tsx
<Card className="border-destructive/50 mt-8">
  <CardHeader>
    <CardTitle className="text-destructive">Danger Zone</CardTitle>
    <CardDescription>
      These actions are destructive and cannot be undone.
    </CardDescription>
  </CardHeader>
  <CardContent>
    <div className="flex items-center justify-between">
      <div>
        <p className="font-medium">Reset All Settings</p>
        <p className="text-sm text-muted-foreground">
          Restore all settings to their default values.
        </p>
      </div>
      <Button variant="destructive">Reset Settings</Button>
    </div>
  </CardContent>
</Card>
```

**Acceptance Criteria:**

- [ ] Clear visual separation from other settings
- [ ] Red/destructive border indicator
- [ ] Confirmation dialog on action

---

### 4. Add Success Feedback on Save

**Change:** Toast notification when settings are saved

**Specifications:**

```tsx
const handleSave = async () => {
  await saveSettings();
  toast.success("Settings saved", {
    description: "Your preferences have been updated.",
  });
};
```

**Acceptance Criteria:**

- [ ] Toast appears after save
- [ ] Message confirms what was saved
- [ ] Auto-dismiss after 3 seconds

---

## Measurements

| Element                    | Current | Recommended               |
| -------------------------- | ------- | ------------------------- |
| Settings content max-width | ~800px? | 640px for optimal reading |
| Card padding               | 16-24px | 24px consistent           |
| Form gap                   | Varies  | 24px between fields       |
| Tab width (desktop)        | ~180px  | 160px sufficient          |
| Input height               | 40px    | ✅ Good                   |

---

## Settings Sections Review

### General

| Setting      | Type       | Notes                |
| ------------ | ---------- | -------------------- |
| Tenant ID    | Text Input | Needs validation     |
| Working Mode | Select     | Hybrid/Local/Global  |
| Language     | Select     | Consider auto-detect |

### Appearance

| Setting      | Type          | Notes             |
| ------------ | ------------- | ----------------- |
| Theme        | Toggle/Select | Light/Dark/System |
| Accent Color | Color Picker? | If supported      |
| Font Size    | Select        | Accessibility     |

### Connection

| Setting        | Type         | Notes                |
| -------------- | ------------ | -------------------- |
| API URL        | Text Input   | Needs URL validation |
| Timeout        | Number Input | With units (ms/s)    |
| Auto-reconnect | Toggle       | Default on           |

### Advanced

| Setting     | Type   | Notes             |
| ----------- | ------ | ----------------- |
| Debug Mode  | Toggle | For developers    |
| Export Data | Button | Download settings |
| Clear Cache | Button | Destructive       |

---

## Responsive Behavior

### Mobile (320-428px)

- ⚠️ Tabs need horizontal scroll or dropdown
- ⚠️ Cards should be full-width with less padding
- ✅ Form elements should stack vertically

### Tablet (768px)

- ✅ Vertical tabs could work
- ⚠️ Consider 2-column for simple toggles

### Desktop (1280px+)

- ✅ Sidebar + tabs + content layout works well
- ✅ Plenty of space for descriptions
- ⚠️ Max-width prevents over-wide forms

---

## Accessibility

| Check                | Status   | Notes                   |
| -------------------- | -------- | ----------------------- |
| Form labels          | ✅ Good  | All inputs labeled      |
| Field descriptions   | ✅ Good  | Using CardDescription   |
| Error messages       | ⚠️ Check | Need aria-describedby   |
| Tab navigation       | ✅ Good  | Standard form flow      |
| Settings persistence | ✅ Good  | Uses localStorage       |
| Reset confirmation   | ⚠️ Needs | Add confirmation dialog |

---

## Form Validation

| Field     | Validation Needed                 |
| --------- | --------------------------------- |
| Tenant ID | Non-empty, alphanumeric           |
| API URL   | Valid URL format                  |
| Timeout   | Positive number, reasonable range |
| Language  | From supported list               |

```tsx
// Example validation with Zod
const settingsSchema = z.object({
  tenantId: z
    .string()
    .min(1)
    .regex(/^[a-zA-Z0-9_-]+$/),
  apiUrl: z.string().url(),
  timeout: z.number().min(1000).max(60000),
  language: z.enum(["en", "es", "fr", "de", "zh"]),
});
```

---

## State Management

Settings should:

- [ ] Persist to localStorage immediately
- [ ] Sync to backend if authenticated
- [ ] Show loading state during save
- [ ] Show error state on failure
- [ ] Support import/export

---

## Screenshots Reference

| State        | Breakpoint       | File                         |
| ------------ | ---------------- | ---------------------------- |
| General      | Desktop 1280px   | `05-settings-desktop.png`    |
| General      | Desktop L 1536px | `05-settings-desktop-l.png`  |
| General      | Tablet 768px     | `05-settings-tablet.png`     |
| General      | Mobile L 428px   | `05-settings-mobile-l.png`   |
| Appearance   | Desktop          | `05-settings-appearance.png` |
| Theme Toggle | Desktop          | `05-settings-theme.png`      |

---

_Last updated: December 25, 2025_
