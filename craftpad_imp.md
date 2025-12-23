# EdgeQuake WebUI Implementation Craftpad

**Date:** 2025-12-23
**Session:** UI/UX Improvements & Accessibility

## Issues to Address

### 1. ✅ Reprocess Message Display Formatting

- When regenerating a message, the content is not well formatted
- Need to investigate the reprocess/regenerate flow in query-interface

### 2. ⏳ Document Upload vs Process Pipeline Separation

- Current: Upload and process are combined, already has progress phases
- Backend: Already returns document_id with entity_count, relationship_count
- Toast: Already auto-dismisses after 3 seconds (line 245)
- TODO: Improve the toast UX with entity count display

### 3. ✅ Graph Floating Menu Improvements

- Fixed: "key 'graph.legend (en)' returned an object instead of string" error
- Changed to use t('graph.legend.title') instead of t('graph.legend')
- Hide/show node categories now works - using useFilteredNodes/useFilteredEdges

### 4. ⏳ Layout Best Practices Review

- Review all screens for consistent spacing, alignment, visual hierarchy

### 5. ⏳ Right Floating Menu Reopening

- Ensure the floating menu on the right can be reopened after closing

### 6. ⏳ Search Node Enhancement

- Compare with LightRAG search implementation
- Add better filtering, fuzzy search, result highlighting

### 7. ⏳ Accessibility (WCAG Compliance)

- [ ] Sufficient color contrast
- [ ] Readable font sizes
- [ ] Keyboard navigation support
- [ ] Screen-reader friendly structure (semantic HTML)
- [ ] Alt text for images
- [ ] Focus indicators
- [ ] ARIA labels where needed

---

## Current Work: Search Node Enhancement
