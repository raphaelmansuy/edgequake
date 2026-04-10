# OODA-27 Orient

## Analysis

| File                   | Missing WHY                                       | Impact                                    |
| ---------------------- | ------------------------------------------------- | ----------------------------------------- |
| storage/error.rs       | Error variant hierarchy design                    | HIGH — all storage callers depend on this |
| core/error.rs          | Two-level error hierarchy (Error + QueryError)    | HIGH — top-level error type               |
| pipeline/cache.rs      | Cache key = content_hash + model + prompt_version | MED — prevents stale cache hits           |
| pipeline/validation.rs | Blocked extensions security rationale             | MED — security boundary                   |
| tasks/queue.rs         | Arc<Mutex<Receiver>> pattern for multi-consumer   | MED — concurrency correctness             |

## Plan
Add WHY comments + edge case tests where coverage is thin.
Storage error.rs and core error.rs already have good test coverage.
Focus WHY comments on design rationale.
