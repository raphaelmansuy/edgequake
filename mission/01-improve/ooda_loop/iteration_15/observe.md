# OODA-15: Observe

## Production unwrap() Audit

Found 55+ bare `.unwrap()` in production code paths (excluding `#[cfg(test)]`).

### Critical Risk Sites

| File                          | Line                                      | Pattern                           | Risk   |
| ----------------------------- | ----------------------------------------- | --------------------------------- | ------ |
| `reranking.rs:22`             | `self.reranker.as_ref().unwrap()`         | Panics if reranker not configured | HIGH   |
| `community.rs:205`            | `node_to_community.get(node_id).unwrap()` | Panics if node missing from map   | HIGH   |
| `community.rs:360`            | `labels.get(&node.id).unwrap()`           | Panics if label missing           | HIGH   |
| `pdf_storage_impl.rs:143,200` | `status.parse().unwrap()`                 | Panics on invalid DB status       | HIGH   |
| `pdf_list_query.rs:92`        | `status_str.parse().unwrap()`             | Same DB status issue              | HIGH   |
| `entity.rs:133,207`           | `from_f64(importance).unwrap()`           | Panics on NaN/Inf                 | MEDIUM |
| `relationship.rs:164,209`     | `from_f64(weight).unwrap()`               | Panics on NaN/Inf                 | MEDIUM |

### Safe Sites (infallible by construction)

| File                              | Line                                 | Pattern                                        | Why Safe |
| --------------------------------- | ------------------------------------ | ---------------------------------------------- | -------- |
| `middleware.rs:77,84,328,346,350` | `HeaderValue::from_str(uuid/number)` | UUID and numbers are always valid ASCII        |
| `json_parser.rs:176-202`          | `Regex::new(literal).unwrap()`       | Compile-time verifiable regex                  |
| `sota.rs:362,427`                 | `last_error.unwrap()`                | Only reached in `if last_error.is_some()` path |

### RwLock Pattern (20+ sites)

`in_memory.rs` has 20+ `RwLock.read/write().unwrap()`. This service is used in production for conversation state. Same pattern fixed in OODA-06 for `keywords/cache.rs`.
