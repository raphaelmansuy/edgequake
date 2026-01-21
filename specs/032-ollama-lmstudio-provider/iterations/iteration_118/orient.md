# OODA 118: Orient

## Analysis
Query lineage display is critical for:
- Transparency: Users need to know which model answered
- Debugging: Identify which provider was used
- Cost tracking: Associate responses with provider costs

## Technical Understanding
- Query API: POST /api/v1/query
- Response structure: { answer, mode, sources, stats }
- Stats contain timing metrics but not provider info

## Strategy
Add tests that verify:
1. Query endpoint returns structured response
2. Response has expected fields
3. Handle both success and error cases gracefully
