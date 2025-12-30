# Query Screen Audit

**Route:** `/query`  
**Viewport(s) Tested:** 320px, 428px, 768px, 1280px, 1536px  
**UI Regions:** Header, Sidebar, Breadcrumb, Query Interface, Mode Selector, Conversation Area, History Panel  
**States Captured:** Initial, With Input, Focus State, Loading  
**Screenshots:** `screenshots/screens/query/`  
**Relevant Files:** `src/app/(dashboard)/query/page.tsx`, `src/components/query/query-interface.tsx`

---

## What I Reviewed

### Layout Structure

```
┌─────────────────────────────────────────────────────────┐
│ Header (fixed, h: 64px)                    │ API │ 🌐 ☀️ 👤│
├────────────┬────────────────────────────────┬───────────┤
│ Sidebar    │ Breadcrumb: EdgeQuake > Query  │           │
│ w: 256px   ├────────────────────────────────┤ Historique│
│            │ ┌────────────────────────────┐ │           │
│            │ │ Requête          │+New │Mode│ │ + ▸      │
│            │ │ Posez des questions...     │ │ 🔍 Search │
│            │ ├────────────────────────────┤ │           │
│            │ │      ✨                     │ │ Chat 1   │
│            │ │  Ask about your KG         │ │ Chat 2   │
│            │ │                            │ │ Chat 3   │
│            │ │  ⚠️ No documents yet       │ │ ...      │
│            │ │     [Upload]               │ │           │
│            │ │                            │ │           │
│            │ │  Try asking:               │ │           │
│            │ │  ┌──────┐ ┌──────┐         │ │           │
│            │ │  │Prompt│ │Prompt│         │ │           │
│            │ │  └──────┘ └──────┘         │ │           │
│            │ │  ┌──────┐ ┌──────┐         │ │           │
│            │ │  │Prompt│ │Prompt│         │ │           │
│            │ │  └──────┘ └──────┘         │ │           │
│            │ ├────────────────────────────┤ │           │
│            │ │ [Posez une question...  ▶] │ │           │
│            │ │ Press Enter to send...     │ │           │
│            │ └────────────────────────────┘ │           │
└────────────┴────────────────────────────────┴───────────┘
```

---

## Slickness Score

| Criterion           | Score (1–5) | Notes                                              |
| ------------------- | ----------- | -------------------------------------------------- |
| Visual refinement   | 4.5         | Beautiful gradient avatar, clean cards             |
| Modern styling      | 4.5         | Excellent chat-like interface                      |
| Smooth interactions | 4.0         | Good loading animation, mode selector is smooth    |
| Professional polish | 4.5         | Great empty state, helpful prompts                 |
| **Overall**         | **4.4**     | Best screen in the app, minor polish opportunities |

---

## Issues

### � Major (P1)

#### History Panel UX Improvements Needed

- **Priority:** P1 - Major
- **Severity:** 🟠 Major
- **Location:** Conversation History Panel (right side)
- **Viewport(s) affected:** Desktop
- **Discovered:** December 30, 2025 (scroll audit)

| Issue | Current State | Recommended Fix |
|-------|---------------|-----------------|
| Container padding | `0px` | Add `p-2` (8px) for breathing room |
| Scroll area padding | `0px` | Add `px-2` for list items |
| Conversation item padding | `6px 12px` | Increase to `8px 16px` for better touch target |
| List role | Missing | Add `role="listbox"` for accessibility |
| Item role | Missing | Add `role="option"` on each conversation |
| Keyboard navigation | Tab only | Add arrow key navigation between items |
| Mouse wheel scroll | ✅ Works | Scroll container has `overflow-auto` |

**Impact:**
- Items feel cramped with minimal padding
- No keyboard navigation beyond Tab
- Accessibility issues (missing ARIA roles)

**Recommended Fix:**
```tsx
// Improve conversation list
<div 
  role="listbox" 
  aria-label="Conversation history"
  className="flex-1 overflow-auto p-2"  // Add padding
  onKeyDown={(e) => {
    if (e.key === 'ArrowDown') selectNext();
    if (e.key === 'ArrowUp') selectPrevious();
  }}
>
  {conversations.map(conv => (
    <button
      role="option"
      aria-selected={isSelected}
      className="w-full px-4 py-2 mb-1 rounded-md"  // Better spacing
    >
      {conv.name}
    </button>
  ))}
</div>
```

---

### �🟡 Minor

#### History Panel Search Has No Clear Indicator

- **Severity:** 🟡 Minor
- **Location:** Conversation history panel
- **Viewport(s) affected:** Desktop
- **Current behavior:** Search input looks like regular input
- **Expected behavior:** Add search icon and clear button

#### Mode Selector Buttons Could Be Grouped Better

- **Severity:** 🟡 Minor
- **Location:** Mode selector (Local | Global | Hybrid | Simple)
- **Viewport(s) affected:** All
- **Current behavior:** Individual buttons with gaps
- **Expected behavior:** Button group with connected borders

#### Suggested Prompts Could Have Hover Animation

- **Severity:** 🟡 Minor
- **Location:** "Try asking" section
- **Viewport(s) affected:** Desktop
- **Current behavior:** No hover feedback
- **Expected behavior:** Subtle lift or background change

#### History Panel Collapse Not Obvious

- **Severity:** 🟡 Minor
- **Location:** History panel header
- **Viewport(s) affected:** Desktop
- **Current behavior:** Small collapse button
- **Expected behavior:** More prominent toggle or drag handle

#### Input Focus Ring Could Be More Prominent

- **Severity:** 🟡 Minor
- **Location:** Query textarea
- **Viewport(s) affected:** All
- **Current behavior:** Standard focus ring
- **Expected behavior:** More visible focus state with brand color

---

## Positive Observations

### ✨ Excellent Elements

1. **Loading Animation:** The thinking dots and shimmer effect are delightful
2. **Message Bubbles:** User vs assistant differentiation is clear
3. **Thinking Section:** Collapsible reasoning display is innovative
4. **Avatar Design:** Gradient purple avatar for AI is distinctive
5. **Empty State:** Clear messaging with upload CTA
6. **Suggested Prompts:** Helpful for new users

---

## Recommendations

### 1. Enhance Suggested Prompt Cards

**Change:** Add interactive hover states to prompt suggestions

**Specifications:**

```tsx
<button
  className={cn(
    "flex items-center gap-2 p-4 rounded-lg border",
    "text-left text-sm",
    "transition-all duration-200",
    "hover:border-primary/50 hover:bg-muted/50",
    "hover:-translate-y-0.5 hover:shadow-sm"  // Lift effect
  )}
>
```

**Animation:**

- Transform: translateY(-2px)
- Shadow: Add subtle shadow
- Duration: 200ms

**Acceptance Criteria:**

- [ ] Cards lift on hover
- [ ] Border color changes
- [ ] Smooth transition

---

### 2. Group Mode Selector Buttons

**Change:** Create a connected button group

**Specifications:**

```tsx
<div className="inline-flex rounded-lg border overflow-hidden">
  <button
    className={cn(
      "px-3 py-1.5 text-sm",
      "border-r", // Divider between buttons
      isActive && "bg-primary text-primary-foreground"
    )}
  >
    Local
  </button>
  // ... more buttons, last one without border-r
</div>
```

**Applies to:** Mode selector

**Acceptance Criteria:**

- [ ] Buttons appear connected
- [ ] Active state clearly visible
- [ ] Keyboard navigation works

---

### 3. Enhance Query Input Focus State

**Change:** Make focus state more prominent

**Specifications:**

```tsx
<Textarea
  className={cn(
    "...",
    "focus:ring-2 focus:ring-primary/30",
    "focus:border-primary",
    "transition-all duration-200"
  )}
/>
```

**Applies to:** Main query textarea

**Acceptance Criteria:**

- [ ] Focus ring is clearly visible
- [ ] Uses brand color
- [ ] Smooth transition

---

### 4. Add History Panel Improvements

**Change:** Improve history panel UX

**Specifications:**

- Add search icon to input
- Add clear button when search has content
- Make collapse button larger (44×44px)
- Add tooltip to collapse button

**Acceptance Criteria:**

- [ ] Search has icon and clear button
- [ ] Collapse button meets touch target size
- [ ] Tooltip shows "Collapse history"

---

## Measurements

| Element             | Current          | Recommended |
| ------------------- | ---------------- | ----------- |
| History panel width | ~300px           | ✅ Good     |
| Query input height  | Auto (min ~80px) | ✅ Good     |
| Message max width   | 85%              | ✅ Good     |
| Prompt card padding | 16px             | ✅ Good     |
| Section spacing     | 16-24px          | ✅ Good     |
| Avatar size         | 32×32px          | ✅ Good     |

---

## Responsive Behavior

### Mobile (320-428px)

- ⚠️ History panel should be hidden or sheet-based
- ✅ Query input at bottom works well
- ✅ Suggested prompts stack vertically
- ⚠️ Mode selector might overflow

### Tablet (768px)

- ✅ Good layout balance
- ⚠️ History panel could be collapsible

### Desktop (1280px+)

- ✅ Three-column layout works
- ✅ Ample space for conversation
- ✅ History visible and useful

---

## Accessibility

| Check                  | Status        | Notes                              |
| ---------------------- | ------------- | ---------------------------------- |
| Form labeling          | ✅ Good       | Textarea has aria-label            |
| Submit button          | ✅ Good       | Disabled when empty                |
| Loading states         | ✅ Good       | Uses aria-busy                     |
| Conversation announced | ⚠️ Consider   | Add aria-live for new messages     |
| Keyboard shortcuts     | ✅ Documented | Enter to send, Shift+Enter newline |

---

## Streaming UX Quality

The streaming implementation is excellent:

| Feature            | Status       | Notes                         |
| ------------------ | ------------ | ----------------------------- |
| Thinking indicator | ✅ Excellent | Animated brain icon with ping |
| Progress shimmer   | ✅ Excellent | Three-line shimmer effect     |
| Cursor blink       | ✅ Good      | Shows streaming in progress   |
| Stop button        | ✅ Present   | Can cancel streaming          |
| Token count        | ✅ Good      | Shows tokens used             |

---

## Screenshots Reference

| State           | Breakpoint       | File                             |
| --------------- | ---------------- | -------------------------------- |
| Initial         | Desktop 1280px   | `03-query-initial-desktop.png`   |
| Initial         | Desktop L 1536px | `03-query-initial-desktop-l.png` |
| Initial         | Tablet 768px     | `03-query-initial-tablet.png`    |
| Initial         | Mobile L 428px   | `03-query-initial-mobile-l.png`  |
| With Input      | Desktop          | `03-query-with-input.png`        |
| Focus State     | Desktop          | `03-query-focus-state.png`       |
| Input Component | Desktop          | `03-query-input.png`             |
| Right Panel     | Desktop          | `03-query-right-panel.png`       |

---

---

## December 30, 2025 - Live Testing Update

### Testing with Real Query

Performed live query against indexed document (AI safety research paper with 794 entities).

**Query tested:** "What are the main LLM vulnerabilities?"

#### ✅ Verified Features

1. **Streaming Response Works Flawlessly**

   - Response streams in real-time with smooth character-by-character appearance
   - Cursor blink visible during streaming
   - "Thinking" indicator shows appropriately
   - Response renders markdown correctly (headers, lists, bold text)
   - Formatted output is well-spaced and readable

2. **History Panel Collapse**

   - Right panel collapses smoothly
   - Collapsed state shows vertical "Conversation History" label
   - Panel width transitions from ~300px to ~40px
   - Content in main area expands correctly

3. **Conversation Flow**

   - User message appears instantly with distinct styling
   - AI response streams below with gradient avatar
   - Clear visual separation between messages
   - Scroll-to-bottom works during streaming

4. **Mode Selector**
   - All four modes visible (Local, Global, Hybrid, Simple)
   - Active mode highlighted correctly
   - Switching modes works immediately

#### 🟡 Minor Issues Observed

1. **Mobile Layout (320px)**

   - History panel hidden correctly (as expected)
   - Input area at bottom works well
   - Mode selector slightly cramped but functional
   - **Observation:** Consider hiding mode selector on mobile or moving to settings

2. **Scroll Behavior**
   - Main conversation area scrolls correctly
   - Auto-scroll to bottom works during streaming
   - Fixed zones (header, input bar) remain in place ✅

#### Fixed Zone Verification (VERY IMPORTANT)

| Zone          | Behavior                            | Status     |
| ------------- | ----------------------------------- | ---------- |
| Header        | Fixed at top, 64px                  | ✅ Correct |
| Input Area    | Fixed at bottom                     | ✅ Correct |
| Sidebar       | Fixed on left                       | ✅ Correct |
| History Panel | Fixed on right, scrolls internally  | ✅ Correct |
| Conversation  | Scrollable area between fixed zones | ✅ Correct |

### Updated Screenshots (Dec 30)

| State             | Breakpoint     | File                                  |
| ----------------- | -------------- | ------------------------------------- |
| Initial           | Desktop 1280px | `query-desktop-1280-initial.png`      |
| With Response     | Desktop 1280px | `query-desktop-with-response.png`     |
| History Collapsed | Desktop        | `query-desktop-history-collapsed.png` |
| Mobile            | 320px          | `query-mobile-320.png`                |

### Updated Score

| Criterion           | Previous | Current  | Change    |
| ------------------- | -------- | -------- | --------- |
| Visual refinement   | 4.5      | 4.6      | +0.1      |
| Modern styling      | 4.5      | 4.6      | +0.1      |
| Smooth interactions | 4.0      | 4.5      | +0.5      |
| Professional polish | 4.5      | 4.6      | +0.1      |
| **Overall**         | **4.4**  | **4.58** | **+0.18** |

**Note:** Query page remains the best-implemented screen. Streaming UX is excellent, markdown rendering is clean, and the chat-style interface feels modern and responsive.

---

_Last updated: December 30, 2025_
