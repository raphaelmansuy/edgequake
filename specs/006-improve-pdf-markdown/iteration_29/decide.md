# OODA-29 Decide: Add LLM Enhance Documentation

## Decision

Add WHY documentation to LlmEnhanceProcessor and text_needs_improvement, plus additional tests.

## Implementation Plan

### 1. Add WHY to LlmEnhanceProcessor

Explain the enhancement strategy and when to use each feature.

### 2. Add WHY to text_needs_improvement()

Document the heuristics:
- 0.3 word character threshold
- OCR error patterns (nurnber, 0O, etc.)

### 3. Add Test for with_image_ocr Builder

```rust
#[test]
fn test_processor_with_image_ocr() {
    let provider = Arc::new(MockProvider::new());
    let processor = LlmEnhanceProcessor::with_defaults(provider)
        .with_image_ocr_enabled();
    assert!(processor.image_ocr_config.is_some());
}
```

## Expected Outcome

- WHY comments: 0 → 2
- Tests: 484 → 486 (+2)
