# UX/UI Improvement: Navigation & Layout

## Current State Analysis

### Sidebar Navigation

- **Logo Placement**: EdgeQuake logo links to `/graph` instead of a dedicated home page
- **Navigation Items**: 5 menu items (Knowledge Graph, Documents, Query, API Explorer, Settings)
- **Active State**: Good visual indicator with background color change
- **Version Info**: Version and platform info shown at bottom (good)

### Header

- **API Status Indicator**: Shows "API 0.1.0" or "Connecting..." - clear feedback
- **Utility Buttons**: Language, Theme, User menu - compact and accessible
- **Breadcrumb**: Present but could be more interactive

---

## UX Issues Identified

### Critical

1. **Logo Navigation Target**

   - **Issue**: EdgeQuake logo links to `/graph` instead of a proper landing/home page
   - **Impact**: Confusing for first-time users; no obvious entry point
   - **Recommendation**: Create a dedicated home/dashboard page with overview stats

2. **No Home/Dashboard Page**
   - **Issue**: Missing central dashboard showing system overview
   - **Impact**: Users can't see at-a-glance status (document count, entity count, graph stats)
   - **Recommendation**: Add `/` home route with:
     - Quick stats (documents, entities, relationships)
     - Recent activity
     - Quick actions (upload, query)
     - System health

### High Priority

3. **Sidebar Collapse Functionality**

   - **Issue**: Sidebar doesn't appear to be collapsible
   - **Impact**: Reduces screen real estate on smaller screens
   - **Recommendation**: Add toggle to collapse sidebar to icons-only mode

4. **Active Navigation State Consistency**

   - **Issue**: Active states use different styling patterns
   - **Impact**: Inconsistent visual feedback
   - **Recommendation**: Standardize active state with consistent color/indicator

5. **Breadcrumb Interactivity**
   - **Issue**: Some breadcrumb items are disabled/non-clickable
   - **Impact**: Breaks navigation expectations
   - **Recommendation**: Make all breadcrumb items clickable links

### Medium Priority

6. **Navigation Icons**

   - **Issue**: Icons are small and may be hard to distinguish
   - **Impact**: Accessibility concern
   - **Recommendation**: Ensure minimum touch target of 44x44px

7. **Version Information Prominence**
   - **Issue**: Version shown at bottom of sidebar may be missed
   - **Impact**: Users may not know which version they're using
   - **Recommendation**: Consider adding to header or settings page

---

## Recommendations

### Short Term (Sprint 1)

- [ ] Create home/dashboard page with system overview
- [ ] Make logo link to home page
- [ ] Enable all breadcrumb items as links

### Medium Term (Sprint 2)

- [ ] Add sidebar collapse/expand toggle
- [ ] Implement consistent active state styling
- [ ] Add skip navigation link for accessibility

### Long Term

- [ ] Add keyboard navigation support (Tab, Arrow keys)
- [ ] Implement responsive sidebar for mobile
- [ ] Add navigation history/recent pages

---

## Visual Mockup Suggestions

```
┌─────────────────────────────────────────────────────────────┐
│  🏠 EdgeQuake  [≡]                    API ● │ 🌐 │ ☀ │ 👤   │
├──────────────┬──────────────────────────────────────────────┤
│  Dashboard   │  EdgeQuake > Dashboard                       │
│  ─────────── │  ┌────────────────────────────────────────┐  │
│ 📊 Overview  │  │ Welcome to EdgeQuake                   │  │
│ 📄 Documents │  │                                        │  │
│ 🔍 Query     │  │  [12] Documents  [156] Entities       │  │
│ 🌐 Graph     │  │  [45] Relations  [✓] API Connected    │  │
│ 🔧 Settings  │  │                                        │  │
│              │  │ Quick Actions:                         │  │
│              │  │  [Upload] [Query] [View Graph]        │  │
│              │  └────────────────────────────────────────┘  │
│  ─────────── │                                              │
│  v0.1.0      │  Recent Activity:                            │
│  Graph-RAG   │  • document.md processed (2 min ago)        │
└──────────────┴──────────────────────────────────────────────┘
```

---

## Acceptance Criteria

- [ ] Home page shows system statistics
- [ ] Logo navigates to home page
- [ ] All navigation items have clear active states
- [ ] Breadcrumbs are fully interactive
- [ ] Sidebar can be collapsed on smaller screens
- [ ] Navigation is keyboard accessible
