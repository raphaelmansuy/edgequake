# UX/UI Improvement: Settings Page

## Current State Analysis

### Page Structure

- **Appearance Section**: Theme and Language selectors
- **Graph Visualization Section**: Node labels, edge labels, size, layout
- **Query Defaults Section**: Default mode, streaming toggle
- **Data Management Section**: Clear history, reset settings

### Positive Observations

- Clean sectioned layout with icons
- Toggle switches for boolean options
- Dropdown selectors for choice options
- Clear destructive action buttons

---

## UX Issues Identified

### High Priority

1. **No Save Confirmation**

   - **Issue**: Settings change immediately without feedback
   - **Impact**: Users unsure if changes were saved
   - **Recommendation**:
     - Show toast "Settings saved" on change
     - Or add explicit Save button with unsaved indicator

2. **Theme/Language Change Delay**

   - **Issue**: Theme changes immediately but may flash
   - **Impact**: Jarring transition
   - **Recommendation**:
     - Add smooth transition animation
     - Persist preference to localStorage

3. **Missing Settings**

   - **Issue**: Limited configuration options
   - **Impact**: Power users can't customize fully
   - **Recommendation**: Add:
     - API endpoint configuration
     - LLM model selection
     - Upload file size limits
     - Processing timeout settings

4. **Graph Settings Preview**
   - **Issue**: Can't preview graph settings changes
   - **Impact**: Must navigate away to see effects
   - **Recommendation**:
     - Add small preview graph
     - Or apply changes in real-time

### Medium Priority

5. **Reset Settings Confirmation**

   - **Issue**: "Reset Settings" button has no confirmation
   - **Impact**: Accidental reset possible
   - **Recommendation**:
     - Add confirmation dialog
     - Show what will be reset
     - Option to export current settings first

6. **Clear History Scope**

   - **Issue**: "Clear all saved query history" - what about favorites?
   - **Impact**: Ambiguous behavior
   - **Recommendation**:
     - Clarify: "This will delete X queries and Y favorites"
     - Option to keep favorites
     - Confirm action

7. **Section Collapsibility**

   - **Issue**: All sections always expanded
   - **Impact**: Long scroll on mobile
   - **Recommendation**:
     - Make sections collapsible
     - Remember expanded state

8. **Toggle Label Placement**
   - **Issue**: Labels on left, toggles on right - wide gap
   - **Impact**: Hard to associate label with control
   - **Recommendation**:
     - Reduce row width or
     - Add connecting line/background

### Low Priority

9. **Settings Import/Export**

   - **Issue**: No way to backup or share settings
   - **Impact**: Multi-device users must reconfigure
   - **Recommendation**:
     - Export as JSON
     - Import settings file
     - Sync with account (future)

10. **Keyboard Navigation**

    - **Issue**: Tab order may not be optimal
    - **Impact**: Accessibility concern
    - **Recommendation**:
      - Ensure logical tab order
      - Add keyboard hints

11. **Mobile Layout**

    - **Issue**: Full-width design may waste space on desktop
    - **Impact**: Unbalanced layout
    - **Recommendation**:
      - Max-width container
      - Two-column on wide screens

12. **Search Settings**
    - **Issue**: No way to search for specific setting
    - **Impact**: Users must scroll to find options
    - **Recommendation**:
      - Add search/filter for settings
      - Jump to section links

---

## Recommendations

### Short Term (Sprint 1)

- [ ] Add save confirmation toast
- [ ] Add confirmation for destructive actions
- [ ] Smooth theme transition

### Medium Term (Sprint 2)

- [ ] Add more configuration options (API, LLM)
- [ ] Make sections collapsible
- [ ] Add settings preview for graph

### Long Term

- [ ] Settings import/export
- [ ] Per-document/workspace settings
- [ ] Advanced/Developer settings section

---

## Wireframe: Improved Settings with Confirmations

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  Settings                                                   │
│  Customize your EdgeQuake experience                        │
│                                                             │
│  ┌─ 🎨 Appearance ────────────────────────────────── [▼] ─┐ │
│  │                                                        │ │
│  │  Theme           [Light ▼]     ← "Theme updated" ✓     │ │
│  │                                                        │ │
│  │  Language        [English ▼]                           │ │
│  │                                                        │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌─ 🗑️ Data Management ──────────────────────────── [▼] ─┐ │
│  │                                                        │ │
│  │  Clear Query History                                   │ │
│  │  └─ You have 5 queries and 2 favorites                │ │
│  │                              [Clear History]           │ │
│  │                                                        │ │
│  │  Reset All Settings                                    │ │
│  │  └─ This will reset 12 settings to defaults           │ │
│  │                              [Reset Settings]          │ │
│  │                                                        │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Wireframe: Confirmation Dialog

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│            ⚠️ Reset All Settings?                          │
│                                                             │
│  This will reset all settings to their default values:     │
│                                                             │
│  • Theme → Light                                            │
│  • Language → English                                       │
│  • Node labels → On                                         │
│  • Edge labels → Off                                        │
│  • Query mode → Naive                                       │
│  ... and 7 more settings                                    │
│                                                             │
│  ┌────────────────────────────────────────────────────┐    │
│  │ ☐ Export current settings before resetting        │    │
│  └────────────────────────────────────────────────────┘    │
│                                                             │
│              [Cancel]          [Reset Settings]             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## New Settings to Add

### API Configuration

```
┌─ 🔌 API Configuration ─────────────────────────────────────┐
│                                                            │
│  API Endpoint         [http://localhost:8080 ▼]            │
│                        └─ Custom endpoint...               │
│                                                            │
│  Request Timeout      [30 seconds ▼]                       │
│                                                            │
│  Auto-retry on error  [○───]                               │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### LLM Configuration

```
┌─ 🤖 LLM Configuration ─────────────────────────────────────┐
│                                                            │
│  LLM Provider         [OpenAI ▼]                           │
│                                                            │
│  Model                [gpt-4o-mini ▼]                      │
│                                                            │
│  Temperature          [0.7] ───●───────                    │
│                                                            │
│  Max Tokens           [4096 ▼]                             │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

---

## Acceptance Criteria

- [ ] Settings changes show confirmation toast
- [ ] Destructive actions require confirmation dialog
- [ ] Theme transitions smoothly without flash
- [ ] Sections are collapsible
- [ ] Graph preview shows setting effects
- [ ] Settings can be exported/imported
- [ ] Mobile layout is optimized
