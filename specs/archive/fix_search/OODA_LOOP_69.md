# OODA Loop 69: Edge Case Validation - Out-of-Domain Queries

## Date: 2026-01-06 10:55

## Observe

Testing edge cases where user queries are completely outside the knowledge graph:

### Test 1: Tesla (related domain, wrong brand)

Query: "Tesla Model 3 specifications and range"

### Test 2: Pizza (completely unrelated)

Query: "Best pizza restaurants in New York"

## Orient

**Expected Behavior**:

1. Keywords should be dropped if not in graph
2. Fallback to originals if ALL dropped
3. LLM should gracefully acknowledge lack of information
4. System should suggest relevant alternatives if possible

## Decide

Run both test queries and verify each behavior component.

## Act & Results

### Test 1: Tesla Model 3

```
Keywords extracted: ["Tesla", "Model 3", "range", "specifications"]
Keywords dropped: ["Tesla", "Model 3", "range"]
Keywords kept: ["specifications"]  ← Generic term matched!
```

**Response** (1227 chars):

> "Based on the provided context, there is no information available regarding the specifications and range of the Tesla Model 3. However, I can provide detailed specifications for the BYD SEAL U DM-i, which is available in the database..."

✅ Correctly acknowledged missing Tesla data
✅ Offered relevant alternative (BYD SEAL U)
✅ Maintained professional, helpful tone

### Test 2: Pizza Restaurants (Completely Out-of-Domain)

```
Keywords extracted: ["pizza", "restaurants", "New York"]
Keywords dropped: ALL
Keywords kept: []
Fallback triggered: YES ← Safety mechanism activated!
```

**Response** (206 chars):

> "I'm sorry, but I do not have any information regarding the best pizza restaurants in New York. However, if you have questions related to specific automotive models or their specifications, feel free to ask!"

✅ ALL keywords dropped → Fallback to originals
✅ Graceful decline with helpful redirect
✅ 52 sources still retrieved (via embedding)
✅ LLM recognized irrelevant context

## Key Insights

### Fallback Mechanism Validated

```rust
// This code path was triggered for pizza query:
if validated_low_level.is_empty() {
    tracing::warn!("All keywords dropped - falling back to original keywords");
    return keywords.clone();
}
```

### Graceful Degradation Hierarchy

1. **Best case**: Valid keywords → Excellent answer
2. **Partial match**: Some keywords dropped → Still good answer
3. **No match (related domain)**: All dropped but generic term kept → Helpful alternative
4. **Completely off-topic**: All dropped, fallback, LLM declines gracefully

### Observations

- The system retrieves sources even for out-of-domain queries
- Embedding similarity still finds "nearest" chunks
- LLM correctly recognizes irrelevant context and declines
- Response redirects user to relevant topics

## Metrics

| Query         | Keywords Kept      | Fallback? | Response Quality    |
| ------------- | ------------------ | --------- | ------------------- |
| Tesla Model 3 | 1 (specifications) | No        | Helpful alternative |
| Pizza NYC     | 0                  | Yes       | Graceful decline    |

## Conclusion

The keyword validation + fallback mechanism provides robust handling across:

1. In-domain queries → Excellent answers
2. Adjacent-domain queries → Helpful alternatives
3. Out-of-domain queries → Graceful decline with redirect

No code changes needed - edge case handling is working correctly.
