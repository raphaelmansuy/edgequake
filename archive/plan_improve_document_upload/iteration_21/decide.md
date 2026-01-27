# Iteration 21: Document Preview Error Enhancement - Decide

## Decision

### Integrate Error Categorization
Use the error-categories.ts utility from OODA-09 to provide:
- Category-specific icons and colors
- User-friendly summary
- Actionable suggestions
- Retryable indicator with retry button
- Technical details in collapsible section

### Implementation Approach
1. Import categorization utilities
2. Add useMemo for error categorization
3. Create helper function for category icons
4. Replace simple error display with enhanced version

### Code Structure
- errorInfo useMemo: Categorize error on mount
- getCategoryIconComponent: Map category to Lucide icon
- Enhanced error section with:
  - Category header with icon
  - Retryable badge
  - Summary + suggestion
  - Collapsible technical details
  - Retry button for transient errors
