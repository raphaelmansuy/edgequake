# Task logs

- Actions: Verified table reconstruction single-line heuristic is present; located key line references for handoff; recorded this turn’s status.
- Decisions: Keep table reconstruction conservative; prefer 1-column fallback for single-line collapsed tables to guarantee Markdown pipe-table rendering.
- Next steps: (Optional) Re-run real_dataset PDF→MDF batch + caption/table detector to validate artifacts; iterate column inference only if needed.
- Lessons/insights: Real PDFs sometimes collapse entire tables into one extracted text block, so table reconstruction must handle single-line candidates.
