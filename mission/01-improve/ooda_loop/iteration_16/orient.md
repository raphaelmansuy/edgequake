# OODA-16: Orient — SRP Analysis

## First Principles

1. **Single Responsibility**: Each module should have one reason to change
2. **Database row types** change when schema changes — different trigger than service logic
3. **Row ↔ Domain conversions** are a mapping concern, not a service concern

## Options

| Option                                           | Pros                                                     | Cons                                    |
| ------------------------------------------------ | -------------------------------------------------------- | --------------------------------------- |
| Extract row types → `workspace_row_types.rs`     | Clean separation, ~263 lines moved, testable conversions | Extra module to import                  |
| Extract metrics methods → `workspace_metrics.rs` | Separates stats/metrics from CRUD                        | Can't split trait impl in Rust          |
| Extract row types + normalize_entity_types       | Two clean separations                                    | normalize_entity_types is only 37 lines |

## Recommendation

Extract row types to `workspace_row_types.rs`. This is the cleanest SRP split since row types are a data mapping concern with their own change trigger (DB schema changes).
