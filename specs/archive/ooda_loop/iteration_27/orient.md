# Iteration 27 – ORIENT

## Analysis

Based on observation, the codebase has good safety patterns overall but some error toasts lack retry actions.

### Error Toast Patterns Found

**Good (has retry action):**

- Document upload failure → Retry button
- Reprocess failed → Retry button
- Delete failed → (varies)

**Needs Improvement:**

- Pipeline cancel error → No retry button
- Rebuild errors → Some have retry, some don't
- Copy failures → No retry (minor)

## Focus Areas

1. **Pipeline Cancel Error**: Add retry action
2. **Rebuild Errors**: Ensure consistent retry actions
3. **Add action suggestions**: Where possible, add helpful actions

## Strategy

1. Update error toasts to include retry/action buttons
2. Add description fields for more context
3. Ensure consistency across the codebase
