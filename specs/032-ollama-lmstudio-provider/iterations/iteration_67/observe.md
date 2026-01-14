# OODA 67 - Observe: Model Tags for UI Display

## Current State

### Model Tags API Response

Each model has a `tags` array:

```json
{
  "tags": ["recommended", "cost-effective", "fast"]
}
```

### Common Tags

- `recommended`: Suggested model for use
- `cost-effective`: Budget-friendly option
- `fast`: Quick response times
- `multimodal`: Supports images
- `vision`: Supports image input
- `legacy`: Older model, may be deprecated
- `deprecated`: Should not be used for new projects

### UI Usage

- Model selection UI shows tags as badges
- Filters could use tags for categorization

### Test Gap

No tests validate tags structure or content.
