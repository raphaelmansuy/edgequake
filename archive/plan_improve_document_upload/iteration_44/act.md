# OODA Iteration 44 - Act

## Analysis

**Finding**: Queue position display would require backend API changes.

Current backend provides:
- `running_tasks` count
- `queued_tasks` count

To show individual document queue position, we would need:
- API to return queue position per document
- Or track_id to position mapping

## Result

Out of scope for UX iteration - requires backend changes. Skipping.
