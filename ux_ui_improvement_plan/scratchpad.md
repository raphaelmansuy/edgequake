# UX/UI Improvement Scratchpad

## Analysis Notes

### Current State Observations

1. **Header (64px)**

   - Contains: Mobile menu, API status, language, theme, user menu
   - Too much vertical space consumed
   - Can be reduced to 48px with tighter icon spacing

2. **Sidebar (256px expanded, 64px collapsed)**

   - Contains tenant selector which takes ~60px vertically
   - Navigation items well-organized
   - Tenant selector should move to header for better UX

3. **Graph Page Right Panel (320px fixed)**

   - Not resizable - users may want more/less space
   - Node details shown in scrollable card
   - Filters take up top section

4. **Node Details**

   - Uses Card component with fixed scroll heights
   - Should integrate seamlessly with panel
   - Scroll areas need review for long content

5. **Breadcrumb Bar (~48px)**
   - Background: muted/30
   - Border bottom
   - Can be more compact (36px)

### Design Principles Applied

1. **SLICK**: Smooth transitions, subtle shadows, refined spacing
2. **MINIMAL**: Remove visual clutter, essential elements only
3. **ACCESSIBLE**: 44px touch targets, focus states, contrast

### Implementation Order

1. Header compact + tenant selector move (biggest visual impact)
2. Right panel resizable (functional improvement)
3. Node details scroll optimization
4. Padding/margin refinements
5. Final polish pass
