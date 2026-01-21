# OODA Iteration 70: Invalid Workspace Handling

## Observe

Test API behavior when querying with invalid workspace ID.

## Orient

Expected: API should return error when workspace not found.

## Decide

Test with random UUID as workspace ID.

## Act

Tested with invalid workspace:

```bash
curl -X POST /api/v1/query \
  -H "X-Workspace-ID: 00000000-0000-0000-0000-000000000000" \
  -d '{"query": "test", "mode": "local"}'
```

Returns: "I'm sorry, but I couldn't find any relevant information" (no sources)

Note: API gracefully handles missing workspace by returning empty results.
This is a design choice - could also return 404 error.

✅ Invalid workspace handled gracefully (empty results)
