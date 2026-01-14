# OODA 67 - Decide: Tags Validation

## Decision

Add test for model tags property:

- Verify tags is an array on all models
- Verify tag values are strings
- Verify at least one model has "recommended" tag

## Why Check "recommended" Tag

The "recommended" tag is important for:

- UI default selection
- User guidance
- Feature completeness indication
