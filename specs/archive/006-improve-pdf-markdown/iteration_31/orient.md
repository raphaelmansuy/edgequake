# OODA-31 Orient: Reading Order Testing Opportunities

## Module Analysis

### Untested Functions

| Function                                  | Line | Purpose               | Testability |
| ----------------------------------------- | ---- | --------------------- | ----------- |
| `ReadingOrder::iter()`                    | 32   | Iterate reading order | High        |
| `ReadingOrderDetector::with_tolerances()` | 71   | Custom tolerances     | High        |
| `ReadingOrderDetector::Default`           | 615  | Default trait         | High        |
| `from_xy_cut_order()`                     | 609  | Convert XY-cut order  | Medium      |

### Current Test Coverage

| Test                         | Function Tested                     |
| ---------------------------- | ----------------------------------- |
| test_single_column_order     | single_column_order                 |
| test_same_line_left_to_right | single_column_order                 |
| test_multi_column_order      | determine_order, multi_column_order |
| test_spanning_element        | determine_order                     |
| test_empty_blocks            | determine_order                     |
| test_reading_order_position  | ReadingOrder::position_of           |

### Gap Analysis

1. **ReadingOrder methods** - `iter()` not tested
2. **Constructor variants** - `with_tolerances()` not tested
3. **Default trait** - Default impl not tested
4. **from_xy_cut_order** - Not tested

## Recommendation

Add 4 tests:

1. `test_reading_order_iter` - Test iterator method
2. `test_detector_default` - Test Default trait
3. `test_detector_with_tolerances` - Test custom tolerance constructor
4. `test_from_xy_cut_order` - Test XY-cut conversion
