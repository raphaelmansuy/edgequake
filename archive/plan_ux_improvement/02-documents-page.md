# UX/UI Improvement: Documents Page

## Current State Analysis

### Page Structure

- **Header**: Title with description and action buttons (Refresh, Clear All)
- **Filters**: Status dropdown, Sort by (Created/Updated)
- **Upload Zone**: Drag & drop area with file type hints
- **Table**: Document list with columns (Title, Status, Entities, Created, Actions)
- **Pagination**: Rows per page selector, page navigation

### Positive Observations

- Clear call-to-action for upload
- Status badges with icons (Completed ✓, Processing ⟳)
- Entity count visible per document
- Relative timestamps (e.g., "3 minutes ago")

---

## UX Issues Identified

### Critical

1. **Ghost Documents Bug (FIXED)**

   - **Issue**: Extra documents appeared in table after upload
   - **Status**: ✅ Fixed - was counting `-content` keys as documents
   - **Root Cause**: `list_documents` function was including storage keys with `-content` suffix

2. **No Document Preview/Details**
   - **Issue**: Clicking a document row doesn't open details view
   - **Impact**: Users can't see document content, entities extracted, or processing details
   - **Recommendation**: Add document detail drawer or modal

### High Priority

3. **Upload Progress Feedback**

   - **Issue**: Batch progress card appears but may not always show completion status clearly
   - **Recommendation**:
     - Add success animation/celebration on completion
     - Show entity count immediately after processing
     - Auto-dismiss after 5 seconds with "View Details" link

4. **"Clear All" Button Position**

   - **Issue**: Destructive action (Clear All) is positioned prominently
   - **Impact**: Risk of accidental data deletion
   - **Recommendation**:
     - Move to overflow menu (⋮)
     - Add confirmation dialog
     - Show document count in confirmation

5. **Status Filter UX**
   - **Issue**: "Tous les statuts (1)" - count in dropdown is unclear
   - **Impact**: Users may not understand what the number means
   - **Recommendation**: Use "All Statuses" with count below or separate stats

### Medium Priority

6. **Table Row Actions**

   - **Issue**: Only shows kebab menu (⋮) with unclear options
   - **Impact**: Users don't know what actions are available
   - **Recommendation**:
     - Add visible quick actions (View, Delete)
     - Show on hover for cleaner initial view

7. **Empty State**

   - **Issue**: When no documents, shows "Documents (0)" with no guidance
   - **Impact**: New users don't know what to do
   - **Recommendation**: Add helpful empty state with:
     - Illustration
     - "Get started by uploading your first document"
     - Supported file types list
     - Sample document option

8. **Sort Controls**

   - **Issue**: "Trier par:" followed by buttons that look like tabs
   - **Impact**: Unclear that these are toggle buttons
   - **Recommendation**: Use proper segmented control or dropdown

9. **File Type Support Clarity**
   - **Issue**: "Supports TXT, MD, JSON files" - limited info
   - **Impact**: Users may try unsupported files
   - **Recommendation**:
     - List maximum file size
     - Show PDF coming soon (if planned)
     - Add tooltip with more details

### Low Priority

10. **Pagination Text**

    - **Issue**: "Lignes par page : 20" in French while "Page 1 sur 1" mixed
    - **Impact**: Inconsistent localization
    - **Recommendation**: Ensure full i18n coverage

11. **Responsive Table**
    - **Issue**: Table may overflow on mobile
    - **Impact**: Poor mobile experience
    - **Recommendation**: Add card view for mobile

---

## Recommendations

### Short Term (Sprint 1)

- [ ] Add confirmation dialog for "Clear All"
- [ ] Create empty state design
- [ ] Add document detail view (drawer)
- [ ] Improve batch progress card completion state

### Medium Term (Sprint 2)

- [ ] Redesign filter/sort controls
- [ ] Add visible row actions on hover
- [ ] Implement card view for mobile
- [ ] Add document search

### Long Term

- [ ] Add bulk selection and actions
- [ ] Implement document tagging/categorization
- [ ] Add export functionality (JSON, CSV)
- [ ] Show entity relationship preview

---

## Wireframe: Document Detail Drawer

```
┌─────────────────────────────────────────────────────────────┐
│ ← Back to Documents                            [✕] Close    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  📄 knowledge_test.md                                       │
│  ─────────────────────────────────────────────────────────  │
│  Status: ✓ Completed        Created: Dec 23, 2024 2:44 PM  │
│  Entities: 11               Size: 329 bytes                │
│                                                             │
│  ┌─ Content Preview ─────────────────────────────────────┐  │
│  │ Dr. Sarah Chen leads the AI Research Lab at Stanford  │  │
│  │ University. She works with Dr. Michael Wong (NLP      │  │
│  │ specialist), Emily Johnson (research scientist)...    │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌─ Extracted Entities ──────────────────────────────────┐  │
│  │ 👤 Dr. Sarah Chen (Person)     🏢 Stanford (Org)      │  │
│  │ 👤 Dr. Michael Wong (Person)   🏢 OpenAI (Org)        │  │
│  │ 👤 Emily Johnson (Person)      🏢 Google DeepMind     │  │
│  │ 📁 GraphRAG (Project)                                 │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  [View in Graph]  [Reprocess]  [Delete Document]            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Wireframe: Empty State

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│                    📂                                       │
│              No Documents Yet                               │
│                                                             │
│     Get started by uploading your first document.          │
│     We'll automatically extract entities and build         │
│     your knowledge graph.                                   │
│                                                             │
│              ┌─────────────────────────┐                   │
│              │  ⬆ Upload Documents     │                   │
│              └─────────────────────────┘                   │
│                                                             │
│     Supported: .txt, .md, .json (up to 10MB)               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Acceptance Criteria

- [ ] Document detail drawer shows content preview and entities
- [ ] Empty state provides clear guidance
- [ ] Clear All requires confirmation
- [ ] Table is responsive on mobile
- [ ] All text is properly localized
- [ ] Batch progress shows success state clearly
