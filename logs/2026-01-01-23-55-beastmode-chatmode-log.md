# Task logs

- Actions: Diagnosed failing caption-after-table unit test; updated `TextTableReconstructionProcessor` to handle single-line table blocks; reran `cargo test -p edgequake-pdf` (green); appended OODA loop entries to the crate scratchpad log.
- Decisions: Treat 1 captured line as a valid table candidate when the table-like score is high enough; emit a conservative 1-column Markdown table to guarantee rendering without over-parsing.
- Next steps: If you want to re-validate outputs, re-run the batch `edgequake-pdf convert` for `test-data/real_dataset/*.pdf` and re-run the “missing table after caption” detector script.
- Lessons/insights: Real-world PDFs sometimes collapse entire tables into a single extracted text block; a safe fallback table path keeps UX stable.
