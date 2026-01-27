# Decide: Verify All Query Modes

## Decision

Test all three query modes with same query for comparison:

1. Local mode (already tested)
2. Global mode (already tested)
3. Hybrid mode (just tested)

## Results Summary

| Mode   | Sources | Embed Time | Total Time | Answer Quality |
| ------ | ------- | ---------- | ---------- | -------------- |
| Local  | 29      | ~1200ms    | ~2000ms    | Entity-focused |
| Global | 4       | ~1200ms    | ~2000ms    | High-level     |
| Hybrid | 29      | ~2650ms    | ~4963ms    | Comprehensive  |

## Conclusion

All query modes working correctly with Ollama provider.
