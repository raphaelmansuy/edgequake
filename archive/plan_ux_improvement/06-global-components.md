# UX/UI Improvement: Global Components & Patterns

## Header Bar

### Current State

- API status indicator (green dot + version)
- Language toggle (globe icon)
- Theme toggle (sun/moon icon)
- User menu (avatar icon)

### Issues

1. **API Status Clarity**

   - **Issue**: "API 0.1.0" doesn't clearly indicate connection status
   - **Recommendation**:
     - "Connected" with green dot
     - "Disconnected" with red dot and retry option
     - Show latency on hover

2. **User Menu Empty**

   - **Issue**: Clicking user icon shows empty or minimal menu
   - **Recommendation**:
     - Add user profile section
     - Add logout option
     - Add quick settings

3. **Notification Area**
   - **Issue**: "Notifications alt+T" region exists but unclear
   - **Recommendation**:
     - Add notification bell icon
     - Show unread count badge
     - List recent system notifications

---

## Toast Notifications

### Current State

- Success toasts appear for uploads
- Positioned in bottom-right corner
- Auto-dismiss behavior

### Issues

1. **Toast Stacking**

   - **Issue**: Multiple toasts may overlap
   - **Recommendation**:
     - Stack vertically with spacing
     - Limit to 3 visible, queue others
     - Add "Clear All" when multiple

2. **Toast Types**

   - **Issue**: Only success type visible
   - **Recommendation**:
     - Error toasts (red) for failures
     - Warning toasts (yellow) for attention
     - Info toasts (blue) for neutral

3. **Toast Actions**
   - **Issue**: Toasts are passive (close only)
   - **Recommendation**:
     - Add action buttons ("View", "Retry")
     - Add undo option where applicable

---

## Loading States

### Issues

1. **Page Loading**

   - **Issue**: No consistent loading indicator across pages
   - **Recommendation**:
     - Add top progress bar
     - Or page skeleton loaders
     - Maintain layout during load

2. **Button Loading**

   - **Issue**: Buttons don't show loading state
   - **Recommendation**:
     - Disable + spinner during action
     - Maintain button width
     - Show completion checkmark

3. **Data Fetching**
   - **Issue**: API calls don't show loading in UI
   - **Recommendation**:
     - Skeleton loaders for tables
     - Spinner for small areas
     - "Refreshing..." text for subtle updates

---

## Error States

### Issues

1. **API Connection Error**

   - **Issue**: "Connecting..." shown indefinitely on failure
   - **Recommendation**:
     - Show error state after timeout
     - Offer "Retry" button
     - Show troubleshooting tips

2. **Form Validation**

   - **Issue**: No visible validation errors
   - **Recommendation**:
     - Inline error messages
     - Field highlighting (red border)
     - Form-level error summary

3. **Empty API Responses**
   - **Issue**: Empty responses may show blank areas
   - **Recommendation**:
     - Consistent empty state messaging
     - Actionable guidance

---

## Accessibility (a11y)

### Issues

1. **Color Contrast**

   - **Issue**: Light gray text may not meet WCAG AA
   - **Recommendation**:
     - Audit contrast ratios
     - Ensure 4.5:1 minimum for text
     - 3:1 for large text and icons

2. **Focus Indicators**

   - **Issue**: Focus rings may not be visible
   - **Recommendation**:
     - Clear focus outline on all interactive elements
     - Consistent focus style

3. **Screen Reader Support**

   - **Issue**: Some elements may lack proper labels
   - **Recommendation**:
     - Audit with screen reader
     - Add aria-labels where needed
     - Ensure heading hierarchy

4. **Keyboard Navigation**
   - **Issue**: Not all features keyboard accessible
   - **Recommendation**:
     - Tab through all interactive elements
     - Escape to close modals
     - Skip links for main content

---

## Responsive Design

### Issues

1. **Mobile Sidebar**

   - **Issue**: Sidebar may not collapse on mobile
   - **Recommendation**:
     - Hamburger menu on mobile
     - Drawer overlay
     - Bottom navigation alternative

2. **Table Responsiveness**

   - **Issue**: Tables may overflow on small screens
   - **Recommendation**:
     - Card view on mobile
     - Horizontal scroll with shadow hint
     - Priority columns

3. **Form Controls**
   - **Issue**: Dropdowns may be hard to tap
   - **Recommendation**:
     - Minimum 44px touch targets
     - Full-width on mobile
     - Native select on mobile

---

## Localization (i18n)

### Issues

1. **Incomplete Translation**

   - **Issue**: Some text in English, some in French
   - **Recommendation**:
     - Audit all strings
     - Complete translation files
     - Add missing keys

2. **RTL Support**

   - **Issue**: May not support right-to-left languages
   - **Recommendation**:
     - Use CSS logical properties
     - Test with RTL language
     - Mirror layouts appropriately

3. **Date/Number Formatting**
   - **Issue**: May not use locale-appropriate formats
   - **Recommendation**:
     - Use Intl.DateTimeFormat
     - Use Intl.NumberFormat
     - Respect user locale

---

## Performance

### Issues

1. **Bundle Size**

   - **Issue**: Initial load may be large
   - **Recommendation**:
     - Lazy load routes
     - Tree shake unused code
     - Compress assets

2. **Graph Performance**

   - **Issue**: Large graphs may cause jank
   - **Recommendation**:
     - Virtual rendering for large datasets
     - Web workers for computation
     - Progressive loading

3. **API Caching**
   - **Issue**: May refetch data unnecessarily
   - **Recommendation**:
     - React Query caching (already in use)
     - Stale-while-revalidate strategy
     - Optimistic updates

---

## Recommendations Summary

### Immediate (Sprint 1)

- [ ] Add consistent loading states
- [ ] Improve error handling and messages
- [ ] Complete i18n coverage
- [ ] Add focus indicators for accessibility

### Short Term (Sprint 2)

- [ ] Implement toast stacking and types
- [ ] Add page skeleton loaders
- [ ] Mobile responsive audit
- [ ] Improve API status indicator

### Medium Term (Sprint 3)

- [ ] Full accessibility audit (WCAG AA)
- [ ] RTL language support
- [ ] Performance optimization
- [ ] Add notification center

---

## Design System Recommendations

### Color Tokens

```css
/* Status Colors */
--color-success: #22c55e;
--color-warning: #f59e0b;
--color-error: #ef4444;
--color-info: #3b82f6;

/* Entity Colors */
--color-person: #3b82f6;
--color-organization: #22c55e;
--color-project: #6b7280;
--color-location: #f59e0b;
```

### Spacing Scale

```css
--space-1: 4px;
--space-2: 8px;
--space-3: 12px;
--space-4: 16px;
--space-6: 24px;
--space-8: 32px;
--space-12: 48px;
```

### Typography

```css
--font-size-xs: 12px;
--font-size-sm: 14px;
--font-size-base: 16px;
--font-size-lg: 18px;
--font-size-xl: 20px;
--font-size-2xl: 24px;
```

---

## Acceptance Criteria

- [ ] Consistent loading states across app
- [ ] Error messages are clear and actionable
- [ ] All text is translated
- [ ] WCAG AA compliance
- [ ] Mobile responsive
- [ ] Toast notifications stack properly
- [ ] Focus indicators visible
- [ ] Keyboard navigation works
