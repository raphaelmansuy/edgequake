# OODA Iteration 25 – Decide

## Decision

Extend `convert_standalone_bold_to_headers()` to recognize section-numbered bold lines as headers.

## Change

Add `starts_with_section_number` check alongside `starts_upper`:

```rust
let starts_with_section_number = trimmed
    .chars()
    .next()
    .map(|c| c.is_ascii_digit())
    .unwrap_or(false);
```

Update the condition from:

```rust
if is_short && starts_upper && ...
```

to:

```rust
if is_short && (starts_upper || starts_with_section_number) && ...
```

## Risk

Very low — the combined constraints (bold + standalone + short + no trailing punctuation + digit start) form a highly selective filter.
