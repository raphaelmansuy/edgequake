# OODA 67 - Orient: Model Tags Validation

## Analysis

### Why Validate Tags

1. **UI Display**: Tags shown as badges in model selection
2. **Filtering**: Could be used for model filtering
3. **Data Quality**: Ensures registry has useful metadata

### Test Strategy

- Verify tags property exists as array
- Verify at least one model has "recommended" tag
- Verify tags are strings

## Recommendation

Add lightweight test for tags structure.
