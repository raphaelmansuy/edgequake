# Iteration 06 - ORIENT Phase

## Gap Analysis

### Current State

- Basic E2E tests exist for reprocessing
- No tests for error popover
- No tests for copy-to-clipboard

### Test Strategy

#### Unit-like E2E Tests (UI only)

1. Test popover renders correctly
2. Test copy button UI feedback
3. Test retry button loading state

#### Integration E2E Tests (with backend)

1. Upload document
2. Wait for processing
3. Verify status updates
4. Test error scenarios

## Technical Considerations

### Clipboard Testing

Playwright supports `navigator.clipboard` testing:

```ts
await page.evaluate(() => navigator.clipboard.writeText("test"));
const text = await page.evaluate(() => navigator.clipboard.readText());
```

However, clipboard access requires permissions. Alternative:

- Test copy button click
- Verify toast notification
- Check button state change (icon becomes checkmark)

### Error State Testing

Need to either:

1. Use mock backend that returns errors
2. Upload invalid file that causes error
3. Configure backend to simulate failure

## Recommended Approach

Focus on UI tests that don't require real failures:

1. Test popover open/close
2. Test copy button click and feedback
3. Test button states (loading, disabled)
4. Verify data-testid attributes work
